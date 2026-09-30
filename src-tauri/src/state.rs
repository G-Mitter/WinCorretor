// src-tauri/src/state.rs
//
// Estado compartilhado, criado uma única vez na inicialização.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use crate::capture::Selection;
use crate::llm::{GeminiProvider, LlmProvider};

pub struct AppState {
    pub llm: Box<dyn LlmProvider>,
    /// Mensagem para o usuário quando o atalho global não pôde ser registrado.
    hotkey_error: Mutex<Option<String>>,
    /// Verdadeiro enquanto uma correção está em andamento.
    busy: AtomicBool,
    /// Correção aberta no popup: o texto capturado e, depois, o resultado da IA.
    session: Mutex<Option<Session>>,
}

#[derive(Clone)]
pub struct Session {
    pub selection: Selection,
    pub result: Option<String>,
}

/// Enquanto existir, marca o app como ocupado. Ao sair de escopo, libera.
pub struct BusyGuard<'a>(&'a AtomicBool);

impl Drop for BusyGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

impl AppState {
    pub fn new() -> Self {
        let gemini = GeminiProvider::from_env();
        println!("Provedor de IA: Gemini ({})", gemini.model());
        Self {
            llm: Box::new(gemini),
            hotkey_error: Mutex::new(None),
            busy: AtomicBool::new(false),
            session: Mutex::new(None),
        }
    }

    /// Abre uma sessão no popup com o texto capturado.
    pub fn start_session(&self, selection: Selection) {
        if let Ok(mut slot) = self.session.lock() {
            *slot = Some(Session {
                selection,
                result: None,
            });
        }
    }

    pub fn has_session(&self) -> bool {
        self.session.lock().map(|s| s.is_some()).unwrap_or(false)
    }

    /// Cópia da sessão atual (para ler o texto sem segurar o lock durante a IA).
    pub fn session(&self) -> Option<Session> {
        self.session.lock().ok().and_then(|s| s.clone())
    }

    pub fn set_session_result(&self, result: String) {
        if let Ok(mut slot) = self.session.lock() {
            if let Some(session) = slot.as_mut() {
                session.result = Some(result);
            }
        }
    }

    /// Encerra a sessão e devolve o que havia nela.
    pub fn take_session(&self) -> Option<Session> {
        self.session.lock().ok().and_then(|mut s| s.take())
    }

    pub fn set_hotkey_error(&self, message: String) {
        if let Ok(mut slot) = self.hotkey_error.lock() {
            *slot = Some(message);
        }
    }

    pub fn hotkey_error(&self) -> Option<String> {
        self.hotkey_error.lock().ok().and_then(|slot| slot.clone())
    }

    /// Começa uma correção, ou devolve `None` se já houver uma em andamento.
    pub fn try_begin_correction(&self) -> Option<BusyGuard<'_>> {
        if self.busy.swap(true, Ordering::SeqCst) {
            None
        } else {
            Some(BusyGuard(&self.busy))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn so_uma_correcao_por_vez() {
        let busy = AtomicBool::new(false);
        let state = AppState {
            llm: Box::new(crate::llm::GeminiProvider::new(None, None)),
            hotkey_error: Mutex::new(None),
            busy,
            session: Mutex::new(None),
        };

        let first = state.try_begin_correction();
        assert!(first.is_some());
        assert!(state.try_begin_correction().is_none());

        drop(first);
        assert!(state.try_begin_correction().is_some());
    }

    #[test]
    fn sessao_guarda_texto_e_resultado() {
        let state = AppState {
            llm: Box::new(crate::llm::GeminiProvider::new(None, None)),
            hotkey_error: Mutex::new(None),
            busy: AtomicBool::new(false),
            session: Mutex::new(None),
        };
        assert!(!state.has_session());

        state.start_session(Selection {
            text: "ola".into(),
            source_window: 42,
        });
        state.set_session_result("Olá".into());

        let session = state.take_session().expect("sessão aberta");
        assert_eq!(session.selection.text, "ola");
        assert_eq!(session.result.as_deref(), Some("Olá"));
        assert!(!state.has_session());
    }
}
