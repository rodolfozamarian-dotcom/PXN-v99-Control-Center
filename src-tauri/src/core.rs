//! Estado compartilhado entre a thread de leitura, a thread de emissão e os comandos.

use crate::game::GameStatus;
use crate::hw::centering;
use crate::hw::effects::{EffectLevels, EffectsParams};
use crate::hw::device::Device;
use crate::model::{DeviceStatus, LogEntry, Telemetry};
use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};

const LOG_CAPACITY: usize = 500;
/// Com a janela fora de foco (jogo em primeiro plano), a WebView quase não
/// processa eventos; mandar em alta frequência só enche uma fila que trava a
/// tela quando o usuário volta. Fora de foco, no máximo 4 envios por segundo.
const BACKGROUND_UI_INTERVAL: Duration = Duration::from_millis(250);
/// O arquivo de log é recomeçado quando passa deste tamanho.
const LOG_FILE_MAX_BYTES: u64 = 2 * 1024 * 1024;

/// Gravação da telemetria crua do jogo (uma linha JSON por quadro).
pub struct Recording {
    pub until: Instant,
    pub path: PathBuf,
    pub file: File,
    pub frames: u64,
}

pub struct Core {
    pub app: AppHandle,
    pub start: Instant,
    pub status: Mutex<DeviceStatus>,
    pub telemetry: Mutex<Telemetry>,
    pub device: Mutex<Option<Arc<Device>>>,
    /// Malha de centralização em execução, se houver.
    pub centering: Mutex<Option<centering::Handle>>,
    /// Telemetria recebida do Farming Simulator.
    pub game: Mutex<GameStatus>,
    /// Configuração dos efeitos de jogo (vibração).
    pub effects: Mutex<EffectsParams>,
    /// Intensidade atual de cada efeito, para a interface.
    pub effect_levels: Mutex<EffectLevels>,
    /// Incrementado a cada relatório decodificado.
    pub seq: AtomicU64,
    /// A janela do app está em foco.
    pub ui_focused: AtomicBool,
    /// Vigia da tela (sinal de vida da interface).
    pub ui_watch: crate::shell::UiWatch,
    /// Gravação de diagnóstico em andamento, se houver.
    pub recording: Mutex<Option<Recording>>,
    /// Pasta de logs do app (%LOCALAPPDATA%\com.pxn.v99controlcenter\logs).
    pub log_dir: Option<PathBuf>,
    log: Mutex<VecDeque<LogEntry>>,
    log_file: Mutex<Option<File>>,
    log_id: AtomicU64,
}

impl Core {
    pub fn new(app: AppHandle) -> Self {
        let log_dir = app.path().app_log_dir().ok().filter(|d| std::fs::create_dir_all(d).is_ok());
        let log_file = log_dir.as_ref().and_then(|d| {
            let path = d.join("pxn-control-center.log");
            if std::fs::metadata(&path).map(|m| m.len() > LOG_FILE_MAX_BYTES).unwrap_or(false) {
                let _ = std::fs::rename(&path, d.join("pxn-control-center.old.log"));
            }
            OpenOptions::new().create(true).append(true).open(path).ok()
        });
        Self {
            app,
            start: Instant::now(),
            status: Mutex::new(DeviceStatus::default()),
            telemetry: Mutex::new(Telemetry::default()),
            device: Mutex::new(None),
            centering: Mutex::new(None),
            game: Mutex::new(GameStatus::default()),
            effects: Mutex::new(EffectsParams::default()),
            effect_levels: Mutex::new(EffectLevels::default()),
            seq: AtomicU64::new(0),
            ui_focused: AtomicBool::new(true),
            ui_watch: crate::shell::UiWatch::new(),
            recording: Mutex::new(None),
            log_dir,
            log: Mutex::new(VecDeque::with_capacity(LOG_CAPACITY)),
            log_file: Mutex::new(log_file),
            log_id: AtomicU64::new(1),
        }
    }

    pub fn log(&self, level: &'static str, category: &'static str, message: impl Into<String>) {
        let entry = LogEntry {
            id: self.log_id.fetch_add(1, Ordering::Relaxed),
            ts: SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0),
            level,
            category,
            message: message.into(),
        };
        #[cfg(debug_assertions)]
        eprintln!("[{}] {}: {}", entry.level, entry.category, entry.message);
        if let Ok(mut f) = self.log_file.lock() {
            if let Some(f) = f.as_mut() {
                let _ = writeln!(f, "{} [{}] {}: {}", entry.ts, entry.level, entry.category, entry.message);
            }
        }
        if let Ok(mut log) = self.log.lock() {
            if log.len() == LOG_CAPACITY {
                log.pop_front();
            }
            log.push_back(entry.clone());
        }
        let _ = self.app.emit("log", entry);
    }

    /// Intervalo mínimo entre envios de um evento à interface.
    pub fn ui_interval(&self, normal: Duration) -> Duration {
        if self.ui_focused.load(Ordering::Relaxed) {
            normal
        } else {
            normal.max(BACKGROUND_UI_INTERVAL)
        }
    }

    pub fn log_snapshot(&self) -> Vec<LogEntry> {
        self.log.lock().map(|l| l.iter().cloned().collect()).unwrap_or_default()
    }

    pub fn status_snapshot(&self) -> DeviceStatus {
        self.status.lock().map(|s| s.clone()).unwrap_or_default()
    }

    pub fn emit_status(&self) {
        let _ = self.app.emit("status", self.status_snapshot());
    }

    pub fn current_device(&self) -> Option<Arc<Device>> {
        self.device.lock().ok().and_then(|d| d.clone())
    }
}
