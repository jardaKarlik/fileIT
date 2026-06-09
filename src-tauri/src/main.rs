// src-tauri/src/main.rs
//
// FileIT desktop application entry point.
// The #![windows_subsystem = "windows"] attribute prevents a console window
// from appearing on Windows when the app is launched.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    fileit_lib::run();
}
