// src-tauri/src/error.rs
//
// Erro único do app. Toda função que pode falhar devolve `Result<_, AppError>`.

use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Nenhum texto encontrado para processar.")]
    EmptyText,

    #[error("Erro ao acessar a área de transferência: {0}")]
    Clipboard(String),

    // Usado pelo GeminiProvider na issue #5.
    #[allow(dead_code)]
    #[error("Erro no provedor de IA: {0}")]
    Llm(String),
}

impl From<tauri_plugin_clipboard_manager::Error> for AppError {
    fn from(err: tauri_plugin_clipboard_manager::Error) -> Self {
        AppError::Clipboard(err.to_string())
    }
}

// O front-end recebe o erro como uma mensagem de texto simples.
impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
