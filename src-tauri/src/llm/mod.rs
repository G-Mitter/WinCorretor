// src-tauri/src/llm/mod.rs
//
// Contrato comum dos provedores de IA e a escolha de qual usar.
// Groq como principal (rápido) e Gemini como reserva automática.

mod fallback;
pub mod gemini;
pub mod groq;
#[cfg(test)]
mod mock;

use std::sync::Arc;
use std::time::Duration;

pub use fallback::FallbackProvider;
pub use gemini::GeminiProvider;
pub use groq::GroqProvider;

use reqwest::Client;

use crate::error::{AppError, AppResult};
use crate::prompts::TextStyle;

/// Limite de tamanho por pedido: protege a cota gratuita e a latência.
pub const MAX_INPUT_CHARS: usize = 8_000;

#[async_trait::async_trait]
pub trait LlmProvider: Send + Sync {
    /// Reescreve `text` no estilo pedido e devolve só o texto final.
    async fn rewrite(&self, text: &str, style: TextStyle) -> AppResult<String>;

    /// Abre a conexão com antecedência (enquanto o usuário escolhe o tom),
    /// para o pedido de verdade não pagar o custo de conectar. Opcional.
    async fn warm_up(&self) {}

    /// Nome para os logs, ex.: "Groq (openai/gpt-oss-20b)".
    fn describe(&self) -> String;
}

/// Monta o provedor conforme as chaves disponíveis:
/// - Groq e Gemini → Groq com Gemini de reserva;
/// - só uma delas → só esse provedor;
/// - nenhuma → Gemini, que avisa "chave não configurada" ao usar.
pub fn build(
    groq_key: Option<String>,
    gemini_key: Option<String>,
    groq_model: &str,
    gemini_model: &str,
) -> Arc<dyn LlmProvider> {
    let groq = |key| GroqProvider::new(Some(key), Some(groq_model.to_string()));
    let gemini = GeminiProvider::new(gemini_key.clone(), Some(gemini_model.to_string()));
    match (groq_key, gemini_key) {
        (Some(key), Some(_)) => {
            Arc::new(FallbackProvider::new(Box::new(groq(key)), Box::new(gemini)))
        }
        (Some(key), None) => Arc::new(groq(key)),
        (None, _) => Arc::new(gemini),
    }
}

/// Cliente HTTP com os mesmos cuidados para todos os provedores.
fn http_client(timeout: Duration) -> Client {
    Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(timeout)
        // Descarta conexões paradas antes que o servidor as derrube,
        // evitando esperar por uma conexão "morta" (ex.: após trocar de rede).
        .pool_idle_timeout(Duration::from_secs(60))
        .tcp_keepalive(Duration::from_secs(30))
        .build()
        .unwrap_or_default()
}

/// Traduz falhas de rede para mensagens simples; o detalhe técnico vai só para o terminal.
fn network_error(err: reqwest::Error) -> AppError {
    eprintln!("Detalhe da falha de rede: {err}");
    if err.is_timeout() {
        AppError::Timeout
    } else {
        AppError::Network
    }
}

/// Abre a conexão TLS com o servidor para ela ficar pronta no pool.
async fn warm_up_connection(client: &Client, url: &str, name: &str) {
    let started = std::time::Instant::now();
    match client
        .head(url)
        .timeout(Duration::from_secs(5))
        .send()
        .await
    {
        Ok(_) => println!(
            "Conexão com {name} aquecida em {} ms.",
            started.elapsed().as_millis()
        ),
        Err(err) => eprintln!("Aquecimento da conexão com {name} falhou: {err}"),
    }
}
