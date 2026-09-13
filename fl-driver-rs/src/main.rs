#![windows_subsystem = "console"]

mod protocol;
mod backend;
mod cli;
mod gui;
pub mod storage;

use clap::Parser;
use cli::{Cli, Commands};

fn main() {
    let args = Cli::parse();

    match args.command {
        Some(Commands::Gui) | None => {
            if let Err(e) = gui::run_gui() {
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
