// src-tauri/src/commands.rs
//
// Comandos expostos ao front-end via `invoke`.

use tauri::{AppHandle, State};

use crate::error::{AppError, AppResult};
use crate::flow;
use crate::prompts::TextStyle;
use crate::state::AppState;

/// Devolve a mensagem de erro do atalho global, se ele não pôde ser registrado.
#[tauri::command]
pub fn hotkey_error(state: State<'_, AppState>) -> Option<String> {
    state.hotkey_error()
}

/// Popup: gera a prévia do texto no tom escolhido.
#[tauri::command]
pub async fn popup_rewrite(app: AppHandle, style: TextStyle) -> AppResult<String> {
    flow::rewrite(&app, style).await
}

/// Popup: aplica a prévia no lugar do texto selecionado.
#[tauri::command]
pub async fn popup_apply(app: AppHandle) -> AppResult<()> {
    tauri::async_runtime::spawn_blocking(move || flow::apply(&app))
        .await
        .map_err(|e| AppError::Popup(e.to_string()))?
}

/// Popup: só copia a prévia para o clipboard.
#[tauri::command]
pub async fn popup_copy(app: AppHandle) -> AppResult<()> {
    tauri::async_runtime::spawn_blocking(move || flow::copy(&app))
        .await
        .map_err(|e| AppError::Popup(e.to_string()))?
}

/// Popup: fecha sem alterar nada e devolve o foco à janela de origem.
#[tauri::command]
pub async fn popup_cancel(app: AppHandle) {
    let _ = tauri::async_runtime::spawn_blocking(move || flow::cancel(&app, true)).await;
}
