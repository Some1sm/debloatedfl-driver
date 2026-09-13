# debloatedfl-driver (FL-ESPORTS MK870 Rust Driver)

A clean, high-performance, 100% offline, bloatware-free driver and control suite for the **FL-ESPORTS MK870** mechanical keyboard.

---

## 🛡️ Spyware & Bloatware Removed
The original vendor installer (`FL_setup_2.0.18(WIN20231121)(1).exe`, ~80MB installer, ~250MB installed) bundled:
- **`node-machine-id`**: Motherboard/CPU GUID hardware fingerprinting.
- **`universal-analytics`**: Google Analytics user behavior telemetry.
- **Outbound Sockets**: Persistent network calls to `https://api.rongyuan.tech:3814/v1`.
- **Chromium / Electron Runtime**: Heavy multi-process browser shell consuming hundreds of megabytes of RAM.

### The Rust Replacement:
- **Zero outbound internet connections**: 100% offline operation.
- **Zero telemetry or fingerprinting**: Respects your privacy completely.
- **Instant startup**: Consumes <15MB of RAM instead of ~250MB.
- **Single portable package**: No installer needed, no registry clutter.

---

## 🚀 Quick Start

### 1. Launch Native Desktop GUI
Simply double-click `fl-driver-rs.exe` or run:
```powershell
.\fl-driver-rs.exe gui
```
Features available in GUI:
- **RGB Lighting Studio**: 16 hardware lighting modes, color picker with quick palette presets, brightness & animation speed sliders, multicolor/rainbow toggle.
- **Performance & Sync**: USB Polling Rate selector (125Hz, 250Hz, 500Hz, 1000Hz), Switch Debounce filter, Wireless Sleep timeout sliders, OLED Clock synchronization, and Factory Reset.
- **Device Info & Privacy**: Hardware connection details, battery percentage, firmware date, and privacy guarantees.

---

### 2. Command Line Interface (CLI)

#### Check Device Info & Battery
```powershell
.\fl-driver-rs.exe info
```

#### Control RGB Lighting
```powershell
# Set static color (HEX or name)
.\fl-driver-rs.exe rgb --mode static --color FF0055
.\fl-driver-rs.exe rgb --mode static --color cyan

# Rainbow wave animation
.\fl-driver-rs.exe rgb --mode wave --rainbow --speed 3 --brightness 4

# Breathing effect with custom color
.\fl-driver-rs.exe rgb --mode breath --color 00FF88

# Turn all LEDs off
.\fl-driver-rs.exe rgb --mode off
```

#### Adjust Polling Rate
```powershell
.\fl-driver-rs.exe rate 1000   # 1000 Hz (1ms)
.\fl-driver-rs.exe rate 500    # 500 Hz (2ms)
```

#### Adjust Switch Debounce
```powershell
.\fl-driver-rs.exe debounce 4  # 4 ms
```

#### Sync OLED / LCD Clock (for models with display)
```powershell
.\fl-driver-rs.exe clock
```

---

## 📁 File Structure
- `fl-driver-rs.exe`: Rust native dual-mode CLI + GUI application.
- `bin/iot_driver.exe`: Local low-level Windows HID communication helper.
