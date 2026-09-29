// src-tauri/src/state.rs
//
// Estado compartilhado, criado uma única vez na inicialização.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use crate::llm::{GeminiProvider, LlmProvider};

pub struct AppState {
    pub llm: Box<dyn LlmProvider>,
    /// Mensagem para o usuário quando o atalho global não pôde ser registrado.
    hotkey_error: Mutex<Option<String>>,
    /// Verdadeiro enquanto uma correção está em andamento.
    busy: AtomicBool,
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
        }
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
        };

        let first = state.try_begin_correction();
        assert!(first.is_some());
        assert!(state.try_begin_correction().is_none());

        drop(first);
        assert!(state.try_begin_correction().is_some());
    }
}
