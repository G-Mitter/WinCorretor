// src-tauri/src/hotkey.rs
//
// Atalho global, configurável na tela de configurações.
// Combinações evitadas por conflito:
// - Ctrl+Shift+C: DevTools do Chrome/Edge e "copiar" no Windows Terminal/VS Code;
// - Ctrl+Alt+Space: usado pelo app do Claude;
// - Ctrl+Alt+G: usado pelo Google Drive;
// - Ctrl+Alt+Q/W/E/C e números: no teclado ABNT2 viram AltGr (/, ?, °, ₢, ¹²³…).

use std::str::FromStr;

use tauri::plugin::TauriPlugin;
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_global_shortcut::{
    Builder, GlobalShortcutExt, Shortcut, ShortcutEvent, ShortcutState,
};

use crate::error::{AppError, AppResult};
use crate::flow;
use crate::state::AppState;

pub fn plugin<R: Runtime>() -> TauriPlugin<R> {
    Builder::new().with_handler(on_shortcut).build()
}

/// Confere se o texto é um atalho válido e exige pelo menos um modificador
/// (Ctrl, Alt, Shift ou Win), para não roubar uma tecla comum do usuário.
pub fn validate(shortcut: &str) -> AppResult<String> {
    let text = shortcut.trim();
    let parsed =
        Shortcut::from_str(text).map_err(|_| AppError::InvalidShortcut(text.to_string()))?;
    if parsed.mods.is_empty() {
        return Err(AppError::InvalidShortcut(text.to_string()));
    }
    Ok(text.to_string())
}

pub fn register<R: Runtime>(app: &AppHandle<R>, shortcut: &str) -> AppResult<()> {
    app.global_shortcut()
        .register(shortcut)
        .map_err(|_| AppError::ShortcutInUse(shortcut.to_string()))
}

pub fn unregister<R: Runtime>(app: &AppHandle<R>, shortcut: &str) {
    let _ = app.global_shortcut().unregister(shortcut);
}

/// Troca o atalho. Se o novo estiver ocupado, volta para o antigo.
pub fn change<R: Runtime>(app: &AppHandle<R>, old: &str, new: &str) -> AppResult<()> {
    if old == new {
        return Ok(());
    }
    unregister(app, old);
    if let Err(err) = register(app, new) {
        let _ = register(app, old);
        return Err(err);
    }
    Ok(())
}

fn on_shortcut<R: Runtime>(app: &AppHandle<R>, _shortcut: &Shortcut, event: ShortcutEvent) {
    // Dispara ao SOLTAR a tecla: assim ela não fica repetindo e digitando
    // por cima do texto selecionado durante a captura.
    if event.state != ShortcutState::Released || app.state::<AppState>().is_paused() {
        return;
    }

    // A captura espera teclas e a IA espera a rede: roda fora da thread da interface.
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || flow::run(&app));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aceita_atalho_com_modificador() {
        assert_eq!(validate(" Ctrl+Alt+O ").unwrap(), "Ctrl+Alt+O");
        assert!(validate("Ctrl+Shift+K").is_ok());
    }

    #[test]
    fn recusa_atalho_invalido_ou_sem_modificador() {
        assert!(matches!(
            validate("banana"),
            Err(AppError::InvalidShortcut(_))
        ));
        assert!(matches!(validate("O"), Err(AppError::InvalidShortcut(_))));
    }
}
