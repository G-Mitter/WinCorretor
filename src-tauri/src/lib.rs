// src-tauri/src/lib.rs
//
// Ponto de entrada do app: só monta os plugins, o estado e os comandos.
// Cada responsabilidade vive no seu próprio módulo.

mod capture;
mod commands;
mod error;
mod hotkey;
mod llm;
mod prompts;
mod state;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(hotkey::plugin())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![commands::process_clipboard_text])
        .setup(|app| {
            hotkey::register_default(app.handle())?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("erro ao iniciar o WinCorretor");
}
