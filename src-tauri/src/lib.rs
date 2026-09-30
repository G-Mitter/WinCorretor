// src-tauri/src/lib.rs
//
// Ponto de entrada do app: só monta os plugins, o estado e os comandos.
// Cada responsabilidade vive no seu próprio módulo.

mod autostart;
mod capture;
mod commands;
mod crashlog;
mod error;
mod flow;
mod hotkey;
mod llm;
mod notify;
mod popup;
mod prompts;
mod secrets;
mod settings;
mod state;
mod tray;
#[cfg(windows)]
mod win32;

use state::AppState;
use tauri::{Manager, WindowEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    crashlog::install();

    // Em desenvolvimento, as chaves também podem vir do arquivo src-tauri/.env (fora do Git).
    // O normal é salvá-las pela tela de configurações, no cofre do Windows.
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
        .plugin(autostart::plugin())
        // O estado existe desde o início (com valores padrão). As janelas são
        // criadas antes do `setup` e seus eventos já podem precisar dele.
        .manage(AppState::new(settings::Settings::default(), None))
        .invoke_handler(tauri::generate_handler![
            commands::hotkey_error,
            commands::get_settings,
            commands::save_settings,
            commands::test_ai,
            commands::popup_rewrite,
            commands::popup_apply,
            commands::popup_copy,
            commands::popup_cancel
        ])
        .setup(|app| {
            // Preferências salvas (ou padrão na primeira execução).
            let path = app
                .path()
                .app_config_dir()
                .ok()
                .map(|dir| dir.join("settings.json"));
            let loaded = path.as_deref().map(settings::load).unwrap_or_default();
            let shortcut = loaded.shortcut.clone();
            app.state::<AppState>().configure(loaded, path);

            tray::create(app.handle())?;

            // Se o atalho já estiver em uso, o app abre mesmo assim e avisa.
            if let Err(err) = hotkey::register(app.handle(), &shortcut) {
                let message =
                    format!("{err} Abra Configurações pelo ícone da bandeja para trocar o atalho.");
                eprintln!("{message}");
                notify::show(app.handle(), &message);
                app.state::<AppState>().set_hotkey_error(Some(message));
            }

            welcome(app.handle(), &shortcut);
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

/// Aberto pelo usuário (não pelo Windows no login): avisa que o app está na
/// bandeja e, se ainda não há chave de IA, já abre as Configurações.
fn welcome(app: &tauri::AppHandle, shortcut: &str) {
    let by_autostart = std::env::args().any(|arg| arg == autostart::AUTOSTART_ARG);
    if by_autostart {
        return;
    }

    let has_key = [secrets::ApiKey::Groq, secrets::ApiKey::Gemini]
        .into_iter()
        .any(|key| secrets::resolve(key).0.is_some());

    if has_key {
        notify::show(
            app,
            &format!("WinCorretor está ativo na bandeja. Selecione um texto e aperte {shortcut}."),
        );
    } else {
        tray::show_main_window(app);
    }
}
