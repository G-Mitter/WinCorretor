// src-tauri/src/hotkey.rs
//
// Atalho global. Combinações evitadas por conflito:
// - Ctrl+Shift+C: DevTools do Chrome/Edge e "copiar" no Windows Terminal/VS Code;
// - Ctrl+Alt+Space: usado pelo app do Claude;
// - Ctrl+Alt+G: usado pelo Google Drive;
// - Ctrl+Alt+Q/W/E/C e números: no teclado ABNT2 viram AltGr (/, ?, °, ₢, ¹²³…).

use tauri::plugin::TauriPlugin;
use tauri::{AppHandle, Emitter, Runtime};
use tauri_plugin_global_shortcut::{
    Builder, GlobalShortcutExt, Shortcut, ShortcutEvent, ShortcutState,
};

use crate::capture;

pub const DEFAULT_SHORTCUT: &str = "Ctrl+Alt+O";

/// Eventos enviados ao front-end.
pub const SELECTION_CAPTURED_EVENT: &str = "selection-captured";
pub const CAPTURE_FAILED_EVENT: &str = "capture-failed";

pub fn plugin<R: Runtime>() -> TauriPlugin<R> {
    Builder::new().with_handler(on_shortcut).build()
}

/// Registra o atalho padrão. Falha com erro claro se outro programa já o usa.
pub fn register_default<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<(), tauri_plugin_global_shortcut::Error> {
    app.global_shortcut().register(DEFAULT_SHORTCUT)
}

fn on_shortcut<R: Runtime>(app: &AppHandle<R>, _shortcut: &Shortcut, event: ShortcutEvent) {
    // Dispara ao SOLTAR a tecla O: assim ela não fica repetindo e digitando
    // "o" por cima do texto selecionado durante a captura.
    if event.state != ShortcutState::Released {
        return;
    }

    // A captura espera teclas e o clipboard: roda fora da thread da interface.
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || match capture::capture_selection(&app) {
        Ok(selection) => {
            println!(
                "Texto capturado: {} caracteres",
                selection.text.chars().count()
            );
            let _ = app.emit(SELECTION_CAPTURED_EVENT, selection);
        }
        Err(err) => {
            eprintln!("Falha na captura: {err}");
            let _ = app.emit(CAPTURE_FAILED_EVENT, err.to_string());
        }
    });
}
