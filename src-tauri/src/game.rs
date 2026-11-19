//! Telemetria do Farming Simulator 25.
//!
//! O mod `FS25_PXNV99Telemetry` (pasta `fs25-mod/`) escreve uma linha JSON por
//! quadro no named pipe `\\.\pipe\PXNV99Telemetry`. Este módulo é o servidor do
//! pipe: aceita a conexão do jogo, decodifica as linhas e publica o estado para
//! a interface (evento `game`) e, nas próximas fases, para o motor de vibração.
//!
//! O pipe só aceita clientes locais. Todos os valores recebidos são tratados
//! como dados não confiáveis e validados antes de qualquer uso.

use crate::core::Core;
use crate::hw::ffi::*;
use serde::{Deserialize, Serialize};
use std::ptr::null_mut;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tauri::Emitter;

pub const PIPE_NAME: &str = r"\\.\pipe\PXNV99Telemetry";
const READ_BUFFER: usize = 16 * 1024;
const MAX_LINE: usize = 8 * 1024;
const EMIT_INTERVAL: Duration = Duration::from_millis(33);
/// O mod reabre o pipe a cada 10 s. Uma nova conexão até este tempo depois da
/// anterior é a mesma sessão de jogo (sem log nem zerar contadores).
const RECONNECT_GRACE: Duration = Duration::from_millis(1500);

/// Conexões do jogo abertas agora.
static ACTIVE: AtomicUsize = AtomicUsize::new(0);
/// Fim da última conexão (para distinguir reabertura de reconexão).
static LAST_END: Mutex<Option<Instant>> = Mutex::new(None);

/// Um quadro enviado pelo mod. Os nomes curtos são os do protocolo v1.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GameFrame {
    pub v: Option<u32>,
    pub t: Option<f64>,
    /// Jogo pausado ou com menu aberto.
    pub pz: Option<bool>,
    /// Jogador dentro de um veículo.
    #[serde(rename = "in")]
    pub in_vehicle: Option<bool>,
    /// Motor ligado.
    pub on: Option<bool>,
    pub rpm: Option<f32>,
    pub rmin: Option<f32>,
    pub rmax: Option<f32>,
    /// Carga do motor 0..1.
    pub ld: Option<f32>,
    /// Acelerador -1..1.
    pub thr: Option<f32>,
    /// Pedal de embreagem 0..1.
    pub clt: Option<f32>,
    pub gr: Option<i32>,
    /// Velocidade em km/h (negativa em ré).
    pub spd: Option<f32>,
    pub wn: Option<u32>,
    pub wc: Option<u32>,
    #[serde(rename = "gRd")]
    pub g_road: Option<u32>,
    #[serde(rename = "gHd")]
    pub g_hard: Option<u32>,
    #[serde(rename = "gFd")]
    pub g_field: Option<u32>,
    #[serde(rename = "gSf")]
    pub g_soft: Option<u32>,
    pub dep: Option<f32>,
    pub slp: Option<f32>,
    /// Embreagem patinando no engate (0..1), quando o jogo controla a embreagem.
    pub csl: Option<f32>,
    /// Motor afogando.
    pub stl: Option<bool>,
    /// Torque aplicado / torque máximo (0..1).
    pub trq: Option<f32>,
    /// Velocidade média da suspensão das rodas (m/s): trepidação real do terreno.
    pub sus: Option<f32>,
    /// Rodas na água.
    pub wat: Option<u32>,
    /// Velocidade da suspensão dianteira esquerda / direita (m/s, com sinal).
    pub sfl: Option<f32>,
    pub sfr: Option<f32>,
    pub msg: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct GameStatus {
    pub connected: bool,
    pub hz: f32,
    pub frames: u64,
    pub bad_lines: u64,
    pub last: Option<GameFrame>,
    pub last_message: Option<String>,
    /// Instante de chegada do último quadro (para descartar dados antigos).
    #[serde(skip)]
    pub last_rx: Option<Instant>,
}

pub fn spawn(core: Arc<Core>) {
    thread::Builder::new()
        .name("pxn-game-pipe".into())
        .spawn(move || server_loop(core))
        .expect("thread do pipe de telemetria");
}

fn server_loop(core: Arc<Core>) {
    let name = wide(PIPE_NAME);
    let mut create_error_logged = false;
    loop {
        let h = unsafe {
            CreateNamedPipeW(
                name.as_ptr(),
                PIPE_ACCESS_INBOUND,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
                PIPE_UNLIMITED_INSTANCES,
                0,
                READ_BUFFER as u32,
                0,
                null_mut(),
            )
        };
        if h == INVALID_HANDLE_VALUE {
            if !create_error_logged {
                create_error_logged = true;
                let e = unsafe { GetLastError() };
                core.log("error", "game", format!("Não foi possível criar o pipe de telemetria (erro Win32 {e})"));
            }
            thread::sleep(Duration::from_secs(5));
            continue;
        }
        create_error_logged = false;

        let connected = unsafe { ConnectNamedPipe(h, null_mut()) != 0 || GetLastError() == ERROR_PIPE_CONNECTED };
        if !connected {
            unsafe {
                CloseHandle(h);
            }
            thread::sleep(Duration::from_millis(500));
            continue;
        }
        // Cada conexão é lida na sua própria thread e o laço volta na hora a
        // criar outra instância: quando o mod reabre o pipe, já há uma à espera.
        let c = core.clone();
        let spawned = thread::Builder::new().name("pxn-game-session".into()).spawn(move || {
            session(&c, h);
            unsafe {
                DisconnectNamedPipe(h);
                CloseHandle(h);
            }
        });
        if spawned.is_err() {
            unsafe {
                DisconnectNamedPipe(h);
                CloseHandle(h);
            }
            thread::sleep(Duration::from_millis(500));
        }
    }
}

/// Inicia a gravação dos próximos `seconds` segundos de telemetria crua.
pub fn start_recording(core: &Core, seconds: u64) -> Result<String, String> {
    let dir = core.log_dir.clone().ok_or("Pasta de logs indisponível")?;
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let path = dir.join(format!("telemetria-{stamp}.jsonl"));
    let file = std::fs::File::create(&path).map_err(|e| format!("Não foi possível criar {}: {e}", path.display()))?;
    let shown = path.display().to_string();
    let mut rec = core.recording.lock().map_err(|_| "estado da gravação corrompido")?;
    *rec = Some(crate::core::Recording { until: Instant::now() + Duration::from_secs(seconds), path, file, frames: 0 });
    drop(rec);
    core.log("info", "game", format!("Gravando {seconds} s de telemetria em {shown}"));
    Ok(shown)
}

fn record_line(core: &Core, line: &str) {
    use std::io::Write;
    let Ok(mut rec) = core.recording.lock() else { return };
    let finished = match rec.as_mut() {
        None => return,
        Some(r) if Instant::now() >= r.until => true,
        Some(r) => {
            let _ = writeln!(r.file, "{line}");
            r.frames += 1;
            false
        }
    };
    if finished {
        if let Some(r) = rec.take() {
            drop(rec);
            core.log("info", "game", format!("Gravação concluída: {} quadros em {}", r.frames, r.path.display()));
        }
    }
}

fn set_status(core: &Core, f: impl FnOnce(&mut GameStatus)) {
    if let Ok(mut g) = core.game.lock() {
        f(&mut g);
    }
}

fn emit(core: &Core) {
    if let Ok(g) = core.game.lock() {
        let _ = core.app.emit("game", g.clone());
    }
}

fn session(core: &Arc<Core>, h: HANDLE) {
    let reopened = ACTIVE.fetch_add(1, Ordering::SeqCst) > 0
        || LAST_END.lock().ok().and_then(|t| *t).map(|t| t.elapsed() < RECONNECT_GRACE).unwrap_or(false);
    if reopened {
        set_status(core, |g| g.connected = true);
    } else {
        core.log("info", "game", "Farming Simulator conectado (mod PXN V99 Telemetry)");
        set_status(core, |g| {
            *g = GameStatus { connected: true, ..Default::default() };
        });
        emit(core);
    }

    let mut buf = vec![0u8; READ_BUFFER];
    let mut pending: Vec<u8> = Vec::with_capacity(MAX_LINE);
    let mut window_start = Instant::now();
    let mut window_frames = 0u32;
    let mut last_emit = Instant::now() - EMIT_INTERVAL;

    loop {
        let mut n = 0u32;
        let ok = unsafe { ReadFile(h, buf.as_mut_ptr(), buf.len() as u32, &mut n, null_mut()) };
        if ok == 0 || n == 0 {
            break;
        }
        pending.extend_from_slice(&buf[..n as usize]);

        while let Some(pos) = pending.iter().position(|&b| b == b'\n') {
            let line: Vec<u8> = pending.drain(..=pos).collect();
            let text = String::from_utf8_lossy(&line);
            let text = text.trim();
            if text.is_empty() {
                continue;
            }
            match serde_json::from_str::<GameFrame>(text) {
                Ok(frame) => {
                    window_frames += 1;
                    record_line(core, text);
                    let message = frame.msg.clone();
                    set_status(core, |g| {
                        g.frames += 1;
                        if frame.msg.is_none() {
                            g.last = Some(frame);
                            g.last_rx = Some(Instant::now());
                        }
                    });
                    if let Some(m) = message {
                        let m: String = m.chars().take(300).collect();
                        core.log("warn", "game", format!("Mod de telemetria: {m}"));
                        set_status(core, |g| g.last_message = Some(m));
                    }
                }
                Err(_) => set_status(core, |g| g.bad_lines += 1),
            }
        }
        if pending.len() > MAX_LINE {
            // Linha sem fim: descartar para não crescer sem limite.
            pending.clear();
            set_status(core, |g| g.bad_lines += 1);
        }

        let elapsed = window_start.elapsed();
        if elapsed >= Duration::from_secs(1) {
            let hz = window_frames as f32 / elapsed.as_secs_f32();
            set_status(core, |g| g.hz = hz);
            window_frames = 0;
            window_start = Instant::now();
        }
        if last_emit.elapsed() >= core.ui_interval(EMIT_INTERVAL) {
            last_emit = Instant::now();
            emit(core);
        }
    }

    if let Ok(mut t) = LAST_END.lock() {
        *t = Some(Instant::now());
    }
    if ACTIVE.fetch_sub(1, Ordering::SeqCst) == 1 {
        // Pode ser só a reabertura periódica do mod: esperar antes de anunciar.
        let c = core.clone();
        let _ = thread::Builder::new().name("pxn-game-grace".into()).spawn(move || {
            thread::sleep(RECONNECT_GRACE);
            if ACTIVE.load(Ordering::SeqCst) > 0 {
                return;
            }
            let (frames, bad) = c.game.lock().map(|g| (g.frames, g.bad_lines)).unwrap_or((0, 0));
            set_status(&c, |g| {
                g.connected = false;
                g.hz = 0.0;
            });
            emit(&c);
            c.log("info", "game", format!("Farming Simulator desconectado ({frames} quadros recebidos, {bad} linhas inválidas)"));
        });
    }
}
