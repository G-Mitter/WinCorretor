// src-tauri/src/llm/mod.rs
//
// Contrato comum dos provedores de IA. Trocar Gemini por Groq (ou outro)
// vira só uma nova implementação deste trait, sem mexer no resto do app.

mod mock;

pub use mock::MockProvider;

use crate::error::AppResult;
use crate::prompts::TextStyle;

#[async_trait::async_trait]
pub trait LlmProvider: Send + Sync {
    /// Reescreve `text` no estilo pedido e devolve só o texto final.
    async fn rewrite(&self, text: &str, style: TextStyle) -> AppResult<String>;
}
