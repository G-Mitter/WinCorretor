// src-tauri/src/flow.rs
//
// Fluxo disparado pelo atalho, agora em três etapas guiadas pelo popup:
// 1. `run`: captura a seleção e abre o popup junto ao cursor;
// 2. `rewrite`: o usuário escolhe o tom e a IA gera a prévia;
// 3. `apply`, `copy` ou `cancel`: cola no lugar, só copia ou desiste.
// Erros viram notificação do Windows.

use std::time::Instant;

use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::capture::{self, PasteOutcome};
use crate::error::{AppError, AppResult};
use crate::notify;
use crate::popup;
use crate::prompts::TextStyle;
use crate::state::AppState;

/// Eventos enviados à janela principal (tela de acompanhamento).
pub const SELECTION_CAPTURED_EVENT: &str = "selection-captured";
pub const CAPTURE_FAILED_EVENT: &str = "capture-failed";
pub const REWRITE_DONE_EVENT: &str = "rewrite-done";

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct RewriteDone {
    original: String,
    result: String,
    outcome: PasteOutcome,
}

/// Etapa 1. Bloqueia: chame fora da thread da interface.
pub fn run<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<AppState>();

    if state.has_session() {
        println!("Atalho ignorado: o popup já está aberto.");
        return;
    }
    let Some(_busy) = state.try_begin_correction() else {
        println!("Atalho ignorado: já existe uma captura em andamento.");
        return;
    };

    let started = Instant::now();
    let selection = match capture::capture_selection(app) {
        Ok(selection) => selection,
        Err(err) => {
            let _ = app.emit(CAPTURE_FAILED_EVENT, err.to_string());
            return fail(app, &err);
        }
    };
    let _ = app.emit(SELECTION_CAPTURED_EVENT, selection.clone());

    state.start_session(selection.clone());
    if let Err(err) = popup::show_near_cursor(app, &selection) {
        state.take_session();
        return fail(app, &err);
    }

    // Enquanto o usuário escolhe o tom, já abre a conexão com a IA.
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        handle.state::<AppState>().llm.warm_up().await;
    });

    println!(
        "Popup aberto em {} ms ({} caracteres capturados).",
        started.elapsed().as_millis(),
        selection.text.chars().count()
    );
}

/// Etapa 2. Gera a prévia no tom escolhido e guarda na sessão.
pub async fn rewrite<R: Runtime>(app: &AppHandle<R>, style: TextStyle) -> AppResult<String> {
    let state = app.state::<AppState>();
    let session = state.session().ok_or(AppError::NoSession)?;

    let started = Instant::now();
    let result = state
        .llm
        .rewrite(&session.selection.text, style)
        .await
        .inspect_err(|err| {
            eprintln!(
                "IA falhou (\"{}\") após {} ms: {err}",
                style.label(),
                started.elapsed().as_millis()
            )
        })?;
    println!(
        "IA respondeu (\"{}\") em {} ms.",
        style.label(),
        started.elapsed().as_millis()
    );

    // Se o usuário fechou o popup enquanto a IA respondia, descarta.
    if !state.set_session_result(session.id, result.clone()) {
        return Err(AppError::NoSession);
    }
    Ok(result)
}

/// Etapa 3a. Fecha o popup, devolve o foco à janela de origem e cola.
/// Bloqueia: chame fora da thread da interface.
pub fn apply<R: Runtime>(app: &AppHandle<R>) -> AppResult<()> {
    let state = app.state::<AppState>();
    let session = state.take_session().ok_or(AppError::NoSession)?;
    let Some(result) = session.result else {
        popup::hide(app);
        return Err(AppError::NoSession);
    };

    popup::hide(app);
    capture::restore_focus(&session.selection);

    let outcome = capture::paste_result(app, &session.selection, &result)?;
    println!("Resultado entregue: {outcome:?}");
    if outcome == PasteOutcome::CopiedOnly {
        notify::show(
            app,
            "Não foi possível voltar para a janela original. O resultado foi copiado: cole com Ctrl+V.",
        );
    }

    let _ = app.emit(
        REWRITE_DONE_EVENT,
        RewriteDone {
            original: session.selection.text,
            result,
            outcome,
        },
    );
    Ok(())
}

/// Etapa 3b. Só copia o resultado (sem colar e sem restaurar o clipboard),
/// para o usuário colar onde quiser. Bloqueia: chame fora da thread da interface.
pub fn copy<R: Runtime>(app: &AppHandle<R>) -> AppResult<()> {
    let state = app.state::<AppState>();
    let session = state.take_session().ok_or(AppError::NoSession)?;
    popup::hide(app);
    let result = session.result.ok_or(AppError::NoSession)?;

    capture::write_text(app, &result)?;
    capture::restore_focus(&session.selection);
    println!("Resultado copiado para o clipboard.");
    Ok(())
}

/// Etapa 3c. Fecha o popup sem mexer no texto.
/// `return_focus`: devolve o foco à janela de origem (Esc). Falso quando o
/// usuário clicou em outro lugar, para não roubar o foco de onde ele clicou.
pub fn cancel<R: Runtime>(app: &AppHandle<R>, return_focus: bool) {
    let session = app.state::<AppState>().take_session();
    popup::hide(app);
    if let (true, Some(session)) = (return_focus, session) {
        capture::restore_focus(&session.selection);
    }
}

/// Registra a falha no terminal e mostra uma notificação.
fn fail<R: Runtime>(app: &AppHandle<R>, err: &AppError) {
    eprintln!("Falha: {err}");
    notify::show(app, &err.to_string());
}
