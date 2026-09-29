// src-tauri/src/notify.rs
//
// Avisos do Windows (notificações no canto da tela). Usados quando algo dá
// errado, porque durante o fluxo a janela do WinCorretor não está visível.

use tauri::{AppHandle, Runtime};
use tauri_plugin_notification::NotificationExt;

const TITLE: &str = "WinCorretor";

pub fn show<R: Runtime>(app: &AppHandle<R>, message: &str) {
    if let Err(err) = app
        .notification()
        .builder()
        .title(TITLE)
        .body(message)
        .show()
    {
        eprintln!("Não foi possível mostrar a notificação: {err}");
    }
}
