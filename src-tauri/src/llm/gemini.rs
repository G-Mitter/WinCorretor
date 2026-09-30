// src-tauri/src/llm/gemini.rs
//
// Provedor Google Gemini (camada gratuita do Google AI Studio).
// Documentação: https://ai.google.dev/api/generate-content

use std::time::Duration;

use reqwest::{Client, StatusCode};
use serde::Deserialize;
use serde_json::json;

use super::{http_client, network_error, warm_up_connection, LlmProvider, MAX_INPUT_CHARS};
use crate::error::{AppError, AppResult};
use crate::prompts::{self, TextStyle};

const DEFAULT_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";
/// Modelo da camada gratuita usado como reserva. Pode ser trocado pela variável GEMINI_MODEL.
/// Medido em 30/09/2026: flash respondeu em 5–12 s e flash-lite em 7–15 s.
pub const DEFAULT_MODEL: &str = "gemini-3.5-flash";

pub struct GeminiProvider {
    client: Client,
    api_key: Option<String>,
    model: String,
    base_url: String,
}

impl GeminiProvider {
    pub fn new(api_key: Option<String>, model: Option<String>) -> Self {
        Self::with_base_url(api_key, model, DEFAULT_BASE_URL.to_string())
    }

    fn with_base_url(api_key: Option<String>, model: Option<String>, base_url: String) -> Self {
        let client = http_client(Duration::from_secs(15));
        Self {
            client,
            api_key,
            model: model.unwrap_or_else(|| DEFAULT_MODEL.to_string()),
            base_url,
        }
    }
}

#[async_trait::async_trait]
impl LlmProvider for GeminiProvider {
    async fn rewrite(&self, text: &str, style: TextStyle) -> AppResult<String> {
        let api_key = self.api_key.as_deref().ok_or(AppError::MissingApiKey)?;

        let length = text.chars().count();
        if length > MAX_INPUT_CHARS {
            return Err(AppError::TextTooLong(length, MAX_INPUT_CHARS));
        }

        let url = format!("{}/models/{}:generateContent", self.base_url, self.model);
        let body = json!({
            "systemInstruction": { "parts": [{ "text": prompts::system_instruction(style) }] },
            "contents": [{ "role": "user", "parts": [{ "text": prompts::user_content(text) }] }],
            "generationConfig": { "temperature": 0.2 }
        });

        let response = self
            .client
            .post(url)
            .header("x-goog-api-key", api_key)
            .json(&body)
            .send()
            .await
            .map_err(network_error)?;

        let status = response.status();
        let raw = response.text().await.map_err(network_error)?;

        if !status.is_success() {
            return Err(map_http_error(status, &raw));
        }

        let parsed: GenerateResponse = serde_json::from_str(&raw)
            .map_err(|e| AppError::Llm(format!("resposta inesperada do Gemini: {e}")))?;
        extract_text(parsed)
    }

    async fn warm_up(&self) {
        warm_up_connection(&self.client, &self.base_url, "Gemini").await;
    }

    fn describe(&self) -> String {
        format!("Gemini ({})", self.model)
    }
}

fn map_http_error(status: StatusCode, raw: &str) -> AppError {
    let message = serde_json::from_str::<ErrorResponse>(raw)
        .map(|e| e.error.message)
        .unwrap_or_else(|_| raw.chars().take(200).collect());

    match status {
        StatusCode::TOO_MANY_REQUESTS => AppError::RateLimited,
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => AppError::InvalidApiKey,
        StatusCode::BAD_REQUEST if message.to_lowercase().contains("api key") => {
            AppError::InvalidApiKey
        }
        _ => AppError::Llm(format!("HTTP {}: {message}", status.as_u16())),
    }
}

fn extract_text(response: GenerateResponse) -> AppResult<String> {
    if let Some(reason) = response.prompt_feedback.and_then(|f| f.block_reason) {
        return Err(AppError::Blocked(reason));
    }

    let candidate = response
        .candidates
        .into_iter()
        .next()
        .ok_or_else(|| AppError::Llm("o Gemini não devolveu nenhuma resposta".into()))?;

    // Ignora partes de "raciocínio" (thought) e junta só o texto final.
    let text: String = candidate
        .content
        .map(|c| c.parts)
        .unwrap_or_default()
        .into_iter()
        .filter(|p| !p.thought)
        .filter_map(|p| p.text)
        .collect();

    let text = prompts::clean_output(&text);
    if text.is_empty() {
        let reason = candidate.finish_reason.unwrap_or_else(|| "vazia".into());
        return Err(AppError::Blocked(reason));
    }
    Ok(text)
}

#[derive(Deserialize)]
struct GenerateResponse {
    #[serde(default)]
    candidates: Vec<Candidate>,
    #[serde(rename = "promptFeedback")]
    prompt_feedback: Option<PromptFeedback>,
}

#[derive(Deserialize)]
struct Candidate {
    content: Option<Content>,
    #[serde(rename = "finishReason")]
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct Content {
    #[serde(default)]
    parts: Vec<Part>,
}

#[derive(Deserialize)]
struct Part {
    text: Option<String>,
    #[serde(default)]
    thought: bool,
}

#[derive(Deserialize)]
struct PromptFeedback {
    #[serde(rename = "blockReason")]
    block_reason: Option<String>,
}

#[derive(Deserialize)]
struct ErrorResponse {
    error: ErrorBody,
}

#[derive(Deserialize)]
struct ErrorBody {
    message: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    const MODEL_PATH: &str = "/models/modelo-teste:generateContent";

    fn provider(server: &MockServer, key: Option<&str>) -> GeminiProvider {
        GeminiProvider::with_base_url(
            key.map(String::from),
            Some("modelo-teste".into()),
            server.uri(),
        )
    }

    #[tokio::test]
    async fn devolve_o_texto_corrigido() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(MODEL_PATH))
            .and(header("x-goog-api-key", "chave-teste"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "candidates": [{
                    "content": { "parts": [
                        { "text": "pensando...", "thought": true },
                        { "text": "  Olá, tudo bem?  " }
                    ]},
                    "finishReason": "STOP"
                }]
            })))
            .mount(&server)
            .await;

        let result = provider(&server, Some("chave-teste"))
            .rewrite("ola tudo bem", TextStyle::Grammar)
            .await;
        assert_eq!(result.unwrap(), "Olá, tudo bem?");
    }

    #[tokio::test]
    async fn sem_chave_nao_chama_a_api() {
        let server = MockServer::start().await;
        let result = provider(&server, None)
            .rewrite("texto", TextStyle::Grammar)
            .await;
        assert!(matches!(result, Err(AppError::MissingApiKey)));
    }

    #[tokio::test]
    async fn limite_gratuito_vira_rate_limited() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(429).set_body_json(json!({
                "error": { "code": 429, "message": "Resource exhausted", "status": "RESOURCE_EXHAUSTED" }
            })))
            .mount(&server)
            .await;

        let result = provider(&server, Some("k"))
            .rewrite("texto", TextStyle::Grammar)
            .await;
        assert!(matches!(result, Err(AppError::RateLimited)));
    }

    #[tokio::test]
    async fn chave_invalida_vira_invalid_api_key() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(400).set_body_json(json!({
                "error": { "code": 400, "message": "API key not valid. Please pass a valid API key.", "status": "INVALID_ARGUMENT" }
            })))
            .mount(&server)
            .await;

        let result = provider(&server, Some("k"))
            .rewrite("texto", TextStyle::Grammar)
            .await;
        assert!(matches!(result, Err(AppError::InvalidApiKey)));
    }

    #[tokio::test]
    async fn texto_bloqueado_vira_blocked() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "promptFeedback": { "blockReason": "SAFETY" }
            })))
            .mount(&server)
            .await;

        let result = provider(&server, Some("k"))
            .rewrite("texto", TextStyle::Grammar)
            .await;
        assert!(matches!(result, Err(AppError::Blocked(r)) if r == "SAFETY"));
    }

    #[tokio::test]
    async fn texto_longo_demais_e_recusado_antes_de_enviar() {
        let server = MockServer::start().await;
        let longo = "a".repeat(MAX_INPUT_CHARS + 1);
        let result = provider(&server, Some("k"))
            .rewrite(&longo, TextStyle::Grammar)
            .await;
        assert!(matches!(result, Err(AppError::TextTooLong(_, _))));
    }

    #[tokio::test]
    async fn sem_internet_vira_mensagem_simples() {
        // Porta fechada no próprio computador: a conexão é recusada na hora.
        let provider =
            GeminiProvider::with_base_url(Some("k".into()), None, "http://127.0.0.1:9".into());
        let result = provider.rewrite("texto", TextStyle::Grammar).await;
        assert!(matches!(result, Err(AppError::Network)));
        assert!(!AppError::Network.to_string().contains("http"));
    }
}
