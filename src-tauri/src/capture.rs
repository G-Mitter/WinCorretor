// src-tauri/src/capture.rs
//
// Captura do texto selecionado em qualquer programa:
// 1. espera soltar as teclas do atalho;
// 2. guarda a janela ativa e o clipboard atual;
// 3. simula Ctrl+C e espera o Windows avisar que algo foi copiado;
// 4. lê o texto e devolve o clipboard original ao usuário.

use tauri::{AppHandle, Runtime};
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::error::{AppError, AppResult};

/// Resultado da captura: o texto e a janela de onde ele veio
/// (usada na issue #6 para colar o resultado no mesmo lugar).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Selection {
    pub text: String,
    pub source_window: isize,
}

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

/// Captura o texto selecionado. Bloqueia por até ~1,5 s: chame fora da thread principal.
#[cfg(windows)]
pub fn capture_selection<R: Runtime>(app: &AppHandle<R>) -> AppResult<Selection> {
    use std::thread::sleep;
    use std::time::{Duration, Instant};

    use crate::win32;

    const KEY_RELEASE_TIMEOUT: Duration = Duration::from_millis(1000);
    const COPY_TIMEOUT: Duration = Duration::from_millis(400);

    if !win32::wait_keys_released(KEY_RELEASE_TIMEOUT) {
        return Err(AppError::Capture(
            "solte as teclas do atalho e tente de novo".into(),
        ));
    }

    let source_window = win32::foreground_window();
    let previous = app.clipboard().read_text().ok();
    let sequence_before = win32::clipboard_sequence();

    if !win32::send_ctrl_c() {
        return Err(AppError::Capture(
            "o Windows bloqueou o Ctrl+C simulado".into(),
        ));
    }

    // Espera o programa de origem colocar a seleção no clipboard.
    let start = Instant::now();
    while win32::clipboard_sequence() == sequence_before {
        if start.elapsed() > COPY_TIMEOUT {
            return Err(AppError::NothingSelected);
        }
        sleep(Duration::from_millis(15));
    }

    let text = app.clipboard().read_text().unwrap_or_default();

    // Devolve o que o usuário tinha copiado antes (apenas texto, por enquanto).
    if let Some(previous) = previous {
        let _ = app.clipboard().write_text(previous);
    }

    if text.trim().is_empty() {
        return Err(AppError::NothingSelected);
    }

    Ok(Selection {
        text,
        source_window,
    })
}

#[cfg(not(windows))]
pub fn capture_selection<R: Runtime>(_app: &AppHandle<R>) -> AppResult<Selection> {
    Err(AppError::Capture(
        "captura disponível apenas no Windows".into(),
    ))
}
