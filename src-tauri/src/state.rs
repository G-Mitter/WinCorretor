// src-tauri/src/state.rs
//
// Estado compartilhado, criado uma única vez na inicialização.

use std::sync::Mutex;

use crate::llm::{LlmProvider, MockProvider};

pub struct AppState {
    pub llm: Box<dyn LlmProvider>,
    /// Mensagem para o usuário quando o atalho global não pôde ser registrado.
    hotkey_error: Mutex<Option<String>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            // Trocado pelo GeminiProvider na issue #5.
            llm: Box::new(MockProvider),
            hotkey_error: Mutex::new(None),
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
}
