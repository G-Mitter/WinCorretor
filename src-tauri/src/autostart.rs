// src-tauri/src/autostart.rs
//
// "Iniciar com o Windows". O próprio Windows guarda essa opção (entrada em
// HKCU\Software\Microsoft\Windows\CurrentVersion\Run), por isso ela não fica
// no settings.json: a fonte da verdade é o sistema. Desligado por padrão.

use tauri::plugin::TauriPlugin;
use tauri::{AppHandle, Runtime};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};

use crate::error::{AppError, AppResult};

/// Argumento passado ao app quando ele é aberto pelo Windows no login.
pub const AUTOSTART_ARG: &str = "--autostart";

pub fn plugin<R: Runtime>() -> TauriPlugin<R> {
    tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec![AUTOSTART_ARG]))
}

pub fn is_enabled<R: Runtime>(app: &AppHandle<R>) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}

/// Liga ou desliga; não faz nada se já estiver no estado pedido.
pub fn set<R: Runtime>(app: &AppHandle<R>, enabled: bool) -> AppResult<()> {
    if is_enabled(app) == enabled {
        return Ok(());
    }
    let manager = app.autolaunch();
    let result = if enabled {
        manager.enable()
    } else {
        manager.disable()
    };
    result.map_err(|e| {
        AppError::Settings(format!("não foi possível alterar o início automático: {e}"))
    })
}
