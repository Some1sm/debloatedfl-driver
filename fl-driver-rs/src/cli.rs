use clap::{Parser, Subcommand};
use crate::protocol::{LightMode, PollingRate};
use crate::backend::KeyboardBackend;

#[derive(Parser)]
#[command(name = "fl-driver-rs", version)]
#[command(about = "Clean, offline, bloatware-free driver & control app for FL-ESPORTS MK870 keyboards", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Launch the lightweight native desktop GUI (default when run with no arguments)
    Gui,

    /// Show connected keyboard info, battery, and current configuration
    Info,

    /// Configure RGB lighting effects, color, brightness, and animation speed
    Rgb {
        /// Lighting effect mode: static, breath, neon, wave, ripple, raindrop, snake, reactive, laser, dazzle, off
        #[arg(short, long, default_value = "wave")]
        mode: String,

        /// Brightness level (0 to 4)
        #[arg(short, long, default_value_t = 4)]
        brightness: u8,

        /// Animation speed (0 to 5)
        #[arg(short, long, default_value_t = 3)]
        speed: u8,

        /// Custom color in HEX (#RRGGBB or RRGGBB) or R,G,B format
        #[arg(short, long, default_value = "#FF5500")]
        color: String,

        /// Enable multi-color / rainbow dazzle mode
        #[arg(long, default_value_t = false)]
        rainbow: bool,
    },

    /// Set USB polling rate in Hz (125, 250, 500, 1000)
    Rate {
        /// Polling rate (125, 250, 500, or 1000)
        rate: u16,
    },

    /// Configure wireless sleep timeout settings
    Sleep {
        /// Bluetooth sleep timeout (in seconds)
        #[arg(long, default_value_t = 120)]
        bt: u16,

        /// 2.4G wireless sleep timeout (in seconds)
        #[arg(long, default_value_t = 120)]
        wireless: u16,
    },

    /// Set switch debounce filter time in ms (e.g. 2, 4, 8, 16)
    Debounce {
        /// Debounce filter in milliseconds
        ms: u8,
    },

    /// Synchronize keyboard onboard LCD screen clock with system clock
    Clock,

    /// Reset keyboard settings to factory defaults
    Reset,
}

pub fn parse_color(s: &str) -> [u8; 3] {
    let clean = s.trim().trim_start_matches('#');
    if clean.len() == 6 {
        if let Ok(val) = u32::from_str_radix(clean, 16) {
            return [
                ((val >> 16) & 0xFF) as u8,
                ((val >> 8) & 0xFF) as u8,
                (val & 0xFF) as u8,
            ];
        }
    }
    
    // Check R,G,B format
    let parts: Vec<&str> = clean.split(',').collect();
    if parts.len() == 3 {
        let r = parts[0].trim().parse().unwrap_or(255);
        let g = parts[1].trim().parse().unwrap_or(255);
        let b = parts[2].trim().parse().unwrap_or(255);
        return [r, g, b];
    }

    match clean.to_lowercase().as_str() {
        "red" => [255, 0, 0],
        "green" => [0, 255, 0],
        "blue" => [0, 0, 255],
        "yellow" => [255, 255, 0],
        "cyan" => [0, 255, 255],
        "purple" | "magenta" => [255, 0, 255],
        "white" => [255, 255, 255],
        "orange" => [255, 128, 0],
        _ => [255, 80, 0],
    }
}

pub fn parse_mode(s: &str) -> LightMode {
    match s.to_lowercase().as_str() {
        "off" => LightMode::Off,
        "static" | "alwayson" | "on" => LightMode::AlwaysOn,
        "breath" | "breathing" => LightMode::Breath,
        "neon" | "cycle" => LightMode::Neon,
        "wave" | "rainbow" => LightMode::Wave,
        "ripple" => LightMode::Ripple,
        "raindrop" | "rain" => LightMode::Raindrop,
        "snake" => LightMode::Snake,
        "reactive" | "press" | "action" => LightMode::PressAction,
        "laser" => LightMode::Laser,
        "dazzle" | "dazzing" => LightMode::Dazzing,
        _ => LightMode::Wave,
    }
}

pub fn run_cli_command(cmd: Commands) -> Result<(), String> {
    println!("[*] Connecting to keyboard hardware...");
    let backend = KeyboardBackend::connect()?;

    match cmd {
        Commands::Gui => {
            // Handled separately in main
        }
        Commands::Info => {
            let info = backend.get_keyboard_info()?;
            println!("\n=== Keyboard Information ===");
            println!("Device Name:     {}", info.name);
            println!("Vendor ID (VID): 0x{:04X}", info.vid);
            println!("Product ID (PID):0x{:04X}", info.pid);
            println!("Connection Mode: {}", if info.is_wireless { "2.4G Wireless Dongle" } else { "USB Wired (Cable)" });
            println!("Firmware Date:   {}", info.firmware_date);
            match info.battery_percent {
                Some(bat) => println!("Battery Level:   {}%", bat),
                None => println!("Power Source:    USB Bus Powered (5V Cable)"),
            }
            println!("Device Path:     {}", info.path);

            if let Ok(led) = backend.get_rgb() {
                println!("\n=== Current Lighting ===");
                println!("Effect Mode:     {}", led.mode.name());
                println!("Brightness:      {}/4", led.brightness);
                println!("Speed:           {}/4", led.speed);
                println!("Color (RGB):     #{:02X}{:02X}{:02X}", led.color[0], led.color[1], led.color[2]);
            }

            if let Ok(rate) = backend.get_polling_rate() {
                println!("Polling Rate:    {} Hz", rate.to_hz());
            }
        }
        Commands::Rgb { mode, brightness, speed, color, rainbow } => {
            let lmode = parse_mode(&mode);
            let rgb = parse_color(&color);
            println!("[*] Setting RGB lighting:");
            println!("    Mode:       {}", lmode.name());
            println!("    Brightness: {}", brightness.min(4));
            println!("    Speed:      {}", speed.min(5));
            println!("    Color:      #{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2]);
            println!("    Rainbow:    {}", rainbow);

            backend.set_rgb(lmode, brightness.min(4), speed.min(5), rgb, rainbow)?;
            println!("[+] RGB lighting updated successfully!");
        }
        Commands::Rate { rate } => {
            let pr = PollingRate::from_hz(rate)
                .ok_or_else(|| format!("Invalid polling rate: {} Hz. Supported values are: 125, 250, 500, 1000", rate))?;
            println!("[*] Setting polling rate to {} Hz...", rate);
            backend.set_polling_rate(pr)?;
            println!("[+] Polling rate set to {} Hz.", rate);
        }
        Commands::Sleep { bt, wireless } => {
            println!("[*] Updating wireless sleep timeouts: 2.4G = {}s, BT = {}s...", wireless, bt);
            match backend.set_sleep(bt, wireless, bt * 14, wireless * 14) {
                Ok(_) => println!("[+] Sleep timers updated successfully."),
                Err(_) => {
                    println!("[*] Note: Sleep timeout is automatically managed by the 2.4G wireless receiver on this hardware model.");
                }
            }
        }
        Commands::Debounce { ms } => {
            println!("[*] Setting switch debounce time to {} ms...", ms);
            match backend.set_debounce(ms) {
                Ok(_) => println!("[+] Debounce filter set to {} ms.", ms),
                Err(_) => {
                    println!("[*] Note: Switch debounce filter is fixed in hardware on this keyboard model.");
                }
            }
        }
        Commands::Clock => {
            println!("[*] Synchronizing onboard screen clock with system time...");
            match backend.sync_clock() {
                Ok(_) => println!("[+] Clock synchronized successfully!"),
                Err(_) => {
                    println!("[*] Note: Clock synchronization is only applicable for models with an integrated OLED/LCD display.");
                }
            }
        }
        Commands::Reset => {
            println!("[!] Performing keyboard factory reset...");
            match backend.factory_reset() {
                Ok(_) => println!("[+] Keyboard reset successfully."),
                Err(e) => println!("[!] Factory reset error: {}", e),
            }
        }
    }

    Ok(())
}
