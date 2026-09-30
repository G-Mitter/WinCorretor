// src-tauri/src/llm/mod.rs
//
// Contrato comum dos provedores de IA e a escolha de qual usar.
// Hoje: Groq como principal (rápido) e Gemini como reserva automática.

mod fallback;
mod gemini;
mod groq;
#[cfg(test)]
mod mock;

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

/// Monta o provedor a partir das chaves no ambiente (ou no .env):
/// - GROQ_API_KEY e GEMINI_API_KEY → Groq com Gemini de reserva;
/// - só uma delas → só esse provedor;
/// - nenhuma → Gemini, que avisa "chave não configurada" ao usar.
pub fn from_env() -> Box<dyn LlmProvider> {
    let groq_key = env_var("GROQ_API_KEY");
    let gemini_key = env_var("GEMINI_API_KEY");

    let gemini = GeminiProvider::new(gemini_key.clone(), env_var("GEMINI_MODEL"));
    match (groq_key, gemini_key) {
        (Some(key), Some(_)) => Box::new(FallbackProvider::new(
            Box::new(GroqProvider::new(Some(key), env_var("GROQ_MODEL"))),
            Box::new(gemini),
        )),
        (Some(key), None) => Box::new(GroqProvider::new(Some(key), env_var("GROQ_MODEL"))),
        (None, _) => Box::new(gemini),
    }
}

/// Lê uma variável de ambiente, ignorando valores vazios.
fn env_var(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
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
