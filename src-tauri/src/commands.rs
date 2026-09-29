// src-tauri/src/commands.rs
//
// Comandos expostos ao front-end via `invoke`.

use tauri::{AppHandle, State};

use crate::capture;
use crate::error::AppResult;
use crate::prompts::TextStyle;
use crate::state::AppState;

/// Lê o clipboard, reescreve no estilo pedido e devolve o resultado ao clipboard.
#[tauri::command]
pub async fn process_clipboard_text(
    app: AppHandle,
    state: State<'_, AppState>,
    style: TextStyle,
) -> AppResult<String> {
    let original = capture::read_text(&app)?;
    let result = state.llm.rewrite(&original, style).await?;
    capture::write_text(&app, &result)?;
    Ok(result)
}
