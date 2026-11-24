// Hide the console window in release builds while keeping stderr visible during development.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod engine;
mod error;
mod ipc;
mod layout;
mod shell;
mod tabs;
mod window;

use std::process::ExitCode;

fn main() -> ExitCode {
    if let Err(err) = app::run() {
        eprintln!("error: {err}");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
