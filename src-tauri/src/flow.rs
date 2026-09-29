// src-tauri/src/flow.rs
//
// Fluxo disparado pelo atalho:
// captura a seleção → pede a correção à IA → cola o resultado no lugar.

use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::capture::{self, PasteOutcome, Selection};
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
    outcome: PasteOutcome,
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

    rewrite_and_paste(app, selection);
}

fn rewrite_and_paste<R: Runtime>(app: &AppHandle<R>, selection: Selection) {
    let state = app.state::<AppState>();
    let result = tauri::async_runtime::block_on(state.llm.rewrite(&selection.text, DEFAULT_STYLE));

    let result = match result {
        Ok(result) => result,
        Err(err) => {
            eprintln!("Falha na IA: {err}");
            let _ = app.emit(REWRITE_FAILED_EVENT, err.to_string());
            return;
        }
    };
    println!("Resposta da IA: {} caracteres", result.chars().count());

    // Se nem copiar para o clipboard der certo, avisa como falha.
    let outcome = match capture::paste_result(app, &selection, &result) {
        Ok(outcome) => outcome,
        Err(err) => {
            eprintln!("Falha ao colar: {err}");
            let _ = app.emit(REWRITE_FAILED_EVENT, err.to_string());
            return;
        }
    };
    println!("Resultado entregue: {outcome:?}");

    let _ = app.emit(
        REWRITE_DONE_EVENT,
        RewriteDone {
            original: selection.text,
            result,
            style: DEFAULT_STYLE,
            outcome,
        },
    );
}
