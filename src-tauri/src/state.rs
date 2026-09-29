// src-tauri/src/state.rs
//
// Estado compartilhado, criado uma única vez na inicialização.

use crate::llm::{LlmProvider, MockProvider};

pub struct AppState {
    pub llm: Box<dyn LlmProvider>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            // Trocado pelo GeminiProvider na issue #5.
            llm: Box::new(MockProvider),
        }
    }
}
