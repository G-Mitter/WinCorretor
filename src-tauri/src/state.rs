// src-tauri/src/state.rs
//
// Estado compartilhado, criado uma única vez na inicialização.

use std::sync::Mutex;

use crate::llm::{GeminiProvider, LlmProvider};

pub struct AppState {
    pub llm: Box<dyn LlmProvider>,
    /// Mensagem para o usuário quando o atalho global não pôde ser registrado.
    hotkey_error: Mutex<Option<String>>,
}

impl AppState {
    pub fn new() -> Self {
        let gemini = GeminiProvider::from_env();
        println!("Provedor de IA: Gemini ({})", gemini.model());
        Self {
            llm: Box::new(gemini),
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
