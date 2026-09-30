// src-tauri/src/lib.rs
//
// Ponto de entrada do app: só monta os plugins, o estado e os comandos.
// Cada responsabilidade vive no seu próprio módulo.

mod capture;
mod commands;
mod error;
mod flow;
mod hotkey;
mod llm;
mod notify;
mod popup;
mod prompts;
mod state;
mod tray;
#[cfg(windows)]
mod win32;

use state::AppState;
use tauri::{Manager, WindowEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Em desenvolvimento, lê GEMINI_API_KEY do arquivo src-tauri/.env (fora do Git).
    let _ = dotenvy::dotenv();

    tauri::Builder::default()
        // Precisa ser o primeiro plugin: se o app já estiver aberto, a segunda
        // cópia só mostra a janela da primeira e fecha (evita atalho duplicado).
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tray::show_main_window(app);
        }))
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(hotkey::plugin())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::hotkey_error,
            commands::popup_rewrite,
            commands::popup_apply,
            commands::popup_copy,
            commands::popup_cancel
        ])
        .setup(|app| {
            tray::create(app.handle())?;

            // Se o atalho já estiver em uso, o app abre mesmo assim e avisa.
            if let Err(err) = hotkey::register_default(app.handle()) {
                let message = format!(
                    "O atalho {} já está em uso. Feche outras janelas do WinCorretor (ou o programa que usa esse atalho) e reinicie.",
                    hotkey::DEFAULT_SHORTCUT
                );
                eprintln!("{message} Detalhe: {err}");
                notify::show(app.handle(), &message);
                app.state::<AppState>().set_hotkey_error(message);
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            let is_popup = window.label() == popup::POPUP_LABEL;
            match event {
                // Fechar uma janela só a esconde: o app continua na bandeja.
                WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    if is_popup {
                        flow::cancel(window.app_handle(), false);
                    } else {
                        let _ = window.hide();
                    }
                }
                // Clicou fora do popup: fecha sem alterar nada.
                WindowEvent::Focused(false) if is_popup => {
                    flow::cancel(window.app_handle(), false);
                }
                _ => {}
            }
        })
        .run(tauri::generate_context!())
        .expect("erro ao iniciar o WinCorretor");
}
