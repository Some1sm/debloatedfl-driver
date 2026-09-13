use std::time::Duration;

use crate::protocol::*;

#[repr(C)]
#[derive(Clone, Copy)]
struct GUID {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}

#[repr(C)]
struct SpDeviceInterfaceData {
    cb_size: u32,
    interface_class_guid: GUID,
    flags: u32,
    reserved: usize,
}

#[link(name = "kernel32")]
extern "system" {
    fn CreateFileW(
        lpFileName: *const u16,
        dwDesiredAccess: u32,
        dwShareMode: u32,
        lpSecurityAttributes: *mut std::ffi::c_void,
        dwCreationDisposition: u32,
        dwFlagsAndAttributes: u32,
        hTemplateFile: *mut std::ffi::c_void,
    ) -> *mut std::ffi::c_void;

    fn CloseHandle(hObject: *mut std::ffi::c_void) -> i32;
}

#[link(name = "hid")]
extern "system" {
    fn HidD_GetHidGuid(HidGuid: *mut GUID);
    fn HidD_GetProductString(HidDeviceObject: *mut std::ffi::c_void, Buffer: *mut u16, BufferLength: u32) -> i32;
    fn HidD_SetFeature(HidDeviceObject: *mut std::ffi::c_void, ReportBuffer: *const u8, ReportBufferLength: u32) -> i32;
    fn HidD_GetFeature(HidDeviceObject: *mut std::ffi::c_void, ReportBuffer: *mut u8, ReportBufferLength: u32) -> i32;
}

#[link(name = "setupapi")]
extern "system" {
    fn SetupDiGetClassDevsW(
        ClassGuid: *const GUID,
        Enumerator: *const u16,
        hwndParent: *mut std::ffi::c_void,
        Flags: u32,
    ) -> *mut std::ffi::c_void;

    fn SetupDiEnumDeviceInterfaces(
        DeviceInfoSet: *mut std::ffi::c_void,
        DeviceInfoData: *mut std::ffi::c_void,
        InterfaceClassGuid: *const GUID,
        MemberIndex: u32,
        DeviceInterfaceData: *mut SpDeviceInterfaceData,
    ) -> i32;

    fn SetupDiGetDeviceInterfaceDetailW(
        DeviceInfoSet: *mut std::ffi::c_void,
        DeviceInterfaceData: *mut SpDeviceInterfaceData,
        DeviceInterfaceDetailData: *mut u8,
        DeviceInterfaceDetailDataSize: u32,
        RequiredSize: *mut u32,
        DeviceInfoData: *mut std::ffi::c_void,
    ) -> i32;

    fn SetupDiDestroyDeviceInfoList(DeviceInfoSet: *mut std::ffi::c_void) -> i32;
}

const INVALID_HANDLE: *mut std::ffi::c_void = -1isize as *mut std::ffi::c_void;
const FILE_SHARE_READ: u32 = 1;
const FILE_SHARE_WRITE: u32 = 2;
const OPEN_EXISTING: u32 = 3;
const DIGCF_PRESENT: u32 = 0x02;
const DIGCF_DEVICEINTERFACE: u32 = 0x10;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KeyboardInfo {
    pub name: String,
    pub vid: u16,
    pub pid: u16,
    pub is_wireless: bool,
    pub path: String,
    pub firmware_date: String,
    pub battery_percent: Option<u8>,
}

pub struct KeyboardBackend {
    pub device_path: String,
    pub is_wireless: bool,
}

impl KeyboardBackend {
    pub fn connect() -> Result<Self, String> {
        let (dev_path, is_wireless) = Self::find_hid_device_native()?;
        Ok(Self {
            device_path: dev_path,
            is_wireless,
        })
    }

    /// Dynamic enumeration of HID devices matching VID_05AC & PID_024F via Windows SetupAPI
    pub fn find_hid_device_native() -> Result<(String, bool), String> {
        let mut hid_guid = GUID { data1: 0, data2: 0, data3: 0, data4: [0; 8] };
        unsafe { HidD_GetHidGuid(&mut hid_guid) };

        let dev_info = unsafe {
            SetupDiGetClassDevsW(
                &hid_guid,
                std::ptr::null(),
                std::ptr::null_mut(),
                DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
            )
        };

        if dev_info == INVALID_HANDLE || dev_info.is_null() {
            return Err("Failed to query Windows SetupAPI for HID devices".to_string());
        }

        let mut candidate_paths: Vec<String> = Vec::new();
        let mut index = 0;
        loop {
            let mut if_data = SpDeviceInterfaceData {
                cb_size: std::mem::size_of::<SpDeviceInterfaceData>() as u32,
                interface_class_guid: hid_guid,
                flags: 0,
                reserved: 0,
            };

            let ok = unsafe {
                SetupDiEnumDeviceInterfaces(
                    dev_info,
                    std::ptr::null_mut(),
                    &hid_guid,
                    index,
                    &mut if_data,
                )
            };

            if ok == 0 {
                break;
            }
            index += 1;

            let mut req_size = 0u32;
            unsafe {
                SetupDiGetDeviceInterfaceDetailW(
                    dev_info,
                    &mut if_data,
                    std::ptr::null_mut(),
                    0,
                    &mut req_size,
                    std::ptr::null_mut(),
                );
            }

            if req_size == 0 {
                continue;
            }

            let mut buffer = vec![0u8; req_size as usize];
            let cb_size: u32 = if std::mem::size_of::<usize>() == 8 { 8 } else { 5 };
            unsafe {
                std::ptr::copy_nonoverlapping(
                    &cb_size as *const u32 as *const u8,
                    buffer.as_mut_ptr(),
                    4,
                );
            }

            let ok_detail = unsafe {
                SetupDiGetDeviceInterfaceDetailW(
                    dev_info,
                    &mut if_data,
                    buffer.as_mut_ptr(),
                    req_size,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            };

            if ok_detail != 0 {
                let wide_slice = unsafe {
                    let ptr = buffer.as_ptr().add(4) as *const u16;
                    let len = (req_size as usize - 4) / 2;
                    std::slice::from_raw_parts(ptr, len)
                };
                let null_pos = wide_slice.iter().position(|&c| c == 0).unwrap_or(wide_slice.len());
                let path_str = String::from_utf16_lossy(&wide_slice[..null_pos]);

                let p_lower = path_str.to_lowercase();
                if p_lower.contains("05ac") && p_lower.contains("024f") && p_lower.contains("mi_00") {
                    candidate_paths.push(path_str);
                }
            }
        }

        unsafe { SetupDiDestroyDeviceInfoList(dev_info) };

        if candidate_paths.is_empty() {
            return Err("No FL-ESPORTS MK870 keyboard or 2.4G dongle found".to_string());
        }

        // Test candidates and check product strings
        let mut best_candidate = None;
        for p in candidate_paths {
            let p_kbd = if !p.to_lowercase().ends_with(r"\kbd") {
                format!(r"{}\kbd", p)
            } else {
                p.clone()
            };

            let wide_path: Vec<u16> = p_kbd.encode_utf16().chain(std::iter::once(0)).collect();
            let handle = unsafe {
                CreateFileW(
                    wide_path.as_ptr(),
                    0, // Query access
                    FILE_SHARE_READ | FILE_SHARE_WRITE,
                    std::ptr::null_mut(),
                    OPEN_EXISTING,
                    0,
                    std::ptr::null_mut(),
                )
            };

            if handle != INVALID_HANDLE && !handle.is_null() {
                let mut prod_buf = [0u16; 128];
                let ok_str = unsafe {
                    HidD_GetProductString(handle, prod_buf.as_mut_ptr(), 256)
                };
                unsafe { CloseHandle(handle) };

                let prod_name = if ok_str != 0 {
                    let null_pos = prod_buf.iter().position(|&c| c == 0).unwrap_or(prod_buf.len());
                    String::from_utf16_lossy(&prod_buf[..null_pos])
                } else {
                    String::new()
                };

                let is_wireless = prod_name.contains("2.4G");
                if !is_wireless && !prod_name.is_empty() {
                    // Wired connection detected! Prioritize immediately for full hardware configuration capability.
                    return Ok((p_kbd, false));
                } else if best_candidate.is_none() {
                    best_candidate = Some((p_kbd, true));
                }
            }
        }

        if let Some(cand) = best_candidate {
            return Ok(cand);
        }

        Err("Could not open handle to MK870 HID interface".to_string())
    }

    pub fn send_command(&self, cmd: &[u8; 64], cs_type: CheckSumType) -> Result<(), String> {
        let wide_path: Vec<u16> = self.device_path.encode_utf16().chain(std::iter::once(0)).collect();
        let handle = unsafe {
            CreateFileW(
                wide_path.as_ptr(),
                0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null_mut(),
                OPEN_EXISTING,
                0,
                std::ptr::null_mut(),
            )
        };

        if handle == INVALID_HANDLE || handle.is_null() {
            return Err("Failed to open HID handle to MK870".to_string());
        }

        let mut p_buf = *cmd;
        calculate_checksum(&mut p_buf, cs_type);

        let mut report = [0u8; 65];
        report[0] = 0x00; // Report ID 0
        report[1..65].copy_from_slice(&p_buf);

        let ok = unsafe { HidD_SetFeature(handle, report.as_ptr(), 65) };
        unsafe { CloseHandle(handle) };

        if ok == 0 {
            Err("HidD_SetFeature failed".to_string())
        } else {
            Ok(())
        }
    }

    pub fn read_response(&self) -> Result<[u8; 64], String> {
        let wide_path: Vec<u16> = self.device_path.encode_utf16().chain(std::iter::once(0)).collect();
        let handle = unsafe {
            CreateFileW(
                wide_path.as_ptr(),
                0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null_mut(),
                OPEN_EXISTING,
                0,
                std::ptr::null_mut(),
            )
        };

        if handle == INVALID_HANDLE || handle.is_null() {
            return Err("Failed to open HID handle to MK870".to_string());
        }

        let mut report = [0u8; 65];
        report[0] = 0x00; // Report ID 0
        let ok = unsafe { HidD_GetFeature(handle, report.as_mut_ptr(), 65) };
        unsafe { CloseHandle(handle) };

        if ok == 0 {
            Err("HidD_GetFeature failed".to_string())
        } else {
            let mut out = [0u8; 64];
            out.copy_from_slice(&report[1..65]);
            Ok(out)
        }
    }

    pub fn query(&self, cmd: &[u8; 64], cs_type: CheckSumType) -> Result<[u8; 64], String> {
        self.send_command(cmd, cs_type)?;
        std::thread::sleep(Duration::from_millis(15));
        self.read_response()
    }

    pub fn get_keyboard_info(&self) -> Result<KeyboardInfo, String> {
        let mut cmd = [0u8; 64];
        cmd[0] = FEA_CMD_GET_INFOR;
        let raw = self.query(&cmd, CheckSumType::Bit7).unwrap_or([0u8; 64]);

        let fw_date = if raw[27] == 0x22 {
            format!("20{:02X}-{:02X}-{:02X}", raw[27], raw[28], raw[29])
        } else if raw[28] == 0x22 {
            format!("20{:02X}-{:02X}-{:02X}", raw[28], raw[29], raw[30])
        } else {
            "v2.0 (Standard)".to_string()
        };

        // Battery percentage: guard against static RF buffer reading (byte 4 = 2)
        let battery = if self.is_wireless {
            match self.get_battery() {
                Ok(bat) if bat > 5 && bat <= 100 => Some(bat),
                _ => None,
            }
        } else {
            None
        };

        Ok(KeyboardInfo {
            name: "FL-ESPORTS MK870".to_string(),
            vid: 0x05AC,
            pid: 0x024F,
            is_wireless: self.is_wireless,
            path: self.device_path.clone(),
            firmware_date: fw_date,
            battery_percent: battery,
        })
    }

    pub fn set_rgb(
        &self,
        mode: LightMode,
        brightness: u8,
        speed: u8,
        color: [u8; 3],
        dazzle: bool,
    ) -> Result<(), String> {
        let packet = build_set_led(mode, brightness, speed, color, dazzle);
        self.send_command(&packet, CheckSumType::Bit7)
    }

    pub fn set_per_key_rgb(
        &self,
        key_colors: &[[u8; 3]],
        layer: u8,
        brightness: u8,
    ) -> Result<(), String> {
        let packets = crate::protocol::packet::build_per_key_packets(key_colors, layer);
        for p in &packets {
            self.send_command(p, CheckSumType::None)?;
            std::thread::sleep(std::time::Duration::from_millis(15));
        }
        let mode_cmd = crate::protocol::packet::build_set_user_pic_mode(layer, brightness);
        self.send_command(&mode_cmd, CheckSumType::Bit7)?;
        Ok(())
    }

    pub fn get_rgb(&self) -> Result<LedStatus, String> {
        let mut req = [0u8; 64];
        req[0] = FEA_CMD_GET_LEDPARAM;
        let data = self.query(&req, CheckSumType::Bit7)?;
        Ok(parse_led_status(&data))
    }

    pub fn set_polling_rate(&self, rate: PollingRate) -> Result<(), String> {
        let packet = build_set_report_rate(rate);
        self.send_command(&packet, CheckSumType::Bit7)
    }

    pub fn get_polling_rate(&self) -> Result<PollingRate, String> {
        let mut req = [0u8; 64];
        req[0] = FEA_CMD_GET_REPORT;
        let data = self.query(&req, CheckSumType::Bit7)?;
        Ok(parse_report_rate(&data))
    }

    pub fn set_sleep(
        &self,
        time_bt: u16,
        time_24: u16,
        deep_bt: u16,
        deep_24: u16,
    ) -> Result<(), String> {
        let packet = build_set_sleep(time_bt, time_24, deep_bt, deep_24);
        self.send_command(&packet, CheckSumType::Bit7)
    }

    pub fn get_sleep(&self) -> Result<SleepStatus, String> {
        let mut req = [0u8; 64];
        req[0] = FEA_CMD_GET_SLEEPTIME;
        let data = self.query(&req, CheckSumType::Bit7)?;
        Ok(parse_sleep_status(&data))
    }

    pub fn set_debounce(&self, debounce_ms: u8) -> Result<(), String> {
        let packet = build_set_debounce(debounce_ms);
        self.send_command(&packet, CheckSumType::Bit7)
    }

    pub fn sync_clock(&self) -> Result<(), String> {
        let now = chrono::Local::now();
        use chrono::Datelike;
        use chrono::Timelike;

        let packet = build_set_clock(
            now.year() as u16,
            now.month() as u8,
            now.day() as u8,
            now.hour() as u8,
            now.minute() as u8,
            now.second() as u8,
        );
        self.send_command(&packet, CheckSumType::Bit7)
    }

    pub fn factory_reset(&self) -> Result<(), String> {
        let packet = build_reset();
        self.send_command(&packet, CheckSumType::Bit7)
    }

    pub fn get_battery(&self) -> Result<u8, String> {
        let mut req = [0u8; 64];
        req[0] = FEA_CMD_GET_BATTERY;
        let data = self.query(&req, CheckSumType::Bit7)?;
        let bat = if data[4] > 5 && data[4] <= 100 {
            data[4]
        } else if data[5] > 5 && data[5] <= 100 {
            data[5]
        } else {
            100
        };
        Ok(bat)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_discovery() {
        match KeyboardBackend::find_hid_device_native() {
            Ok((path, is_wireless)) => {
                println!("Found device: {} (wireless={})", path, is_wireless);
            }
            Err(e) => {
                println!("Device discovery error: {}", e);
            }
        }
    }
}
