// src-tauri/src/capture.rs
//
// Captura do texto selecionado em qualquer programa:
// 1. espera soltar as teclas do atalho;
// 2. guarda a janela ativa e o clipboard atual;
// 3. simula Ctrl+C e espera o Windows avisar que algo foi copiado;
// 4. lê o texto e devolve o clipboard original ao usuário.
//
// E a colagem do resultado (`paste_result`), que faz o caminho inverso.

use tauri::{AppHandle, Runtime};
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::error::{AppError, AppResult};

/// Como o resultado chegou ao usuário.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PasteOutcome {
    /// O texto selecionado foi substituído pelo resultado.
    #[cfg_attr(not(windows), allow(dead_code))]
    Pasted,
    /// Não era seguro colar (o usuário mudou de janela): o resultado ficou no clipboard.
    CopiedOnly,
}

/// Resultado da captura: o texto e a janela de onde ele veio
/// (usada na issue #6 para colar o resultado no mesmo lugar).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Selection {
    pub text: String,
    pub source_window: isize,
}

/// Troca temporária do clipboard, escondida do histórico do Win+V quando possível.
/// Se a forma "privada" falhar, usa a escrita normal para não quebrar o fluxo.
#[cfg_attr(not(windows), allow(dead_code))]
fn write_temporary<R: Runtime>(app: &AppHandle<R>, text: &str) -> AppResult<()> {
    #[cfg(windows)]
    if crate::win32::set_clipboard_text_private(text) {
        return Ok(());
    }
    write_text(app, text)
}

/// Escrita normal: usada quando o usuário QUER o texto no clipboard (ex.: "copiar").
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
        let _ = write_temporary(app, &previous);
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

/// Cola `text` no lugar da seleção e depois restaura o clipboard do usuário.
/// Se o usuário trocou de janela enquanto a IA respondia, não cola: deixa o
/// resultado no clipboard para ele colar onde quiser.
#[cfg(windows)]
pub fn paste_result<R: Runtime>(
    app: &AppHandle<R>,
    selection: &Selection,
    text: &str,
) -> AppResult<PasteOutcome> {
    use std::thread::sleep;
    use std::time::Duration;

    use crate::win32;

    /// Tempo para o programa de destino ler o clipboard antes de restaurarmos.
    const PASTE_SETTLE: Duration = Duration::from_millis(300);

    let same_window = win32::foreground_window() == selection.source_window;
    if !same_window || !win32::wait_keys_released(Duration::from_millis(1000)) {
        write_text(app, text)?;
        return Ok(PasteOutcome::CopiedOnly);
    }

    let previous = app.clipboard().read_text().ok();
    write_temporary(app, text)?;

    if !win32::send_ctrl_v() {
        // O resultado continua no clipboard: o usuário pode colar manualmente.
        return Ok(PasteOutcome::CopiedOnly);
    }

    sleep(PASTE_SETTLE);
    if let Some(previous) = previous {
        let _ = write_temporary(app, &previous);
    }
    Ok(PasteOutcome::Pasted)
}

#[cfg(not(windows))]
pub fn paste_result<R: Runtime>(
    app: &AppHandle<R>,
    _selection: &Selection,
    text: &str,
) -> AppResult<PasteOutcome> {
    write_text(app, text)?;
    Ok(PasteOutcome::CopiedOnly)
}

/// Devolve o foco à janela de origem (o popup roubou o foco para receber o teclado).
#[cfg(windows)]
pub fn restore_focus(selection: &Selection) -> bool {
    use std::thread::sleep;
    use std::time::Duration;

    let ok = crate::win32::focus_window(selection.source_window);
    // Dá tempo ao Windows de concluir a troca antes de simular teclas.
    sleep(Duration::from_millis(60));
    ok
}

#[cfg(not(windows))]
pub fn restore_focus(_selection: &Selection) -> bool {
    false
}
