#![windows_subsystem = "windows"]

mod protocol;
mod backend;
mod cli;
mod gui;
pub mod storage;

use clap::Parser;
use cli::{Cli, Commands};

#[cfg(windows)]
extern "system" {
    fn AttachConsole(dw_process_id: u32) -> i32;
}

#[cfg(windows)]
const ATTACH_PARENT_PROCESS: u32 = 0xFFFFFFFF;

fn main() {
    let raw_args: Vec<String> = std::env::args().collect();
    let is_cli = raw_args.len() > 1 && raw_args[1] != "gui";

    if is_cli {
        #[cfg(windows)]
        unsafe {
            AttachConsole(ATTACH_PARENT_PROCESS);
        }
    }

    let args = Cli::parse();

    match args.command {
        Some(Commands::Gui) | None => {
            if let Err(e) = gui::run_gui() {
                #[cfg(windows)]
                unsafe {
                    AttachConsole(ATTACH_PARENT_PROCESS);
                }
                eprintln!("[!] GUI Error: {}", e);
            }
        }
        Some(cmd) => {
            if let Err(e) = cli::run_cli_command(cmd) {
                eprintln!("[!] Error: {}", e);
                std::process::exit(1);
            }
        }
    }
}
