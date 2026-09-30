// src-tauri/src/state.rs
//
// Estado compartilhado, criado uma única vez na inicialização.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
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
    /// Contador para identificar cada sessão (evita resposta atrasada cair na sessão errada).
    next_session_id: AtomicU64,
}

#[derive(Clone)]
pub struct Session {
    pub id: u64,
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
            next_session_id: AtomicU64::new(1),
        }
    }

    /// Abre uma sessão no popup com o texto capturado.
    pub fn start_session(&self, selection: Selection) {
        if let Ok(mut slot) = self.session.lock() {
            *slot = Some(Session {
                id: self.next_session_id.fetch_add(1, Ordering::SeqCst),
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

    /// Guarda o resultado da IA, desde que a sessão ainda seja a mesma
    /// que fez o pedido. Devolve `false` se ela foi fechada ou trocada.
    pub fn set_session_result(&self, id: u64, result: String) -> bool {
        if let Ok(mut slot) = self.session.lock() {
            if let Some(session) = slot.as_mut().filter(|s| s.id == id) {
                session.result = Some(result);
                return true;
            }
        }
        false
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
            next_session_id: AtomicU64::new(1),
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
            next_session_id: AtomicU64::new(1),
        };
        assert!(!state.has_session());

        state.start_session(Selection {
            text: "ola".into(),
            source_window: 42,
        });
        let id = state.session().expect("sessão aberta").id;
        assert!(state.set_session_result(id, "Olá".into()));

        let session = state.take_session().expect("sessão aberta");
        assert_eq!(session.selection.text, "ola");
        assert_eq!(session.result.as_deref(), Some("Olá"));
        assert!(!state.has_session());
    }

    #[test]
    fn resposta_atrasada_nao_entra_em_outra_sessao() {
        let state = AppState {
            llm: Box::new(crate::llm::GeminiProvider::new(None, None)),
            hotkey_error: Mutex::new(None),
            busy: AtomicBool::new(false),
            session: Mutex::new(None),
            next_session_id: AtomicU64::new(1),
        };
        let selection = Selection {
            text: "texto".into(),
            source_window: 1,
        };

        state.start_session(selection.clone());
        let old_id = state.session().unwrap().id;
        state.take_session(); // usuário cancelou
        state.start_session(selection); // e abriu de novo

        assert!(!state.set_session_result(old_id, "atrasada".into()));
        assert_eq!(state.session().unwrap().result, None);
    }
}
