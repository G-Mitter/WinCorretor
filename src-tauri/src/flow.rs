// src-tauri/src/flow.rs
//
// Fluxo disparado pelo atalho:
// captura a seleção → pede a correção à IA → cola o resultado no lugar.
// Erros viram notificação do Windows, já que a janela do app fica escondida.

use std::time::Instant;

use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::capture::{self, PasteOutcome};
use crate::notify;
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
    let state = app.state::<AppState>();

    // Ignora o atalho se a correção anterior ainda não terminou.
    let Some(_busy) = state.try_begin_correction() else {
        println!("Atalho ignorado: já existe uma correção em andamento.");
        return;
    };

    let started = Instant::now();

    // 1. Captura
    let selection = match capture::capture_selection(app) {
        Ok(selection) => selection,
        Err(err) => return fail(app, CAPTURE_FAILED_EVENT, &err.to_string()),
    };
    let captured_at = Instant::now();
    let _ = app.emit(SELECTION_CAPTURED_EVENT, selection.clone());

    // 2. IA
    let rewritten =
        tauri::async_runtime::block_on(state.llm.rewrite(&selection.text, DEFAULT_STYLE));
    let result = match rewritten {
        Ok(result) => result,
        Err(err) => return fail(app, REWRITE_FAILED_EVENT, &err.to_string()),
    };
    let rewritten_at = Instant::now();

    // 3. Colar
    let outcome = match capture::paste_result(app, &selection, &result) {
        Ok(outcome) => outcome,
        Err(err) => return fail(app, REWRITE_FAILED_EVENT, &err.to_string()),
    };
    let finished_at = Instant::now();

    println!(
        "Correção \"{}\" concluída ({outcome:?}) em {} ms: captura {} ms, IA {} ms, colar {} ms, {} caracteres.",
        DEFAULT_STYLE.label(),
        (finished_at - started).as_millis(),
        (captured_at - started).as_millis(),
        (rewritten_at - captured_at).as_millis(),
        (finished_at - rewritten_at).as_millis(),
        selection.text.chars().count(),
    );

    if outcome == PasteOutcome::CopiedOnly {
        notify::show(
            app,
            "Você trocou de janela durante a correção. O resultado foi copiado: cole com Ctrl+V.",
        );
    }

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

/// Registra a falha no terminal, avisa a janela e mostra uma notificação.
fn fail<R: Runtime>(app: &AppHandle<R>, event: &str, message: &str) {
    eprintln!("Falha: {message}");
    let _ = app.emit(event, message);
    notify::show(app, message);
}
