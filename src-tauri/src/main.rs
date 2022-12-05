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

    thread::Builder::new()
        .name("Discord Connection".to_string())
        .spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(discord::start_client());
        })
        .unwrap();

        tauri::Builder::default()
    // This is where you pass in your commands
    .invoke_handler(tauri::generate_handler![greet, discord::getGuildById])
    .run(tauri::generate_context!())
    .expect("failed to run app");
}
