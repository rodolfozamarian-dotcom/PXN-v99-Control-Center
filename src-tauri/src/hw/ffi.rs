//! Declarações mínimas da API do Windows usadas pela camada de hardware.
//! Mantidas à mão (em vez de um crate de bindings) para que cada chamada ao
//! hardware fique explícita e auditável.
#![allow(non_snake_case, non_camel_case_types, clippy::upper_case_acronyms)]

use std::ffi::c_void;

pub type HANDLE = isize;
pub type BOOL = i32;
pub type BOOLEAN = u8;
pub type NTSTATUS = i32;

pub const INVALID_HANDLE_VALUE: HANDLE = -1;
pub const GENERIC_READ: u32 = 0x8000_0000;
pub const GENERIC_WRITE: u32 = 0x4000_0000;
pub const FILE_SHARE_READ: u32 = 1;
pub const FILE_SHARE_WRITE: u32 = 2;
pub const OPEN_EXISTING: u32 = 3;
pub const FILE_FLAG_OVERLAPPED: u32 = 0x4000_0000;
pub const ERROR_IO_PENDING: u32 = 997;
pub const ERROR_PIPE_CONNECTED: u32 = 535;
pub const PIPE_ACCESS_INBOUND: u32 = 0x1;
pub const PIPE_TYPE_BYTE: u32 = 0x0;
pub const PIPE_READMODE_BYTE: u32 = 0x0;
pub const PIPE_WAIT: u32 = 0x0;
pub const PIPE_REJECT_REMOTE_CLIENTS: u32 = 0x8;
pub const PIPE_UNLIMITED_INSTANCES: u32 = 255;
pub const WAIT_OBJECT_0: u32 = 0;
pub const WAIT_TIMEOUT: u32 = 258;

pub const DIGCF_PRESENT: u32 = 0x02;
pub const DIGCF_DEVICEINTERFACE: u32 = 0x10;

pub const HIDP_STATUS_SUCCESS: NTSTATUS = 0x0011_0000;
pub const HIDP_INPUT: i32 = 0;
pub const HIDP_OUTPUT: i32 = 1;
pub const HIDP_FEATURE: i32 = 2;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct GUID {
    pub d1: u32,
    pub d2: u16,
    pub d3: u16,
    pub d4: [u8; 8],
}

#[repr(C)]
pub struct OVERLAPPED {
    pub Internal: usize,
    pub InternalHigh: usize,
    pub Offset: u32,
    pub OffsetHigh: u32,
    pub hEvent: HANDLE,
}

impl OVERLAPPED {
    pub fn with_event(ev: HANDLE) -> Self {
        Self { Internal: 0, InternalHigh: 0, Offset: 0, OffsetHigh: 0, hEvent: ev }
    }
}

#[repr(C)]
pub struct SP_DEVICE_INTERFACE_DATA {
    pub cbSize: u32,
    pub InterfaceClassGuid: GUID,
    pub Flags: u32,
    pub Reserved: usize,
}

#[repr(C)]
#[derive(Default)]
pub struct HIDD_ATTRIBUTES {
    pub Size: u32,
    pub VendorID: u16,
    pub ProductID: u16,
    pub VersionNumber: u16,
}

#[link(name = "kernel32")]
extern "system" {
    pub fn CreateFileW(
        name: *const u16,
        access: u32,
        share: u32,
        sa: *mut c_void,
        disposition: u32,
        flags: u32,
        template: HANDLE,
    ) -> HANDLE;
    pub fn ReadFile(h: HANDLE, buf: *mut u8, len: u32, read: *mut u32, ov: *mut OVERLAPPED) -> BOOL;
    pub fn WriteFile(h: HANDLE, buf: *const u8, len: u32, written: *mut u32, ov: *mut OVERLAPPED) -> BOOL;
    pub fn GetOverlappedResult(h: HANDLE, ov: *mut OVERLAPPED, transferred: *mut u32, wait: BOOL) -> BOOL;
    pub fn CancelIoEx(h: HANDLE, ov: *mut OVERLAPPED) -> BOOL;
    pub fn CreateEventW(sa: *mut c_void, manual_reset: BOOL, initial: BOOL, name: *const u16) -> HANDLE;
    pub fn ResetEvent(h: HANDLE) -> BOOL;
    pub fn WaitForSingleObject(h: HANDLE, ms: u32) -> u32;
    pub fn CloseHandle(h: HANDLE) -> BOOL;
    pub fn GetLastError() -> u32;
    pub fn CreateNamedPipeW(
        name: *const u16,
        open_mode: u32,
        pipe_mode: u32,
        max_instances: u32,
        out_buffer: u32,
        in_buffer: u32,
        default_timeout: u32,
        sa: *mut c_void,
    ) -> HANDLE;
    pub fn ConnectNamedPipe(h: HANDLE, ov: *mut OVERLAPPED) -> BOOL;
    pub fn DisconnectNamedPipe(h: HANDLE) -> BOOL;
}

#[link(name = "hid")]
extern "system" {
    pub fn HidD_GetHidGuid(guid: *mut GUID);
    pub fn HidD_GetAttributes(h: HANDLE, attr: *mut HIDD_ATTRIBUTES) -> BOOLEAN;
    pub fn HidD_GetPreparsedData(h: HANDLE, pp: *mut isize) -> BOOLEAN;
    pub fn HidD_FreePreparsedData(pp: isize) -> BOOLEAN;
    pub fn HidD_GetProductString(h: HANDLE, buf: *mut c_void, len: u32) -> BOOLEAN;
    pub fn HidD_GetManufacturerString(h: HANDLE, buf: *mut c_void, len: u32) -> BOOLEAN;
    pub fn HidD_GetSerialNumberString(h: HANDLE, buf: *mut c_void, len: u32) -> BOOLEAN;
    pub fn HidD_GetFeature(h: HANDLE, buf: *mut c_void, len: u32) -> BOOLEAN;
    pub fn HidD_SetFeature(h: HANDLE, buf: *mut c_void, len: u32) -> BOOLEAN;

    pub fn HidP_GetCaps(pp: isize, caps: *mut u8) -> NTSTATUS;
    pub fn HidP_GetValueCaps(report_type: i32, caps: *mut u8, len: *mut u16, pp: isize) -> NTSTATUS;
    pub fn HidP_GetButtonCaps(report_type: i32, caps: *mut u8, len: *mut u16, pp: isize) -> NTSTATUS;
    pub fn HidP_GetUsageValue(
        report_type: i32,
        usage_page: u16,
        link_collection: u16,
        usage: u16,
        value: *mut u32,
        pp: isize,
        report: *const u8,
        report_len: u32,
    ) -> NTSTATUS;
    pub fn HidP_GetUsages(
        report_type: i32,
        usage_page: u16,
        link_collection: u16,
        usage_list: *mut u16,
        usage_len: *mut u32,
        pp: isize,
        report: *const u8,
        report_len: u32,
    ) -> NTSTATUS;
    pub fn HidP_SetUsages(
        report_type: i32,
        usage_page: u16,
        link_collection: u16,
        usage_list: *mut u16,
        usage_len: *mut u32,
        pp: isize,
        report: *mut u8,
        report_len: u32,
    ) -> NTSTATUS;
    pub fn HidP_SetUsageValue(
        report_type: i32,
        usage_page: u16,
        link_collection: u16,
        usage: u16,
        value: u32,
        pp: isize,
        report: *mut u8,
        report_len: u32,
    ) -> NTSTATUS;
}

#[link(name = "setupapi")]
extern "system" {
    pub fn SetupDiGetClassDevsW(guid: *const GUID, enumerator: *const u16, hwnd: isize, flags: u32) -> HANDLE;
    pub fn SetupDiEnumDeviceInterfaces(
        set: HANDLE,
        devinfo: *mut c_void,
        guid: *const GUID,
        index: u32,
        data: *mut SP_DEVICE_INTERFACE_DATA,
    ) -> BOOL;
    pub fn SetupDiGetDeviceInterfaceDetailW(
        set: HANDLE,
        data: *mut SP_DEVICE_INTERFACE_DATA,
        detail: *mut u8,
        size: u32,
        required: *mut u32,
        devinfo: *mut c_void,
    ) -> BOOL;
    pub fn SetupDiDestroyDeviceInfoList(set: HANDLE) -> BOOL;
}

#[link(name = "winmm")]
extern "system" {
    /// Resolução do temporizador do sistema, em ms (para a malha de 250 Hz).
    pub fn timeBeginPeriod(period: u32) -> u32;
    pub fn timeEndPeriod(period: u32) -> u32;
}

pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
