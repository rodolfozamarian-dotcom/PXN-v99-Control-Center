//! Descoberta do PXN V99 pela enumeração de interfaces HID presentes.

use super::ffi::*;
use std::ptr::null_mut;

pub const PXN_VID: u16 = 0x11FF;
/// PID apresentado em modo DirectInput (HID joystick + HID PID force feedback).
pub const V99_PID_DINPUT: u16 = 0x3245;
/// PID apresentado em modo XInput ("Controller (PXN-V99)").
pub const V99_PID_XINPUT: u16 = 0xF201;

#[derive(Debug, Clone, PartialEq)]
pub enum Found {
    /// Interface joystick/FFB em DirectInput; contém o caminho da interface HID.
    DirectInput(String),
    /// Volante presente, porém em modo XInput (sem acesso HID PID).
    XInput,
    None,
}

/// Lista os caminhos de todas as interfaces HID presentes (em minúsculas).
pub fn hid_interface_paths() -> Vec<String> {
    let mut out = Vec::new();
    unsafe {
        let mut guid = GUID::default();
        HidD_GetHidGuid(&mut guid);
        let set = SetupDiGetClassDevsW(&guid, std::ptr::null(), 0, DIGCF_PRESENT | DIGCF_DEVICEINTERFACE);
        if set == INVALID_HANDLE_VALUE {
            return out;
        }
        let mut index = 0u32;
        loop {
            let mut data = SP_DEVICE_INTERFACE_DATA {
                cbSize: std::mem::size_of::<SP_DEVICE_INTERFACE_DATA>() as u32,
                InterfaceClassGuid: GUID::default(),
                Flags: 0,
                Reserved: 0,
            };
            if SetupDiEnumDeviceInterfaces(set, null_mut(), &guid, index, &mut data) == 0 {
                break;
            }
            index += 1;

            let mut required = 0u32;
            SetupDiGetDeviceInterfaceDetailW(set, &mut data, null_mut(), 0, &mut required, null_mut());
            if required < 6 {
                continue;
            }
            // SP_DEVICE_INTERFACE_DETAIL_DATA_W: cbSize (u32) seguido do caminho em UTF-16.
            let mut buf = vec![0u8; required as usize + 2];
            let cb: u32 = if cfg!(target_pointer_width = "64") { 8 } else { 6 };
            std::ptr::write_unaligned(buf.as_mut_ptr() as *mut u32, cb);
            if SetupDiGetDeviceInterfaceDetailW(set, &mut data, buf.as_mut_ptr(), required, null_mut(), null_mut()) == 0 {
                continue;
            }
            let chars: Vec<u16> = buf[4..]
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .take_while(|&c| c != 0)
                .collect();
            out.push(String::from_utf16_lossy(&chars).to_lowercase());
        }
        SetupDiDestroyDeviceInfoList(set);
    }
    out
}

pub fn find_v99() -> Found {
    let paths = hid_interface_paths();
    let dinput = format!("vid_{:04x}&pid_{:04x}", PXN_VID, V99_PID_DINPUT);
    let xinput = format!("vid_{:04x}&pid_{:04x}", PXN_VID, V99_PID_XINPUT);
    // A interface 0 (mi_00) carrega o joystick e o HID PID; a mi_01 é teclado/mouse.
    if let Some(p) = paths.iter().find(|p| p.contains(&dinput) && p.contains("&mi_00")) {
        return Found::DirectInput(p.clone());
    }
    if paths.iter().any(|p| p.contains(&xinput)) {
        return Found::XInput;
    }
    Found::None
}
