// Hide the console window in app binaries while keeping stdout/stderr visible for `cargo test`.
#![cfg_attr(not(test), windows_subsystem = "windows")]

mod adblock;
mod app;
mod engine;
mod error;
mod icon;
mod ipc;
mod layout;
mod omnibox;
mod onboarding;
mod settings;
mod shell;
mod suggest;
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
