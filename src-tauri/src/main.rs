#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod discord;

use std::{sync::Arc, thread};

use tauri::App;

struct Flexcord {
    pub app: App,
}

impl Flexcord {
    fn new() -> Self {
        let app = tauri::Builder::default()
            .build(tauri::generate_context!())
            .expect("error while running tauri application");
        Self { app }
    }
}

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

fn main() {
        tauri::Builder::default()
    // This is where you pass in your commands
    .invoke_handler(tauri::generate_handler![greet])
    .run(tauri::generate_context!())
    .expect("failed to run app");
}
