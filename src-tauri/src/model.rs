//! Tipos trocados entre o núcleo e a interface (serializados em JSON).

use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum Mode {
    DirectInput,
    XInput,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct AxisInfo {
    /// Usage HID (Generic Desktop): 0x30 = X, 0x31 = Y, ... 0x36 = Slider.
    pub usage: u16,
    pub name: String,
    pub bits: u16,
    pub min: i32,
    pub max: i32,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct PidPool {
    pub ram_pool_size: u32,
    pub simultaneous_effects_max: u32,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct DeviceInfo {
    pub product: String,
    pub manufacturer: String,
    pub serial: String,
    pub vid: u16,
    pub pid: u16,
    /// bcdDevice do descritor USB (revisão do dispositivo).
    pub revision: u16,
    pub path: String,
    pub input_len: u16,
    pub output_len: u16,
    pub feature_len: u16,
    pub axes: Vec<AxisInfo>,
    pub button_count: u16,
    pub has_hat: bool,
    pub has_pid_state: bool,
    pub has_device_gain: bool,
    pub pid_pool: Option<PidPool>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct DeviceStatus {
    pub connected: bool,
    pub mode: Option<Mode>,
    pub info: Option<DeviceInfo>,
    /// Relatórios de entrada recebidos por segundo (medido).
    pub report_hz: f32,
    pub total_reports: u64,
    pub error_count: u32,
    pub reconnects: u32,
    pub last_error: Option<String>,
    /// Último Device Gain enviado por este app (0..100), se algum.
    pub applied_gain_percent: Option<u8>,
    /// Malha de centralização do app em execução.
    pub centering_active: bool,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct PidState {
    pub effect_playing: bool,
    pub actuators_enabled: bool,
    pub paused: bool,
    pub safety_switch: bool,
    pub actuator_power: bool,
    pub effect_block_index: u32,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Telemetry {
    pub seq: u64,
    /// Milissegundos desde o início do app, no instante do último relatório.
    pub t_ms: f64,
    /// Valores brutos na mesma ordem de `DeviceInfo::axes`.
    pub axes: Vec<i32>,
    /// Hat: 0..7 (N, NE, E, ...) ou None quando centralizado.
    pub hat: Option<u8>,
    /// Números (1-based) dos botões pressionados.
    pub buttons: Vec<u16>,
    pub pid: Option<PidState>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub id: u64,
    /// Milissegundos desde a época Unix.
    pub ts: u64,
    pub level: &'static str,
    pub category: &'static str,
    pub message: String,
}
