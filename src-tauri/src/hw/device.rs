//! Handle compartilhado do V99: capacidades HID, decodificação de relatórios de
//! entrada e escrita validada de relatórios PID.
//!
//! O dispositivo é aberto com FILE_SHARE_READ | FILE_SHARE_WRITE e nunca é
//! adquirido em modo exclusivo, de modo que um jogo usando DirectInput continua
//! com acesso total ao volante.

use super::ffi::*;
use crate::model::{AxisInfo, DeviceInfo, PidPool, PidState, Telemetry};
use std::ffi::c_void;
use std::ptr::null_mut;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const PAGE_GENERIC: u16 = 0x01;
const PAGE_BUTTON: u16 = 0x09;
const PAGE_PID: u16 = 0x0F;

const USAGE_HAT: u16 = 0x39;
const PID_EFFECT_BLOCK_INDEX: u16 = 0x22;
const PID_DEVICE_GAIN: u16 = 0x7E;
const PID_RAM_POOL_SIZE: u16 = 0x80;
const PID_SIMULTANEOUS_MAX: u16 = 0x83;
const PID_EFFECT_PLAYING: u16 = 0x94;
const PID_DEVICE_PAUSED: u16 = 0x9F;
const PID_ACTUATORS_ENABLED: u16 = 0xA0;
const PID_SAFETY_SWITCH: u16 = 0xA4;
const PID_ACTUATOR_POWER: u16 = 0xA6;

// Coleções (relatórios) e campos PID usados para criar e controlar efeitos.
const COL_SET_EFFECT: u16 = 0x21;
const COL_EFFECT_TYPE: u16 = 0x25;
const COL_AXES_ENABLE: u16 = 0x55;
const COL_DIRECTION: u16 = 0x57;
const COL_SET_PERIODIC: u16 = 0x6E;
const COL_SET_CONSTANT: u16 = 0x73;
const PID_ET_CONSTANT: u16 = 0x26;
const PID_ET_SINE: u16 = 0x31;
const PID_TRIGGER_BUTTON: u16 = 0x53;
const PID_MAGNITUDE: u16 = 0x70;
const PID_PERIOD: u16 = 0x72;

/// Limites de segurança do teste de vibração.
pub const VIBRATION_MAX_MAGNITUDE: i32 = 40;
pub const VIBRATION_MAX_MS: i32 = 2000;

const PID_DIRECTION_ENABLE: u16 = 0x56;
const PAGE_ORDINAL: u16 = 0x0A;
const COL_EFFECT_OPERATION_REPORT: u16 = 0x77;
const COL_EFFECT_OPERATION: u16 = 0x78;
const COL_BLOCK_FREE: u16 = 0x90;
const COL_DEVICE_CONTROL: u16 = 0x96;
const PID_DURATION: u16 = 0x50;
const PID_GAIN: u16 = 0x52;
const PID_OP_START: u16 = 0x79;
const PID_OP_STOP: u16 = 0x7B;
const PID_LOOP_COUNT: u16 = 0x7C;
const PID_BLOCK_LOAD_SUCCESS: u16 = 0x8C;
const PID_BLOCK_LOAD_FULL: u16 = 0x8D;
const PID_BLOCK_LOAD_ERROR: u16 = 0x8E;
const USAGE_X: u16 = 0x30;


/// Comandos do relatório PID "Device Control".
#[derive(Debug, Clone, Copy)]
pub enum DeviceControl {
    EnableActuators,
    DisableActuators,
    StopAllEffects,
    Reset,
}

impl DeviceControl {
    fn usage(self) -> u16 {
        match self {
            DeviceControl::EnableActuators => 0x97,
            DeviceControl::DisableActuators => 0x98,
            DeviceControl::StopAllEffects => 0x99,
            DeviceControl::Reset => 0x9A,
        }
    }
}

/// Intervalo mínimo entre escritas no dispositivo (proteção contra flood).
const MIN_WRITE_INTERVAL: Duration = Duration::from_millis(4);
const WRITE_TIMEOUT_MS: u32 = 500;

#[derive(Debug, Clone)]
struct ValCap {
    page: u16,
    report_id: u8,
    link: u16,
    /// Usage da coleção que contém o campo (identifica o relatório PID).
    link_usage: u16,
    usage_min: u16,
    bits: u16,
    min: i32,
    max: i32,
}

#[derive(Debug, Clone)]
struct BtnCap {
    page: u16,
    report_id: u8,
    link: u16,
    link_usage: u16,
    usage_min: u16,
    usage_max: u16,
}

pub enum ReadOutcome {
    Report(usize),
    Timeout,
    Failed(u32),
}

pub struct Device {
    handle: HANDLE,
    pp: isize,
    pub info: DeviceInfo,
    axes: Vec<ValCap>,
    hat: Option<ValCap>,
    button_caps: Vec<BtnCap>,
    pid_state_links: Vec<u16>,
    pid_block_index: Option<ValCap>,
    gain: Option<ValCap>,
    out_vals: Vec<ValCap>,
    out_btns: Vec<BtnCap>,
    feat_vals: Vec<ValCap>,
    feat_btns: Vec<BtnCap>,
    /// Serializa as sequências de criação e remoção de efeitos.
    effects: Mutex<()>,
    /// Bloco do efeito de força constante usado pela malha de centralização.
    constant: Mutex<Option<u8>>,
    /// Bloco do senoide contínuo usado pela vibração do motor.
    periodic: Mutex<Option<u8>>,
    /// Quando preenchido, `write` guarda os relatórios aqui em vez de enviá-los.
    capture: Mutex<Option<Vec<Vec<u8>>>>,
    last_write: Mutex<Instant>,
}

// O HANDLE e os dados pré-analisados são seguros para uso concorrente: cada
// operação de E/S usa sua própria estrutura OVERLAPPED e seu próprio evento.
unsafe impl Send for Device {}
unsafe impl Sync for Device {}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            CancelIoEx(self.handle, null_mut());
            HidD_FreePreparsedData(self.pp);
            CloseHandle(self.handle);
        }
    }
}

fn u16_at(b: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([b[o], b[o + 1]])
}
fn i32_at(b: &[u8], o: usize) -> i32 {
    i32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

fn axis_name(usage: u16) -> &'static str {
    match usage {
        0x30 => "X",
        0x31 => "Y",
        0x32 => "Z",
        0x33 => "Rx",
        0x34 => "Ry",
        0x35 => "Rz",
        0x36 => "Slider",
        0x37 => "Dial",
        0x38 => "Wheel",
        _ => "?",
    }
}

/// HIDP_VALUE_CAPS tem 72 bytes; os campos são lidos por deslocamento.
unsafe fn value_caps(pp: isize, report_type: i32, count: u16) -> Vec<ValCap> {
    if count == 0 {
        return Vec::new();
    }
    let mut n = count;
    let mut buf = vec![0u8; 72 * count as usize];
    if HidP_GetValueCaps(report_type, buf.as_mut_ptr(), &mut n, pp) != HIDP_STATUS_SUCCESS {
        return Vec::new();
    }
    (0..n as usize)
        .map(|i| {
            let c = &buf[i * 72..(i + 1) * 72];
            let bits = u16_at(c, 18);
            let (mut min, mut max) = (i32_at(c, 40), i32_at(c, 44));
            if min > max {
                // Descritores que declaram 0..65535 em 16 bits podem voltar com sinal trocado.
                min = 0;
                max = if bits >= 31 { i32::MAX } else { (1i32 << bits) - 1 };
            }
            ValCap {
                page: u16_at(c, 0),
                report_id: c[2],
                link: u16_at(c, 6),
                link_usage: u16_at(c, 8),
                usage_min: u16_at(c, 56),
                bits,
                min,
                max,
            }
        })
        .collect()
}

/// HIDP_BUTTON_CAPS também tem 72 bytes.
unsafe fn button_caps(pp: isize, report_type: i32, count: u16) -> Vec<BtnCap> {
    if count == 0 {
        return Vec::new();
    }
    let mut n = count;
    let mut buf = vec![0u8; 72 * count as usize];
    if HidP_GetButtonCaps(report_type, buf.as_mut_ptr(), &mut n, pp) != HIDP_STATUS_SUCCESS {
        return Vec::new();
    }
    (0..n as usize)
        .map(|i| {
            let c = &buf[i * 72..(i + 1) * 72];
            BtnCap {
                page: u16_at(c, 0),
                report_id: c[2],
                link: u16_at(c, 6),
                link_usage: u16_at(c, 8),
                usage_min: u16_at(c, 56),
                usage_max: if c[12] != 0 { u16_at(c, 58) } else { u16_at(c, 56) },
            }
        })
        .collect()
}

unsafe fn hid_string(h: HANDLE, f: unsafe extern "system" fn(HANDLE, *mut c_void, u32) -> BOOLEAN) -> String {
    let mut buf = [0u16; 127];
    if f(h, buf.as_mut_ptr() as *mut c_void, (buf.len() * 2) as u32) == 0 {
        return String::new();
    }
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

impl Device {
    pub fn open(path: &str) -> Result<Device, String> {
        unsafe {
            let wpath = wide(path);
            let handle = CreateFileW(
                wpath.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                null_mut(),
                OPEN_EXISTING,
                FILE_FLAG_OVERLAPPED,
                0,
            );
            if handle == INVALID_HANDLE_VALUE {
                return Err(format!("CreateFile falhou (erro Win32 {})", GetLastError()));
            }
            let mut pp = 0isize;
            if HidD_GetPreparsedData(handle, &mut pp) == 0 {
                CloseHandle(handle);
                return Err("HidD_GetPreparsedData falhou".into());
            }
            let mut caps = [0u8; 64];
            if HidP_GetCaps(pp, caps.as_mut_ptr()) != HIDP_STATUS_SUCCESS {
                HidD_FreePreparsedData(pp);
                CloseHandle(handle);
                return Err("HidP_GetCaps falhou".into());
            }
            let (input_len, output_len, feature_len) = (u16_at(&caps, 4), u16_at(&caps, 6), u16_at(&caps, 8));

            let in_vals = value_caps(pp, HIDP_INPUT, u16_at(&caps, 48));
            let in_btns = button_caps(pp, HIDP_INPUT, u16_at(&caps, 46));
            let out_vals = value_caps(pp, HIDP_OUTPUT, u16_at(&caps, 54));
            let feat_vals = value_caps(pp, HIDP_FEATURE, u16_at(&caps, 60));
            let out_btns = button_caps(pp, HIDP_OUTPUT, u16_at(&caps, 52));
            let feat_btns = button_caps(pp, HIDP_FEATURE, u16_at(&caps, 58));

            let mut axes: Vec<ValCap> = in_vals
                .iter()
                .filter(|v| v.page == PAGE_GENERIC && (0x30..=0x38).contains(&v.usage_min))
                .cloned()
                .collect();
            axes.sort_by_key(|v| v.usage_min);
            let hat = in_vals.iter().find(|v| v.page == PAGE_GENERIC && v.usage_min == USAGE_HAT).cloned();
            let pid_block_index = in_vals.iter().find(|v| v.page == PAGE_PID && v.usage_min == PID_EFFECT_BLOCK_INDEX).cloned();
            let gain = out_vals.iter().find(|v| v.page == PAGE_PID && v.usage_min == PID_DEVICE_GAIN).cloned();

            let button_caps: Vec<BtnCap> = in_btns.iter().filter(|b| b.page == PAGE_BUTTON).cloned().collect();
            let button_count = button_caps.iter().map(|b| b.usage_max).max().unwrap_or(0);
            let mut pid_state_links: Vec<u16> = in_btns.iter().filter(|b| b.page == PAGE_PID).map(|b| b.link).collect();
            pid_state_links.sort_unstable();
            pid_state_links.dedup();
            let pid_state_report = in_btns.iter().find(|b| b.page == PAGE_PID).map(|b| b.report_id);

            let mut attr = HIDD_ATTRIBUTES { Size: std::mem::size_of::<HIDD_ATTRIBUTES>() as u32, ..Default::default() };
            HidD_GetAttributes(handle, &mut attr);

            // PID Pool (somente leitura): capacidade de efeitos do firmware.
            let mut pid_pool = None;
            if let Some(ram) = feat_vals.iter().find(|v| v.page == PAGE_PID && v.usage_min == PID_RAM_POOL_SIZE) {
                let mut fb = vec![0u8; feature_len as usize];
                fb[0] = ram.report_id;
                if HidD_GetFeature(handle, fb.as_mut_ptr() as *mut c_void, fb.len() as u32) != 0 {
                    let read = |usage: u16| -> u32 {
                        let link = feat_vals.iter().find(|v| v.page == PAGE_PID && v.usage_min == usage).map(|v| v.link).unwrap_or(0);
                        let mut val = 0u32;
                        HidP_GetUsageValue(HIDP_FEATURE, PAGE_PID, link, usage, &mut val, pp, fb.as_ptr(), fb.len() as u32);
                        val
                    };
                    pid_pool = Some(PidPool {
                        ram_pool_size: read(PID_RAM_POOL_SIZE),
                        simultaneous_effects_max: read(PID_SIMULTANEOUS_MAX),
                    });
                }
            }

            let info = DeviceInfo {
                product: hid_string(handle, HidD_GetProductString),
                manufacturer: hid_string(handle, HidD_GetManufacturerString),
                serial: hid_string(handle, HidD_GetSerialNumberString),
                vid: attr.VendorID,
                pid: attr.ProductID,
                revision: attr.VersionNumber,
                path: path.to_string(),
                input_len,
                output_len,
                feature_len,
                axes: axes
                    .iter()
                    .map(|a| AxisInfo { usage: a.usage_min, name: axis_name(a.usage_min).into(), bits: a.bits, min: a.min, max: a.max })
                    .collect(),
                button_count,
                has_hat: hat.is_some(),
                has_pid_state: pid_state_report.is_some(),
                has_device_gain: gain.is_some(),
                pid_pool,
            };

            Ok(Device {
                handle,
                pp,
                info,
                axes,
                hat,
                button_caps,
                pid_state_links,
                pid_block_index,
                gain,
                out_vals,
                out_btns,
                feat_vals,
                feat_btns,
                effects: Mutex::new(()),
                constant: Mutex::new(None),
                periodic: Mutex::new(None),
                capture: Mutex::new(None),
                last_write: Mutex::new(Instant::now() - MIN_WRITE_INTERVAL),
            })
        }
    }

    pub fn create_event() -> HANDLE {
        unsafe { CreateEventW(null_mut(), 1, 0, std::ptr::null()) }
    }

    /// Lê um relatório de entrada, aguardando no máximo `timeout_ms`.
    /// `ov` e `buf` precisam sobreviver até a operação terminar; por isso, em caso
    /// de timeout a leitura continua pendente e deve ser retomada com `wait_read`.
    pub fn start_read(&self, buf: &mut [u8], ov: &mut OVERLAPPED) -> Result<(), u32> {
        unsafe {
            ResetEvent(ov.hEvent);
            if ReadFile(self.handle, buf.as_mut_ptr(), buf.len() as u32, null_mut(), ov) == 0 {
                let e = GetLastError();
                if e != ERROR_IO_PENDING {
                    return Err(e);
                }
            }
            Ok(())
        }
    }

    pub fn wait_read(&self, ov: &mut OVERLAPPED, timeout_ms: u32) -> ReadOutcome {
        unsafe {
            match WaitForSingleObject(ov.hEvent, timeout_ms) {
                WAIT_OBJECT_0 => {
                    let mut n = 0u32;
                    if GetOverlappedResult(self.handle, ov, &mut n, 0) == 0 {
                        ReadOutcome::Failed(GetLastError())
                    } else {
                        ReadOutcome::Report(n as usize)
                    }
                }
                WAIT_TIMEOUT => ReadOutcome::Timeout,
                _ => ReadOutcome::Failed(GetLastError()),
            }
        }
    }

    /// Cancela uma leitura pendente e aguarda sua conclusão.
    pub fn cancel_read(&self, ov: &mut OVERLAPPED) {
        unsafe {
            CancelIoEx(self.handle, ov);
            let mut n = 0u32;
            GetOverlappedResult(self.handle, ov, &mut n, 1);
        }
    }

    /// Decodifica um relatório de entrada sobre `t`. Retorna true se algo foi atualizado.
    pub fn decode(&self, report: &[u8], t: &mut Telemetry) -> bool {
        if report.is_empty() {
            return false;
        }
        let rid = report[0];
        let len = report.len() as u32;
        let mut touched = false;
        unsafe {
            if self.axes.first().map(|a| a.report_id) == Some(rid) {
                if t.axes.len() != self.axes.len() {
                    t.axes = vec![0; self.axes.len()];
                }
                for (i, a) in self.axes.iter().enumerate() {
                    let mut raw = 0u32;
                    if HidP_GetUsageValue(HIDP_INPUT, a.page, a.link, a.usage_min, &mut raw, self.pp, report.as_ptr(), len)
                        == HIDP_STATUS_SUCCESS
                    {
                        t.axes[i] = if a.min < 0 && a.bits < 32 {
                            // Extensão de sinal para eixos com intervalo lógico negativo.
                            let shift = 32 - a.bits as u32;
                            ((raw << shift) as i32) >> shift
                        } else {
                            raw as i32
                        };
                    }
                }
                if let Some(h) = &self.hat {
                    let mut raw = 0u32;
                    if HidP_GetUsageValue(HIDP_INPUT, h.page, h.link, h.usage_min, &mut raw, self.pp, report.as_ptr(), len)
                        == HIDP_STATUS_SUCCESS
                    {
                        let v = raw as i32;
                        t.hat = if v >= h.min && v <= h.max { Some((v - h.min) as u8) } else { None };
                    }
                }
                t.buttons.clear();
                for b in self.button_caps.iter().filter(|b| b.report_id == rid) {
                    let mut usages = [0u16; 64];
                    let mut n = usages.len() as u32;
                    if HidP_GetUsages(HIDP_INPUT, PAGE_BUTTON, b.link, usages.as_mut_ptr(), &mut n, self.pp, report.as_ptr(), len)
                        == HIDP_STATUS_SUCCESS
                    {
                        t.buttons.extend_from_slice(&usages[..n as usize]);
                    }
                }
                t.buttons.sort_unstable();
                t.buttons.dedup();
                touched = true;
            } else if !self.pid_state_links.is_empty() {
                let mut on: Vec<u16> = Vec::new();
                let mut any = false;
                for &link in &self.pid_state_links {
                    let mut usages = [0u16; 16];
                    let mut n = usages.len() as u32;
                    if HidP_GetUsages(HIDP_INPUT, PAGE_PID, link, usages.as_mut_ptr(), &mut n, self.pp, report.as_ptr(), len)
                        == HIDP_STATUS_SUCCESS
                    {
                        any = true;
                        on.extend_from_slice(&usages[..n as usize]);
                    }
                }
                if any {
                    let mut block = 0u32;
                    if let Some(b) = &self.pid_block_index {
                        HidP_GetUsageValue(HIDP_INPUT, PAGE_PID, b.link, b.usage_min, &mut block, self.pp, report.as_ptr(), len);
                    }
                    t.pid = Some(PidState {
                        effect_playing: on.contains(&PID_EFFECT_PLAYING),
                        actuators_enabled: on.contains(&PID_ACTUATORS_ENABLED),
                        paused: on.contains(&PID_DEVICE_PAUSED),
                        safety_switch: on.contains(&PID_SAFETY_SWITCH),
                        actuator_power: on.contains(&PID_ACTUATOR_POWER),
                        effect_block_index: block,
                    });
                    touched = true;
                }
            }
        }
        touched
    }

    /// Envia o relatório PID padrão "Device Gain" (ganho global de força).
    /// `percent` é validado em 0..=100 e convertido para o intervalo lógico do
    /// descritor. Reduzir o ganho nunca gera movimento do volante.
    pub fn set_device_gain(&self, percent: u8) -> Result<(), String> {
        if percent > 100 {
            return Err("ganho fora do intervalo 0..100".into());
        }
        let cap = self.gain.as_ref().ok_or("o dispositivo não expõe o relatório Device Gain")?;
        let value = ((percent as i64 * cap.max as i64 + 50) / 100) as u32;
        let mut report = vec![0u8; self.info.output_len as usize];
        unsafe {
            let len = report.len() as u32;
            // O buffer já está zerado; basta o Report ID no primeiro byte.
            // (HidP_InitializeReportForID devolve REPORT_DOES_NOT_EXIST neste
            // descritor mesmo para IDs válidos, então não é usado.)
            report[0] = cap.report_id;
            let st = HidP_SetUsageValue(HIDP_OUTPUT, PAGE_PID, cap.link, PID_DEVICE_GAIN, value, self.pp, report.as_mut_ptr(), len);
            if st != HIDP_STATUS_SUCCESS {
                return Err(format!("HidP_SetUsageValue falhou (status 0x{:08X})", st as u32));
            }
        }
        self.write(&report)
    }

    // ---- Efeitos PID criados por este app ---------------------------------

    fn out_val(&self, usage: u16, collection: u16) -> Result<&ValCap, String> {
        self.out_vals
            .iter()
            .find(|v| v.page == PAGE_PID && v.usage_min == usage && v.link_usage == collection)
            .ok_or_else(|| format!("campo PID 0x{usage:02X} ausente no relatório 0x{collection:02X}"))
    }

    fn btn(caps: &[BtnCap], page: u16, usage: u16, collection: u16) -> Result<&BtnCap, String> {
        caps.iter()
            .find(|b| b.page == page && b.link_usage == collection && usage >= b.usage_min && usage <= b.usage_max)
            .ok_or_else(|| format!("seletor 0x{usage:02X} ausente na coleção 0x{collection:02X}"))
    }

    unsafe fn put(&self, rtype: i32, cap: &ValCap, value: i32, report: &mut [u8]) -> Result<(), String> {
        let mask = if cap.bits >= 32 { u32::MAX } else { (1u32 << cap.bits) - 1 };
        let st = HidP_SetUsageValue(rtype, cap.page, cap.link, cap.usage_min, (value as u32) & mask, self.pp, report.as_mut_ptr(), report.len() as u32);
        if st != HIDP_STATUS_SUCCESS {
            return Err(format!("HidP_SetUsageValue(0x{:02X}) falhou (0x{:08X})", cap.usage_min, st as u32));
        }
        Ok(())
    }

    unsafe fn select(&self, rtype: i32, cap: &BtnCap, usage: u16, report: &mut [u8]) -> Result<(), String> {
        let mut list = [usage];
        let mut n = 1u32;
        let st = HidP_SetUsages(rtype, cap.page, cap.link, list.as_mut_ptr(), &mut n, self.pp, report.as_mut_ptr(), report.len() as u32);
        if st != HIDP_STATUS_SUCCESS {
            return Err(format!("HidP_SetUsages(0x{usage:02X}) falhou (0x{:08X})", st as u32));
        }
        Ok(())
    }

    fn out_report(&self, report_id: u8) -> Vec<u8> {
        let mut r = vec![0u8; self.info.output_len as usize];
        r[0] = report_id;
        r
    }

    fn send_device_control(&self, cmd: DeviceControl) -> Result<(), String> {
        let cap = Self::btn(&self.out_btns, PAGE_PID, cmd.usage(), COL_DEVICE_CONTROL)?;
        let mut r = self.out_report(cap.report_id);
        unsafe { self.select(HIDP_OUTPUT, cap, cmd.usage(), &mut r)? };
        self.write(&r)
    }

    /// Envia um comando PID "Device Control".
    pub fn device_control(&self, cmd: DeviceControl) -> Result<(), String> {
        let _guard = self.effects.lock().map_err(|_| "trava de efeitos corrompida")?;
        self.send_device_control(cmd)?;
        if matches!(cmd, DeviceControl::Reset | DeviceControl::StopAllEffects) {
            // O firmware descarta os efeitos; esquecer os blocos criados pelo app.
            if let Ok(mut c) = self.constant.lock() {
                *c = None;
            }
            if let Ok(mut c) = self.periodic.lock() {
                *c = None;
            }
        }
        Ok(())
    }

    /// Create New Effect (feature) + Block Load: devolve o índice do bloco alocado.
    fn create_effect(&self, effect_type: u16) -> Result<u8, String> {
        unsafe {
            let cap = Self::btn(&self.feat_btns, PAGE_PID, effect_type, COL_EFFECT_TYPE)?;
            let mut fb = vec![0u8; self.info.feature_len as usize];
            fb[0] = cap.report_id;
            self.select(HIDP_FEATURE, cap, effect_type, &mut fb)?;
            if HidD_SetFeature(self.handle, fb.as_mut_ptr() as *mut c_void, fb.len() as u32) == 0 {
                return Err(format!("Create New Effect falhou (erro Win32 {})", GetLastError()));
            }

            let ebi = self
                .feat_vals
                .iter()
                .find(|v| v.page == PAGE_PID && v.usage_min == PID_EFFECT_BLOCK_INDEX)
                .ok_or("relatório Block Load ausente")?;
            let mut rb = vec![0u8; self.info.feature_len as usize];
            rb[0] = ebi.report_id;
            if HidD_GetFeature(self.handle, rb.as_mut_ptr() as *mut c_void, rb.len() as u32) == 0 {
                return Err(format!("Block Load falhou (erro Win32 {})", GetLastError()));
            }
            let len = rb.len() as u32;
            let mut status: Vec<u16> = Vec::new();
            let mut links: Vec<u16> = self
                .feat_btns
                .iter()
                .filter(|b| b.page == PAGE_PID && b.report_id == ebi.report_id)
                .map(|b| b.link)
                .collect();
            links.sort_unstable();
            links.dedup();
            for link in links {
                let mut usages = [0u16; 8];
                let mut n = usages.len() as u32;
                if HidP_GetUsages(HIDP_FEATURE, PAGE_PID, link, usages.as_mut_ptr(), &mut n, self.pp, rb.as_ptr(), len) == HIDP_STATUS_SUCCESS {
                    status.extend_from_slice(&usages[..n as usize]);
                }
            }
            if status.contains(&PID_BLOCK_LOAD_FULL) {
                return Err("memória de efeitos do volante cheia".into());
            }
            if status.contains(&PID_BLOCK_LOAD_ERROR) {
                return Err("o volante recusou a criação do efeito".into());
            }
            let mut idx = 0u32;
            HidP_GetUsageValue(HIDP_FEATURE, PAGE_PID, ebi.link, PID_EFFECT_BLOCK_INDEX, &mut idx, self.pp, rb.as_ptr(), len);
            if idx == 0 || idx as i32 > ebi.max {
                return Err(format!(
                    "Block Load devolveu índice inválido (bytes {rb:02X?}, sucesso={})",
                    status.contains(&PID_BLOCK_LOAD_SUCCESS)
                ));
            }
            Ok(idx as u8)
        }
    }

    /// Set Effect no eixo X, com ganho do efeito no máximo e direção de 90°.
    /// `duration`: 0xFFFF = infinito (convenção PID); `no_trigger` envia 0xFF
    /// no botão de gatilho, como o DirectInput faz para "sem gatilho".
    fn set_effect(&self, block: u8, effect_type: u16, duration: i32, no_trigger: bool) -> Result<(), String> {
        unsafe {
            let ebi = self.out_val(PID_EFFECT_BLOCK_INDEX, COL_SET_EFFECT)?;
            let mut r = self.out_report(ebi.report_id);
            self.put(HIDP_OUTPUT, ebi, block as i32, &mut r)?;
            let et = Self::btn(&self.out_btns, PAGE_PID, effect_type, COL_EFFECT_TYPE)?;
            self.select(HIDP_OUTPUT, et, effect_type, &mut r)?;
            self.put(HIDP_OUTPUT, self.out_val(PID_DURATION, COL_SET_EFFECT)?, duration, &mut r)?;
            let gain = self.out_val(PID_GAIN, COL_SET_EFFECT)?;
            self.put(HIDP_OUTPUT, gain, gain.max, &mut r)?;
            if no_trigger {
                self.put(HIDP_OUTPUT, self.out_val(PID_TRIGGER_BUTTON, COL_SET_EFFECT)?, 0xFF, &mut r)?;
            }
            let axis = Self::btn(&self.out_btns, PAGE_GENERIC, USAGE_X, COL_AXES_ENABLE)?;
            self.select(HIDP_OUTPUT, axis, USAGE_X, &mut r)?;
            // Direção de 90° (eixo X), como o DirectInput envia para volantes.
            let de = Self::btn(&self.out_btns, PAGE_PID, PID_DIRECTION_ENABLE, COL_SET_EFFECT)?;
            self.select(HIDP_OUTPUT, de, PID_DIRECTION_ENABLE, &mut r)?;
            let dir = self
                .out_vals
                .iter()
                .find(|v| v.page == PAGE_ORDINAL && v.usage_min == 1 && v.link_usage == COL_DIRECTION)
                .ok_or("campo Direction ausente no relatório Set Effect")?;
            self.put(HIDP_OUTPUT, dir, (dir.max + 2) / 4, &mut r)?;
            self.write(&r)
        }
    }

    /// Set Periodic: magnitude em 1/10000 da força máxima e período em ms.
    fn set_periodic(&self, block: u8, magnitude: i32, period_ms: i32) -> Result<(), String> {
        unsafe {
            let ebi = self.out_val(PID_EFFECT_BLOCK_INDEX, COL_SET_PERIODIC)?;
            let mut r = self.out_report(ebi.report_id);
            self.put(HIDP_OUTPUT, ebi, block as i32, &mut r)?;
            let mag = self.out_val(PID_MAGNITUDE, COL_SET_PERIODIC)?;
            self.put(HIDP_OUTPUT, mag, magnitude * mag.max / 10000, &mut r)?;
            self.put(HIDP_OUTPUT, self.out_val(PID_PERIOD, COL_SET_PERIODIC)?, period_ms, &mut r)?;
            self.write(&r)
        }
    }

    /// `loops` < 0 usa o valor máximo do campo (repetição contínua).
    fn effect_operation(&self, block: u8, op: u16, loops: i32) -> Result<(), String> {
        unsafe {
            let ebi = self.out_val(PID_EFFECT_BLOCK_INDEX, COL_EFFECT_OPERATION_REPORT)?;
            let mut r = self.out_report(ebi.report_id);
            self.put(HIDP_OUTPUT, ebi, block as i32, &mut r)?;
            let cap = Self::btn(&self.out_btns, PAGE_PID, op, COL_EFFECT_OPERATION)?;
            self.select(HIDP_OUTPUT, cap, op, &mut r)?;
            if op == PID_OP_START {
                let lc = self.out_val(PID_LOOP_COUNT, COL_EFFECT_OPERATION_REPORT)?;
                self.put(HIDP_OUTPUT, lc, if loops < 0 { lc.max } else { loops.min(lc.max) }, &mut r)?;
            }
            self.write(&r)
        }
    }

    fn block_free(&self, block: u8) -> Result<(), String> {
        unsafe {
            let ebi = self.out_val(PID_EFFECT_BLOCK_INDEX, COL_BLOCK_FREE)?;
            let mut r = self.out_report(ebi.report_id);
            self.put(HIDP_OUTPUT, ebi, block as i32, &mut r)?;
            self.write(&r)
        }
    }

    // ---- Força constante (usada pela malha de centralização do app) -------

    fn set_constant(&self, block: u8, magnitude: i32) -> Result<(), String> {
        unsafe {
            let ebi = self.out_val(PID_EFFECT_BLOCK_INDEX, COL_SET_CONSTANT)?;
            let mut r = self.out_report(ebi.report_id);
            self.put(HIDP_OUTPUT, ebi, block as i32, &mut r)?;
            let mag = self.out_val(PID_MAGNITUDE, COL_SET_CONSTANT)?;
            self.put(HIDP_OUTPUT, mag, magnitude.clamp(-10000, 10000) * mag.max / 10000, &mut r)?;
            self.write(&r)
        }
    }

    /// Cria e inicia um efeito de força constante com magnitude zero.
    pub fn constant_start(&self) -> Result<u8, String> {
        let _serial = self.effects.lock().map_err(|_| "trava de efeitos corrompida")?;
        let mut guard = self.constant.lock().map_err(|_| "estado da força constante corrompido")?;
        if let Some(block) = *guard {
            return Ok(block);
        }
        self.send_device_control(DeviceControl::EnableActuators)?;
        self.set_device_gain(100)?;
        let block = self.create_effect(PID_ET_CONSTANT)?;
        let started = self
            .set_effect(block, PID_ET_CONSTANT, 30_000, true)
            .and_then(|_| self.set_constant(block, 0))
            .and_then(|_| self.effect_operation(block, PID_OP_START, 1));
        if let Err(e) = started {
            let _ = self.block_free(block);
            return Err(e);
        }
        *guard = Some(block);
        Ok(block)
    }

    /// Ajusta a força constante em 1/10000 da força máxima (-10000..=10000).
    pub fn constant_force(&self, magnitude: i32) -> Result<(), String> {
        let block = self.constant.lock().map_err(|_| "estado da força constante corrompido")?.ok_or("força constante não iniciada")?;
        self.set_constant(block, magnitude)
    }

    /// Reinicia o efeito (duração finita) para que não expire.
    pub fn constant_keepalive(&self) -> Result<(), String> {
        let block = self.constant.lock().map_err(|_| "estado da força constante corrompido")?.ok_or("força constante não iniciada")?;
        self.effect_operation(block, PID_OP_START, 1)
    }

    /// Zera, para e libera o efeito de força constante.
    pub fn constant_stop(&self) -> Result<(), String> {
        let block = self.constant.lock().map_err(|_| "estado da força constante corrompido")?.take();
        if let Some(block) = block {
            let zero = self.set_constant(block, 0);
            let stop = self.effect_operation(block, PID_OP_STOP, 0);
            let free = self.block_free(block);
            zero.and(stop).and(free)?;
        }
        Ok(())
    }

    // ---- Senoide contínua (vibração do motor nos efeitos de jogo) ----------

    /// Cria e inicia um efeito senoidal com amplitude zero.
    pub fn periodic_start(&self) -> Result<u8, String> {
        let _serial = self.effects.lock().map_err(|_| "trava de efeitos corrompida")?;
        let mut guard = self.periodic.lock().map_err(|_| "estado do efeito periódico corrompido")?;
        if let Some(block) = *guard {
            return Ok(block);
        }
        let block = self.create_effect(PID_ET_SINE)?;
        let started = self
            .set_effect(block, PID_ET_SINE, 30_000, true)
            .and_then(|_| self.set_periodic(block, 0, 33))
            .and_then(|_| self.effect_operation(block, PID_OP_START, 1));
        if let Err(e) = started {
            let _ = self.block_free(block);
            return Err(e);
        }
        *guard = Some(block);
        Ok(block)
    }

    /// Amplitude em 1/10000 da força máxima e período em ms.
    pub fn periodic_set(&self, magnitude: i32, period_ms: i32) -> Result<(), String> {
        let block = self.periodic.lock().map_err(|_| "estado do efeito periódico corrompido")?.ok_or("efeito periódico não iniciado")?;
        self.set_periodic(block, magnitude.clamp(0, 10000), period_ms.clamp(5, 1000))
    }

    pub fn periodic_keepalive(&self) -> Result<(), String> {
        let block = self.periodic.lock().map_err(|_| "estado do efeito periódico corrompido")?.ok_or("efeito periódico não iniciado")?;
        self.effect_operation(block, PID_OP_START, 1)
    }

    pub fn periodic_stop(&self) -> Result<(), String> {
        let block = self.periodic.lock().map_err(|_| "estado do efeito periódico corrompido")?.take();
        if let Some(block) = block {
            let zero = self.set_periodic(block, 0, 33);
            let stop = self.effect_operation(block, PID_OP_STOP, 0);
            let free = self.block_free(block);
            zero.and(stop).and(free)?;
        }
        Ok(())
    }

    /// Inicia uma vibração senoidal curta e fraca (não gira o volante), para
    /// verificar se o firmware executa efeitos PID enviados por este caminho.
    /// Devolve o bloco usado; chame `release_block` depois.
    pub fn vibration_start(&self, magnitude: i32, duration_ms: i32) -> Result<u8, String> {
        if !(1..=VIBRATION_MAX_MAGNITUDE).contains(&magnitude) || !(100..=VIBRATION_MAX_MS).contains(&duration_ms) {
            return Err("parâmetros da vibração fora do limite de segurança".into());
        }
        let _guard = self.effects.lock().map_err(|_| "trava de efeitos corrompida")?;
        self.send_device_control(DeviceControl::EnableActuators)?;
        self.set_device_gain(100)?;
        let block = self.create_effect(PID_ET_SINE)?;
        let started = self
            .set_effect(block, PID_ET_SINE, duration_ms, true)
            .and_then(|_| self.set_periodic(block, magnitude * 100, 40))
            .and_then(|_| self.effect_operation(block, PID_OP_START, 1));
        if let Err(e) = started {
            let _ = self.block_free(block);
            return Err(e);
        }
        Ok(block)
    }

    /// Para e libera um bloco criado por `vibration_start`.
    pub fn release_block(&self, block: u8) -> Result<(), String> {
        let _guard = self.effects.lock().map_err(|_| "trava de efeitos corrompida")?;
        let stop = self.effect_operation(block, PID_OP_STOP, 0);
        let free = self.block_free(block);
        stop.and(free)
    }

    /// Monta, sem enviar nada ao volante, os relatórios PID usados pelo app
    /// (força constante e vibração). Serve para conferir no log que o
    /// descritor tem todos os campos necessários.
    pub fn pid_dry_run(&self) -> Result<String, String> {
        let _guard = self.effects.lock().map_err(|_| "trava de efeitos corrompida")?;
        *self.capture.lock().map_err(|_| "captura corrompida")? = Some(Vec::new());
        let built = self
            .set_effect(1, PID_ET_CONSTANT, 30_000, true)
            .and_then(|_| self.set_constant(1, 1500))
            .and_then(|_| self.effect_operation(1, PID_OP_START, 1))
            .and_then(|_| self.set_effect(2, PID_ET_SINE, 1000, true))
            .and_then(|_| self.set_periodic(2, 2000, 40));
        let reports = self.capture.lock().map_err(|_| "captura corrompida")?.take().unwrap_or_default();
        built?;
        Ok(reports
            .iter()
            .map(|r| r.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(" "))
            .collect::<Vec<_>>()
            .join(" | "))
    }

    /// Escrita serializada e com limite de taxa.
    fn write(&self, report: &[u8]) -> Result<(), String> {
        if let Ok(mut cap) = self.capture.lock() {
            if let Some(list) = cap.as_mut() {
                list.push(report.to_vec());
                return Ok(());
            }
        }
        let mut last = self.last_write.lock().map_err(|_| "lock de escrita corrompido")?;
        let since = last.elapsed();
        if since < MIN_WRITE_INTERVAL {
            std::thread::sleep(MIN_WRITE_INTERVAL - since);
        }
        let result = unsafe {
            let ev = Self::create_event();
            let mut ov = OVERLAPPED::with_event(ev);
            let mut res = Ok(());
            if WriteFile(self.handle, report.as_ptr(), report.len() as u32, null_mut(), &mut ov) == 0 {
                let e = GetLastError();
                if e != ERROR_IO_PENDING {
                    res = Err(format!("WriteFile falhou (erro Win32 {e})"));
                } else if WaitForSingleObject(ev, WRITE_TIMEOUT_MS) != WAIT_OBJECT_0 {
                    CancelIoEx(self.handle, &mut ov);
                    let mut n = 0u32;
                    GetOverlappedResult(self.handle, &mut ov, &mut n, 1);
                    res = Err("tempo esgotado ao escrever no dispositivo".into());
                } else {
                    let mut n = 0u32;
                    if GetOverlappedResult(self.handle, &mut ov, &mut n, 0) == 0 {
                        res = Err(format!("escrita não concluída (erro Win32 {})", GetLastError()));
                    }
                }
            }
            CloseHandle(ev);
            res
        };
        *last = Instant::now();
        result
    }
}
