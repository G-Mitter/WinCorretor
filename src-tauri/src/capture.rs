// src-tauri/src/capture.rs
//
// Entrada e saída de texto. Hoje só lê e escreve o clipboard;
// na issue #4 ganha a captura da seleção (Ctrl+C simulado).

use tauri::{AppHandle, Runtime};
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::error::{AppError, AppResult};

pub fn read_text<R: Runtime>(app: &AppHandle<R>) -> AppResult<String> {
    let text = app.clipboard().read_text()?;
    if text.trim().is_empty() {
        return Err(AppError::EmptyText);
    }
    Ok(text)
}

pub fn write_text<R: Runtime>(app: &AppHandle<R>, text: &str) -> AppResult<()> {
    app.clipboard().write_text(text.to_owned())?;
    Ok(())
}
