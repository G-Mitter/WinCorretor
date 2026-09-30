// src-tauri/src/popup.rs
//
// Janela pequena que aparece junto ao cursor para escolher o tom.
// É criada escondida na inicialização (tauri.conf.json) e só é mostrada
// e reposicionada a cada uso, por isso abre instantaneamente.

use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, Runtime};

use crate::capture::Selection;
use crate::error::{AppError, AppResult};
use crate::prompts::TextStyle;
use crate::state::AppState;

pub const POPUP_LABEL: &str = "popup";
pub const POPUP_OPEN_EVENT: &str = "popup-open";

/// Distância entre o cursor e o canto do popup, em pixels.
const CURSOR_GAP: i32 = 16;
/// Quantos caracteres do texto original aparecem no topo do popup.
const SNIPPET_CHARS: usize = 140;

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct PopupOpen {
    snippet: String,
    total_chars: usize,
    /// Tom que já vem selecionado (escolhido nas configurações).
    default_style: TextStyle,
}

/// Posiciona o popup perto do mouse (sem sair da tela), mostra e dá foco.
pub fn show_near_cursor<R: Runtime>(app: &AppHandle<R>, selection: &Selection) -> AppResult<()> {
    let window = app
        .get_webview_window(POPUP_LABEL)
        .ok_or_else(|| AppError::Popup("janela do popup não encontrada".into()))?;

    let cursor = app.cursor_position().map_err(popup_err)?;
    let size = window.outer_size().map_err(popup_err)?;
    let (width, height) = (size.width as i32, size.height as i32);
    let (cx, cy) = (cursor.x as i32, cursor.y as i32);

    let mut x = cx + CURSOR_GAP;
    let mut y = cy + CURSOR_GAP;

    // Mantém o popup dentro da área útil do monitor (sem cobrir a barra de tarefas).
    if let Ok(Some(monitor)) = app.monitor_from_point(cursor.x, cursor.y) {
        let area = monitor.work_area();
        let (left, top) = (area.position.x, area.position.y);
        let right = left + area.size.width as i32;
        let bottom = top + area.size.height as i32;

        if x + width > right {
            x = cx - CURSOR_GAP - width; // abre para a esquerda do cursor
        }
        if y + height > bottom {
            y = cy - CURSOR_GAP - height; // abre para cima do cursor
        }
        x = x.clamp(left, (right - width).max(left));
        y = y.clamp(top, (bottom - height).max(top));
    }

    window
        .set_position(PhysicalPosition::new(x, y))
        .map_err(popup_err)?;

    let snippet: String = selection.text.chars().take(SNIPPET_CHARS).collect();
    let _ = app.emit_to(
        POPUP_LABEL,
        POPUP_OPEN_EVENT,
        PopupOpen {
            snippet,
            total_chars: selection.text.chars().count(),
            default_style: app.state::<AppState>().settings().default_style,
        },
    );

    window.show().map_err(popup_err)?;
    window.set_focus().map_err(popup_err)?;
    Ok(())
}

pub fn hide<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window(POPUP_LABEL) {
        let _ = window.hide();
    }
}

fn popup_err(err: tauri::Error) -> AppError {
    AppError::Popup(err.to_string())
}
