// src-tauri/src/hotkey.rs
//
// Atalho global. Combinações evitadas por conflito:
// - Ctrl+Shift+C: DevTools do Chrome/Edge e "copiar" no Windows Terminal/VS Code;
// - Ctrl+Alt+Space: usado pelo app do Claude;
// - Ctrl+Alt+G: usado pelo Google Drive;
// - Ctrl+Alt+Q/W/E/C e números: no teclado ABNT2 viram AltGr (/, ?, °, ₢, ¹²³…).

use tauri::plugin::TauriPlugin;
use tauri::{AppHandle, Runtime};
use tauri_plugin_global_shortcut::{
    Builder, GlobalShortcutExt, Shortcut, ShortcutEvent, ShortcutState,
};

use crate::flow;

pub const DEFAULT_SHORTCUT: &str = "Ctrl+Alt+O";

pub fn plugin<R: Runtime>() -> TauriPlugin<R> {
    Builder::new().with_handler(on_shortcut).build()
}

/// Registra o atalho padrão. Falha com erro claro se outro programa já o usa.
pub fn register_default<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<(), tauri_plugin_global_shortcut::Error> {
    app.global_shortcut().register(DEFAULT_SHORTCUT)
}

/// Libera o atalho (usado ao pausar pela bandeja).
pub fn unregister_default<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<(), tauri_plugin_global_shortcut::Error> {
    app.global_shortcut().unregister(DEFAULT_SHORTCUT)
}

fn on_shortcut<R: Runtime>(app: &AppHandle<R>, _shortcut: &Shortcut, event: ShortcutEvent) {
    // Dispara ao SOLTAR a tecla O: assim ela não fica repetindo e digitando
    // "o" por cima do texto selecionado durante a captura.
    if event.state != ShortcutState::Released {
        return;
    }

    // A captura espera teclas e a IA espera a rede: roda fora da thread da interface.
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || flow::run(&app));
}
