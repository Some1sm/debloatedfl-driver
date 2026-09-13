use super::constants::*;

pub fn calculate_checksum(buf: &mut [u8; 64], cs_type: CheckSumType) {
    match cs_type {
        CheckSumType::Bit7 => {
            let mut sum: u8 = 0;
            for i in 0..63 {
                sum = sum.wrapping_add(buf[i]);
            }
            buf[63] = ((sum ^ 0xFF).wrapping_add(1)) as u8;
        }
        CheckSumType::Bit8 => {
            let mut sum: u8 = 0;
            for i in 0..63 {
                sum = sum.wrapping_add(buf[i]);
            }
            buf[63] = sum;
        }
        CheckSumType::None => {}
    }
}

pub fn build_set_led(
    mode: LightMode,
    brightness: u8, // 0..=4
    speed: u8,      // 0..=5
    color: [u8; 3], // [R, G, B]
    dazzle: bool,   // rainbow / multi-color
) -> [u8; 64] {
    let mut buf = [0u8; 64];
    buf[0] = FEA_CMD_SET_LEDPARAM; // 0x07
    buf[1] = mode as u8;
    
    // MAXSPEED is 4 in driver, speed is inverted
    let speed_clamped = speed.min(4);
    buf[2] = 4 - speed_clamped;
    
    buf[3] = brightness.min(4);
    
    // 7 = normal single color, 6 = dazzle / rainbow
    buf[4] = if dazzle { 6 } else { 7 };
    
    buf[5] = color[0];
    buf[6] = color[1];
    buf[7] = color[2];
    buf[8] = 0;
    
    buf
}

pub fn build_set_report_rate(rate: PollingRate) -> [u8; 64] {
    let mut buf = [0u8; 64];
    buf[0] = FEA_CMD_SET_REPORT; // 0x04
    buf[1] = 0; // profile
    buf[2] = rate as u8;
    buf
}

pub fn build_set_sleep(time_bt: u16, time_24: u16, deep_bt: u16, deep_24: u16) -> [u8; 64] {
    let mut buf = [0u8; 64];
    buf[0] = FEA_CMD_SET_SLEEPTIME; // 0x12
    buf[8] = (time_bt & 0xFF) as u8;
    buf[9] = ((time_bt >> 8) & 0xFF) as u8;
    buf[10] = (time_24 & 0xFF) as u8;
    buf[11] = ((time_24 >> 8) & 0xFF) as u8;
    buf[12] = (deep_bt & 0xFF) as u8;
    buf[13] = ((deep_bt >> 8) & 0xFF) as u8;
    buf[14] = (deep_24 & 0xFF) as u8;
    buf[15] = ((deep_24 >> 8) & 0xFF) as u8;
    buf
}

pub fn build_set_debounce(debounce_ms: u8) -> [u8; 64] {
    let mut buf = [0u8; 64];
    buf[0] = FEA_CMD_SET_DEBOUNCE; // 0x11
    buf[1] = 0;
    buf[2] = debounce_ms;
    buf
}

pub fn build_set_clock(year: u16, month: u8, day: u8, hour: u8, min: u8, sec: u8) -> [u8; 64] {
    let mut buf = [0u8; 64];
    buf[0] = FEA_CMD_SET_OLEDCLOCK; // 0x28
    buf[8] = ((year >> 8) & 0xFF) as u8;
    buf[9] = (year & 0xFF) as u8;
    buf[10] = month;
    buf[11] = day;
    buf[12] = hour;
    buf[13] = min;
    buf[14] = sec;
    buf
}

pub fn build_reset() -> [u8; 64] {
    let mut buf = [0u8; 64];
    buf[0] = FEA_CMD_SET_RESERT; // 0x02
    buf
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LedStatus {
    pub mode: LightMode,
    pub brightness: u8,
    pub speed: u8,
    pub color: [u8; 3],
    pub dazzle: bool,
}

pub fn parse_led_status(data: &[u8; 64]) -> LedStatus {
    let mode = LightMode::from_u8(data[1]);
    let speed_raw = data[2];
    let speed = if speed_raw <= 4 { 4 - speed_raw } else { 0 };
    let brightness = data[3].min(4);
    let color_flag = data[4];
    let dazzle = color_flag == 6;
    let color = [data[5], data[6], data[7]];
    
    LedStatus {
        mode,
        brightness,
        speed,
        color,
        dazzle,
    }
}

pub fn parse_report_rate(data: &[u8; 64]) -> PollingRate {
    PollingRate::from_byte(data[2])
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SleepStatus {
    pub time_bt: u16,
    pub deep_bt: u16,
    pub time_24: u16,
    pub deep_24: u16,
}

pub fn parse_sleep_status(data: &[u8; 64]) -> SleepStatus {
    let time_bt = (data[1] as u16) | ((data[2] as u16) << 8);
    let deep_bt = (data[3] as u16) | ((data[4] as u16) << 8);
    let time_24 = (data[5] as u16) | ((data[6] as u16) << 8);
    let deep_24 = (data[7] as u16) | ((data[8] as u16) << 8);
    SleepStatus {
        time_bt,
        deep_bt,
        time_24,
        deep_24,
    }
}

pub fn build_per_key_packets(key_colors: &[[u8; 3]], layer: u8) -> Vec<[u8; 64]> {
    // 128 matrix slots * 3 = 384 bytes, padded to 392 (7 * 56)
    let mut matrix_rgb = vec![0u8; 392];

    for (key_id, &color) in key_colors.iter().enumerate() {
        if key_id < KEY_ID_TO_MATRIX.len() {
            let matrix_idx = KEY_ID_TO_MATRIX[key_id];
            let base = matrix_idx * 3;
            if base + 2 < matrix_rgb.len() {
                matrix_rgb[base] = color[0];
                matrix_rgb[base + 1] = color[1];
                matrix_rgb[base + 2] = color[2];
            }
        }
    }

    let mut packets = Vec::with_capacity(7);
    let layer_idx = layer.saturating_sub(1);

    for n in 0..7 {
        let mut buf = [0u8; 64];
        buf[0] = FEA_CMD_SET_USERPIC; // 0x0C (12)
        buf[1] = layer_idx;
        buf[2] = 0x80; // 384 LE byte 0 (64 * picsCount 6)
        buf[3] = 0x01; // 384 LE byte 1
        buf[4] = n as u8;
        buf[5] = 0;
        buf[6] = 0;
        buf[7] = 0;

        let start = n * 56;
        let end = (start + 56).min(matrix_rgb.len());
        if start < matrix_rgb.len() {
            buf[8..8 + (end - start)].copy_from_slice(&matrix_rgb[start..end]);
        }
        packets.push(buf);
    }

    packets
}

pub fn build_set_user_pic_mode(layer: u8, brightness: u8) -> [u8; 64] {
    let mut buf = [0u8; 64];
    buf[0] = FEA_CMD_SET_LEDPARAM; // 0x07
    buf[1] = 13;                  // LightUserPicture (Mode 13)
    buf[2] = 0;                   // speed
    buf[3] = brightness.min(4);   // brightness
    buf[4] = (layer.saturating_sub(1) * 16).min(64); // 0 for Layer 1, 16 for Layer 2, 32 for Layer 3
    buf[5] = 0;
    buf[6] = 200;
    buf[7] = 200;
    buf
}

