// src-tauri/src/lib.rs

mod corrector;

use corrector::{CorrectorService, TextStyle};
use tauri::{AppHandle, Manager};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

#[tauri::command]
async fn process_clipboard_text(app: AppHandle, style: TextStyle, api_key: String) -> Result<String, String> {
    // 1. Lê o texto atual da área de transferência
    let original_text = app.clipboard().read_text().map_err(|e| e.to_string())?;
    
    if original_text.trim().is_empty() {
        return Err("O clipboard está vazio ou não contém texto.".into());
    }

    // 2. Instancia o serviço e processa o texto
    let service = CorrectorService::new();
    let corrected_text = service.process_text(&original_text, &style, &api_key).await?;

    // 3. Escreve o texto corrigido de volta na área de transferência
    app.clipboard().write_text(corrected_text.clone()).map_err(|e| e.to_string())?;

    Ok(corrected_text)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        // Configuração do Atalho Global
        .plugin(tauri_plugin_global_shortcut::Builder::new().with_handler(|app, shortcut, event| {
            if event.state == ShortcutState::Pressed {
                // Quando o usuário pressionar o atalho, capturamos o evento aqui.
                // Em uma implementação completa, podemos disparar um evento para o Frontend abrir o popup.
                println!("Atalho global acionado! {:?}", shortcut);
                let _ = app.emit("shortcut-pressed", ());
            }
        }).build())
        .invoke_handler(tauri::generate_handler![process_clipboard_text])
        .setup(|app| {
            // Registra um atalho global (Ex: Ctrl+Shift+C ou Cmd+Shift+C)
            let ctrl_shift_c = Shortcut::parse("Ctrl+Shift+C").unwrap();
            app.global_shortcut().register(ctrl_shift_c).unwrap();
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}