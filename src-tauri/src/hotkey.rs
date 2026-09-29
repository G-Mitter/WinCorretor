// src-tauri/src/hotkey.rs
//
// Atalho global. Ctrl+Shift+C foi evitado porque conflita com o DevTools
// do Chrome/Edge e com "copiar" no Windows Terminal e no VS Code.

use tauri::plugin::TauriPlugin;
use tauri::{AppHandle, Emitter, Runtime};
use tauri_plugin_global_shortcut::{
    Builder, GlobalShortcutExt, Shortcut, ShortcutEvent, ShortcutState,
};

pub const DEFAULT_SHORTCUT: &str = "Ctrl+Alt+Space";

/// Evento enviado ao front-end quando o atalho é pressionado.
pub const SHORTCUT_EVENT: &str = "shortcut-pressed";

pub fn plugin<R: Runtime>() -> TauriPlugin<R> {
    Builder::new().with_handler(on_shortcut).build()
}

/// Registra o atalho padrão. Falha com erro claro se outro programa já o usa.
pub fn register_default<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<(), tauri_plugin_global_shortcut::Error> {
    app.global_shortcut().register(DEFAULT_SHORTCUT)
}

fn on_shortcut<R: Runtime>(app: &AppHandle<R>, shortcut: &Shortcut, event: ShortcutEvent) {
    if event.state == ShortcutState::Pressed {
        println!("Atalho global acionado: {shortcut:?}");
        let _ = app.emit(SHORTCUT_EVENT, ());
    }
}
