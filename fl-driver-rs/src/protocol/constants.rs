use serde::{Deserialize, Serialize};

// HID Feature Command Opcodes
pub const FEA_CMD_SET_REV: u8 = 0x00;
pub const FEA_CMD_GET_REV: u8 = 0x80;
pub const FEA_CMD_SET_WIRELESS_SYNC: u8 = 0x01;
pub const FEA_CMD_SET_RESERT: u8 = 0x02;
pub const FEA_CMD_SET_REPORT: u8 = 0x04;
pub const FEA_CMD_GET_REPORT: u8 = 0x84;
pub const FEA_CMD_SET_PROFILE: u8 = 0x05;
pub const FEA_CMD_GET_PROFILE: u8 = 0x85;
pub const FEA_CMD_SET_KBOPTION: u8 = 0x06;
pub const FEA_CMD_GET_KBOPTION: u8 = 0x86;
pub const FEA_CMD_SET_LEDPARAM: u8 = 0x07;
pub const FEA_CMD_GET_LEDPARAM: u8 = 0x87;
pub const FEA_CMD_SET_SLEDPARAM: u8 = 0x08;
pub const FEA_CMD_GET_SLEDPARAM: u8 = 0x88;
pub const FEA_CMD_SET_KEYMATRIX: u8 = 0x09;
pub const FEA_CMD_GET_KEYMATRIX: u8 = 0x89;
pub const FEA_CMD_GET_BATTERY: u8 = 0x83;
pub const FEA_CMD_GET_INFOR: u8 = 0x8F;
pub const FEA_CMD_SET_FN: u8 = 0x10;
pub const FEA_CMD_GET_FN: u8 = 0x90;
pub const FEA_CMD_SET_DEBOUNCE: u8 = 0x11;
pub const FEA_CMD_GET_DEBOUNCE: u8 = 0x91;
pub const FEA_CMD_SET_SLEEPTIME: u8 = 0x12;
pub const FEA_CMD_GET_SLEEPTIME: u8 = 0x92;
pub const FEA_CMD_SET_USERPIC: u8 = 0x0C;
pub const FEA_CMD_GET_USERPIC: u8 = 0x8C;
pub const FEA_CMD_SET_OLEDCLOCK: u8 = 0x28;

// 2.4G receiver relay opcodes. A relaying dongle answers these itself (no checksum) and
// forwards every other feature report to the keyboard over RF.
pub const RF_CMD_GET_STATUS: u8 = 0xFB;
pub const RF_CMD_FETCH_REPLY: u8 = 0xFC;
pub const RF_CMD_PROBE: u8 = 0xFD;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum CheckSumType {
    Bit7 = 0,
    Bit8 = 1,
    None = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LightMode {
    Off = 0,
    AlwaysOn = 1,
    Breath = 2,
    Neon = 3,
    Wave = 4,
    Ripple = 5,
    Raindrop = 6,
    Snake = 7,
    PressAction = 8,
    Converage = 9,
    SineWave = 10,
    Kaleidoscope = 11,
    LineWave = 12,
    CustomStatic = 13,
    Laser = 14,
    Circle = 15,
    Dazzing = 16,
}

impl LightMode {
    pub const ALL: &'static [LightMode] = &[
        LightMode::AlwaysOn,
        LightMode::Breath,
        LightMode::Wave,
        LightMode::Neon,
        LightMode::Ripple,
        LightMode::Raindrop,
        LightMode::Snake,
        LightMode::PressAction,
        LightMode::Converage,
        LightMode::SineWave,
        LightMode::Kaleidoscope,
        LightMode::LineWave,
        LightMode::Laser,
        LightMode::Circle,
        LightMode::Dazzing,
        LightMode::Off,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            LightMode::Off => "Off",
            LightMode::AlwaysOn => "Static (Always On)",
            LightMode::Breath => "Breathing",
            LightMode::Neon => "Neon / Color Cycle",
            LightMode::Wave => "Rainbow Wave",
            LightMode::Ripple => "Ripple",
            LightMode::Raindrop => "Raindrop",
            LightMode::Snake => "Snake",
            LightMode::PressAction => "Reactive (Keypress)",
            LightMode::Converage => "Convergence",
            LightMode::SineWave => "Sine Wave",
            LightMode::Kaleidoscope => "Kaleidoscope",
            LightMode::LineWave => "Line Wave",
            LightMode::CustomStatic => "Custom Static (Per-Key)",
            LightMode::Laser => "Laser",
            LightMode::Circle => "Circle",
            LightMode::Dazzing => "Dazzling",
        }
    }

    pub fn from_u8(val: u8) -> Self {
        match val {
            0 => LightMode::Off,
            1 => LightMode::AlwaysOn,
            2 => LightMode::Breath,
            3 => LightMode::Neon,
            4 => LightMode::Wave,
            5 => LightMode::Ripple,
            6 => LightMode::Raindrop,
            7 => LightMode::Snake,
            8 => LightMode::PressAction,
            9 => LightMode::Converage,
            10 => LightMode::SineWave,
            11 => LightMode::Kaleidoscope,
            12 => LightMode::LineWave,
            13 => LightMode::CustomStatic,
            14 => LightMode::Laser,
            15 => LightMode::Circle,
            16 => LightMode::Dazzing,
            _ => LightMode::AlwaysOn,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PollingRate {
    Hz1000 = 1,
    Hz500 = 2,
    Hz250 = 4,
    Hz125 = 8,
}

impl PollingRate {
    pub fn to_hz(&self) -> u16 {
        match self {
            PollingRate::Hz1000 => 1000,
            PollingRate::Hz500 => 500,
            PollingRate::Hz250 => 250,
            PollingRate::Hz125 => 125,
        }
    }

    pub fn from_hz(hz: u16) -> Option<Self> {
        match hz {
            1000 => Some(PollingRate::Hz1000),
            500 => Some(PollingRate::Hz500),
            250 => Some(PollingRate::Hz250),
            125 => Some(PollingRate::Hz125),
            _ => None,
        }
    }

    pub fn from_byte(b: u8) -> Self {
        match b {
            1 => PollingRate::Hz1000,
            2 => PollingRate::Hz500,
            4 => PollingRate::Hz250,
            8 => PollingRate::Hz125,
            _ => PollingRate::Hz1000,
        }
    }
}

/// Hardware matrix index mapping for the 87 keys of the FL-ESPORTS MK870 (yzw_k210).
pub const KEY_ID_TO_MATRIX: [usize; 87] = [
    0, // [ 0] Esc
    12, // [ 1] F1
    18, // [ 2] F2
    24, // [ 3] F3
    30, // [ 4] F4
    36, // [ 5] F5
    42, // [ 6] F6
    48, // [ 7] F7
    54, // [ 8] F8
    60, // [ 9] F9
    66, // [10] F10
    72, // [11] F11
    78, // [12] F12
    84, // [13] Print Screen
    90, // [14] Scroll Lock
    96, // [15] Pause
    1,  // [16] `~
    7,  // [17] 1
    13, // [18] 2
    19, // [19] 3
    25, // [20] 4
    31, // [21] 5
    37, // [22] 6
    43, // [23] 7
    49, // [24] 8
    55, // [25] 9
    61, // [26] 0
    67, // [27] -
    73, // [28] =
    79, // [29] Backspace
    85, // [30] Insert
    91, // [31] Home
    97, // [32] Page Up
    2,  // [33] Tab
    8,  // [34] Q
    14, // [35] W
    20, // [36] E
    26, // [37] R
    32, // [38] T
    38, // [39] Y
    44, // [40] U
    50, // [41] I
    56, // [42] O
    62, // [43] P
    68, // [44] [
    74, // [45] ]
    80, // [46] \
    86, // [47] Delete
    92, // [48] End
    98, // [49] Page Down
    3,  // [50] Caps Lock
    9,  // [51] A
    15, // [52] S
    21, // [53] D
    27, // [54] F
    33, // [55] G
    39, // [56] H
    45, // [57] J
    51, // [58] K
    57, // [59] L
    63, // [60] ;
    69, // [61] '
    81, // [62] Enter
    4,  // [63] Shift
    16, // [64] Z
    22, // [65] X
    28, // [66] C
    34, // [67] V
    40, // [68] B
    46, // [69] N
    52, // [70] M
    58, // [71] ,
    64, // [72] .
    70, // [73] /
    76, // [74] Right Shift
    82, // [75] ↑
    5,  // [76] Ctrl
    17, // [77] Win
    23, // [78] Alt
    29, // [79] Space
    35, // [80] Right Alt
    0,  // [81] Fn
    71, // [82] App
    47, // [83] Right Ctrl
    53, // [84] ←
    59, // [85] ↓
    65, // [86] →
];
