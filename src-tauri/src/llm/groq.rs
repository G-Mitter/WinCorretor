// src-tauri/src/llm/groq.rs
//
// Provedor Groq (camada gratuita), com API no formato da OpenAI.
// Escolhido como principal pela velocidade: ~1.000 tokens/s no gpt-oss-20b.
// Documentação: https://console.groq.com/docs/api-reference#chat-create

use std::time::Duration;

use reqwest::{Client, StatusCode};
use serde::Deserialize;
use serde_json::{json, Value};

use super::{http_client, network_error, warm_up_connection, LlmProvider, MAX_INPUT_CHARS};
use crate::error::{AppError, AppResult};
use crate::prompts::{self, TextStyle};

const DEFAULT_BASE_URL: &str = "https://api.groq.com/openai/v1";
/// Modelo de produção do plano gratuito. Pode ser trocado pela variável GROQ_MODEL.
pub const DEFAULT_MODEL: &str = "openai/gpt-oss-20b";
/// Tempo curto de propósito: se o Groq travar, a reserva (Gemini) assume logo.
const TIMEOUT: Duration = Duration::from_secs(6);

pub struct GroqProvider {
    client: Client,
    api_key: Option<String>,
    model: String,
    base_url: String,
}

impl GroqProvider {
    pub fn new(api_key: Option<String>, model: Option<String>) -> Self {
        Self::with_base_url(api_key, model, DEFAULT_BASE_URL.to_string())
    }

    fn with_base_url(api_key: Option<String>, model: Option<String>, base_url: String) -> Self {
        Self {
            client: http_client(TIMEOUT),
            api_key,
            model: model.unwrap_or_else(|| DEFAULT_MODEL.to_string()),
            base_url,
        }
    }

    fn request_body(&self, text: &str, style: TextStyle) -> Value {
        let mut body = json!({
            "model": self.model,
            "messages": [
                { "role": "system", "content": prompts::system_instruction(style) },
                { "role": "user", "content": prompts::user_content(text) }
            ],
            "temperature": 0.2,
            "max_completion_tokens": 4096
        });
        // Modelos gpt-oss "pensam" antes de responder: pouco raciocínio basta
        // para reescrever texto e deixa a resposta mais rápida.
        if self.model.starts_with("openai/gpt-oss") {
            body["reasoning_effort"] = json!("low");
            body["include_reasoning"] = json!(false);
        }
        body
    }
}

#[async_trait::async_trait]
impl LlmProvider for GroqProvider {
    async fn rewrite(&self, text: &str, style: TextStyle) -> AppResult<String> {
        let api_key = self.api_key.as_deref().ok_or(AppError::MissingApiKey)?;

        let length = text.chars().count();
        if length > MAX_INPUT_CHARS {
            return Err(AppError::TextTooLong(length, MAX_INPUT_CHARS));
        }

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .bearer_auth(api_key)
            .json(&self.request_body(text, style))
            .send()
            .await
            .map_err(network_error)?;

        let status = response.status();
        let raw = response.text().await.map_err(network_error)?;
        if !status.is_success() {
            return Err(map_http_error(status, &raw));
        }

        let parsed: ChatResponse = serde_json::from_str(&raw)
            .map_err(|e| AppError::Llm(format!("resposta inesperada do Groq: {e}")))?;
        extract_text(parsed)
    }

    async fn warm_up(&self) {
        warm_up_connection(&self.client, &self.base_url, "Groq").await;
    }

    fn describe(&self) -> String {
        format!("Groq ({})", self.model)
    }
}

fn map_http_error(status: StatusCode, raw: &str) -> AppError {
    let message = serde_json::from_str::<ErrorResponse>(raw)
        .map(|e| e.error.message)
        .unwrap_or_else(|_| raw.chars().take(200).collect());

    match status {
        StatusCode::TOO_MANY_REQUESTS => AppError::RateLimited,
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => AppError::InvalidApiKey,
        _ => AppError::Llm(format!("HTTP {}: {message}", status.as_u16())),
    }
}

fn extract_text(response: ChatResponse) -> AppResult<String> {
    let choice = response
        .choices
        .into_iter()
        .next()
        .ok_or_else(|| AppError::Llm("o Groq não devolveu nenhuma resposta".into()))?;

    let text = prompts::clean_output(choice.message.content.as_deref().unwrap_or_default());
    if text.is_empty() {
        let reason = choice.finish_reason.unwrap_or_else(|| "vazia".into());
        return Err(AppError::Blocked(reason));
    }
    Ok(text)
}

#[derive(Deserialize)]
struct ChatResponse {
    #[serde(default)]
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct Message {
    content: Option<String>,
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
    use wiremock::matchers::{body_partial_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn provider(server: &MockServer, model: &str) -> GroqProvider {
        GroqProvider::with_base_url(Some("chave-teste".into()), Some(model.into()), server.uri())
    }

    fn ok_body(text: &str) -> Value {
        json!({ "choices": [{ "message": { "role": "assistant", "content": text }, "finish_reason": "stop" }] })
    }

    #[tokio::test]
    async fn devolve_o_texto_e_pede_pouco_raciocinio_no_gpt_oss() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(header("authorization", "Bearer chave-teste"))
            .and(body_partial_json(json!({
                "model": "openai/gpt-oss-20b",
                "reasoning_effort": "low",
                "include_reasoning": false
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(ok_body(" Olá, tudo bem? ")))
            .mount(&server)
            .await;

        let result = provider(&server, "openai/gpt-oss-20b")
            .rewrite("ola tudo bem", TextStyle::Grammar)
            .await;
        assert_eq!(result.unwrap(), "Olá, tudo bem?");
    }

    #[test]
    fn outros_modelos_nao_recebem_parametros_de_raciocinio() {
        let groq = GroqProvider::new(Some("k".into()), Some("llama-3.3-70b-versatile".into()));
        let body = groq.request_body("texto", TextStyle::Grammar);
        assert!(body.get("reasoning_effort").is_none());
        assert!(body.get("include_reasoning").is_none());
    }

    #[tokio::test]
    async fn limite_gratuito_vira_rate_limited() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(429).set_body_json(json!({
                "error": { "message": "Rate limit reached", "type": "tokens" }
            })))
            .mount(&server)
            .await;

        let result = provider(&server, "openai/gpt-oss-20b")
            .rewrite("texto", TextStyle::Grammar)
            .await;
        assert!(matches!(result, Err(AppError::RateLimited)));
    }

    #[tokio::test]
    async fn chave_invalida_vira_invalid_api_key() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(401).set_body_json(json!({
                "error": { "message": "Invalid API Key", "type": "invalid_request_error" }
            })))
            .mount(&server)
            .await;

        let result = provider(&server, "openai/gpt-oss-20b")
            .rewrite("texto", TextStyle::Grammar)
            .await;
        assert!(matches!(result, Err(AppError::InvalidApiKey)));
    }
}
