// src-tauri/src/error.rs
//
// Erro único do app. Toda função que pode falhar devolve `Result<_, AppError>`.

use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    // Usado só no Windows; em outros sistemas a captura não existe.
    #[cfg_attr(not(windows), allow(dead_code))]
    #[error("Nenhum texto selecionado. Selecione um texto antes de usar o atalho.")]
    NothingSelected,

    #[error("Não foi possível capturar a seleção: {0}")]
    Capture(String),

    #[error("Erro no popup: {0}")]
    Popup(String),

    #[error("Nenhuma correção em andamento. Use o atalho de novo.")]
    NoSession,

    #[error("Erro ao acessar a área de transferência: {0}")]
    Clipboard(String),

    #[error("Texto muito longo ({0} caracteres). O limite é {1}.")]
    TextTooLong(usize, usize),

    #[error("Nenhuma chave de IA configurada. Coloque GROQ_API_KEY (ou GEMINI_API_KEY) no arquivo src-tauri/.env.")]
    MissingApiKey,

    #[error("A chave da API de IA foi recusada. Confira o valor no arquivo .env.")]
    InvalidApiKey,

    #[error(
        "Limite gratuito do Gemini atingido. Aguarde um minuto ou troque o modelo em GEMINI_MODEL."
    )]
    RateLimited,

    #[error("Sem conexão com a IA. Verifique a internet e tente de novo.")]
    Network,

    #[error("A IA demorou demais para responder. Tente de novo.")]
    Timeout,

    #[error("A IA recusou o texto ({0}).")]
    Blocked(String),

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
