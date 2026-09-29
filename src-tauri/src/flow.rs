// src-tauri/src/flow.rs
//
// Fluxo disparado pelo atalho: captura a seleção e pede a correção à IA.
// Na issue #7 ganha o último passo: colar o resultado no lugar do texto.

use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::capture::{self, Selection};
use crate::prompts::TextStyle;
use crate::state::AppState;

/// Eventos enviados ao front-end.
pub const SELECTION_CAPTURED_EVENT: &str = "selection-captured";
pub const CAPTURE_FAILED_EVENT: &str = "capture-failed";
pub const REWRITE_DONE_EVENT: &str = "rewrite-done";
pub const REWRITE_FAILED_EVENT: &str = "rewrite-failed";

/// Estilo usado pelo atalho até existir o popup de escolha (Fase 2).
const DEFAULT_STYLE: TextStyle = TextStyle::Grammar;

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct RewriteDone {
    original: String,
    result: String,
    style: TextStyle,
}

/// Executa o fluxo completo. Bloqueia: chame fora da thread da interface.
pub fn run<R: Runtime>(app: &AppHandle<R>) {
    let selection = match capture::capture_selection(app) {
        Ok(selection) => selection,
        Err(err) => {
            eprintln!("Falha na captura: {err}");
            let _ = app.emit(CAPTURE_FAILED_EVENT, err.to_string());
            return;
        }
    };

    println!(
        "Texto capturado: {} caracteres. Pedindo estilo {}...",
        selection.text.chars().count(),
        DEFAULT_STYLE.label()
    );
    let _ = app.emit(SELECTION_CAPTURED_EVENT, selection.clone());

    rewrite(app, selection);
}

fn rewrite<R: Runtime>(app: &AppHandle<R>, selection: Selection) {
    let state = app.state::<AppState>();
    let result = tauri::async_runtime::block_on(state.llm.rewrite(&selection.text, DEFAULT_STYLE));

    match result {
        Ok(result) => {
            println!("Resposta da IA: {} caracteres", result.chars().count());
            let _ = app.emit(
                REWRITE_DONE_EVENT,
                RewriteDone {
                    original: selection.text,
                    result,
                    style: DEFAULT_STYLE,
                },
            );
        }
        Err(err) => {
            eprintln!("Falha na IA: {err}");
            let _ = app.emit(REWRITE_FAILED_EVENT, err.to_string());
        }
    }
}
