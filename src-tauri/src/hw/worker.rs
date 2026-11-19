//! Threads de fundo:
//! - leitura: descobre o volante, lê relatórios de entrada e reconecta sozinho;
//! - emissão: envia telemetria (no máx. ~60 Hz) e status para a interface.
//! A interface nunca espera por E/S de hardware.

use super::device::{Device, ReadOutcome};
use super::discovery::{find_v99, Found};
use super::ffi::{CloseHandle, OVERLAPPED};
use crate::core::Core;
use crate::model::{Mode, Telemetry};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tauri::Emitter;

const DISCOVERY_INTERVAL: Duration = Duration::from_millis(1000);
const READ_WAIT_MS: u32 = 250;
const TELEMETRY_INTERVAL: Duration = Duration::from_millis(16);
const STATUS_INTERVAL: Duration = Duration::from_millis(500);

pub fn spawn(core: Arc<Core>) {
    let c = core.clone();
    thread::Builder::new().name("pxn-reader".into()).spawn(move || reader_loop(c)).expect("thread de leitura");
    thread::Builder::new().name("pxn-emitter".into()).spawn(move || emitter_loop(core)).expect("thread de emissão");
}

#[derive(PartialEq)]
enum Seen {
    Nothing,
    XInput,
    OpenFailed,
}

fn reader_loop(core: Arc<Core>) {
    let mut seen = Seen::Nothing;
    let mut sessions = 0u32;
    core.log("info", "app", "Núcleo iniciado; procurando PXN V99");
    loop {
        match find_v99() {
            Found::DirectInput(path) => match Device::open(&path) {
                Ok(dev) => {
                    seen = Seen::Nothing;
                    if sessions > 0 {
                        if let Ok(mut s) = core.status.lock() {
                            s.reconnects += 1;
                        }
                    }
                    sessions += 1;
                    session(&core, Arc::new(dev));
                }
                Err(e) => {
                    if seen != Seen::OpenFailed {
                        seen = Seen::OpenFailed;
                        core.log("error", "hid", format!("Volante encontrado, mas não foi possível abrir: {e}"));
                    }
                    if let Ok(mut s) = core.status.lock() {
                        s.connected = false;
                        s.error_count += 1;
                        s.last_error = Some(e);
                    }
                    core.emit_status();
                }
            },
            Found::XInput => {
                if seen != Seen::XInput {
                    seen = Seen::XInput;
                    core.log("warn", "usb", "PXN V99 detectado em modo XInput; telemetria HID e Force Feedback exigem DirectInput");
                    if let Ok(mut s) = core.status.lock() {
                        s.connected = true;
                        s.mode = Some(Mode::XInput);
                        s.info = None;
                    }
                    core.emit_status();
                }
            }
            Found::None => {
                if seen != Seen::Nothing {
                    seen = Seen::Nothing;
                    core.log("info", "usb", "PXN V99 não está mais presente");
                }
                let changed = core
                    .status
                    .lock()
                    .map(|mut s| {
                        let was = s.connected || s.mode.is_some();
                        s.connected = false;
                        s.mode = None;
                        s.info = None;
                        was
                    })
                    .unwrap_or(false);
                if changed {
                    core.emit_status();
                }
            }
        }
        thread::sleep(DISCOVERY_INTERVAL);
    }
}

/// Uma sessão dura do momento em que o volante é aberto até ele desconectar.
fn session(core: &Arc<Core>, dev: Arc<Device>) {
    let info = dev.info.clone();
    core.log(
        "info",
        "usb",
        format!(
            "Conectado: {} (VID {:04X} PID {:04X}, rev {:04X}) em DirectInput: {} eixos, {} botões",
            info.product,
            info.vid,
            info.pid,
            info.revision,
            info.axes.len(),
            info.button_count
        ),
    );
    match dev.pid_dry_run() {
        Ok(hex) => core.log("info", "ffb", format!("Relatórios PID montados (não enviados): {hex}")),
        Err(e) => core.log("warn", "ffb", format!("Efeitos PID do app indisponíveis neste descritor: {e}")),
    }
    if let Ok(mut s) = core.status.lock() {
        s.connected = true;
        s.mode = Some(Mode::DirectInput);
        s.info = Some(info.clone());
        s.last_error = None;
        s.applied_gain_percent = None;
    }
    if let Ok(mut d) = core.device.lock() {
        *d = Some(dev.clone());
    }
    core.emit_status();

    let mut buf = vec![0u8; info.input_len.max(1) as usize];
    let event = Device::create_event();
    let mut ov = OVERLAPPED::with_event(event);
    let reason: String;
    let mut last_pid: Option<(bool, bool, bool, u32)> = None;

    'read: loop {
        if let Err(e) = dev.start_read(&mut buf, &mut ov) {
            reason = format!("leitura falhou (erro Win32 {e})");
            break;
        }
        loop {
            match dev.wait_read(&mut ov, READ_WAIT_MS) {
                ReadOutcome::Report(n) => {
                    let mut pid_change: Option<String> = None;
                    if let Ok(mut t) = core.telemetry.lock() {
                        if dev.decode(&buf[..n], &mut t) {
                            t.t_ms = core.start.elapsed().as_secs_f64() * 1000.0;
                            t.seq = core.seq.fetch_add(1, Ordering::Relaxed) + 1;
                        }
                        let now = t.pid.as_ref().map(|p| (p.effect_playing, p.actuators_enabled, p.paused, p.effect_block_index));
                        if now != last_pid {
                            last_pid = now;
                            if let Some((playing, actuators, paused, block)) = now {
                                pid_change = Some(format!(
                                    "Estado PID do volante: efeito tocando={playing}, atuadores={actuators}, pausado={paused}, bloco={block}"
                                ));
                            }
                        }
                    }
                    if let Some(msg) = pid_change {
                        core.log("info", "ffb", msg);
                    }
                    break;
                }
                // Volante parado não envia relatórios; continuar aguardando a mesma leitura.
                ReadOutcome::Timeout => continue,
                ReadOutcome::Failed(e) => {
                    reason = format!("leitura interrompida (erro Win32 {e})");
                    break 'read;
                }
            }
        }
    }

    dev.cancel_read(&mut ov);
    unsafe {
        CloseHandle(event);
    }
    if let Ok(mut d) = core.device.lock() {
        *d = None;
    }
    if let Ok(mut t) = core.telemetry.lock() {
        *t = Telemetry::default();
    }
    core.seq.fetch_add(1, Ordering::Relaxed);
    if let Ok(mut s) = core.status.lock() {
        s.connected = false;
        s.mode = None;
        s.info = None;
        s.report_hz = 0.0;
        s.applied_gain_percent = None;
        s.centering_active = false;
        s.last_error = Some(reason.clone());
    }
    core.log("warn", "usb", format!("Volante desconectado: {reason}"));
    core.emit_status();
}

fn emitter_loop(core: Arc<Core>) {
    let mut last_seq = 0u64;
    let mut last_status = Instant::now();
    let mut last_count = 0u64;
    let mut last_telemetry_emit = Instant::now();
    loop {
        thread::sleep(TELEMETRY_INTERVAL);
        let seq = core.seq.load(Ordering::Relaxed);
        if seq != last_seq && last_telemetry_emit.elapsed() >= core.ui_interval(TELEMETRY_INTERVAL) {
            last_seq = seq;
            last_telemetry_emit = Instant::now();
            let snapshot = core.telemetry.lock().map(|t| t.clone()).ok();
            if let Some(t) = snapshot {
                let _ = core.app.emit("telemetry", t);
            }
        }
        let elapsed = last_status.elapsed();
        if elapsed >= STATUS_INTERVAL {
            let total = core.telemetry.lock().map(|t| t.seq).unwrap_or(0);
            let hz = total.saturating_sub(last_count) as f32 / elapsed.as_secs_f32();
            last_count = total;
            last_status = Instant::now();
            if let Ok(mut s) = core.status.lock() {
                s.report_hz = if s.connected { hz } else { 0.0 };
                s.total_reports = total;
            }
            core.emit_status();
        }
    }
}
