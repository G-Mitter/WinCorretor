// src-tauri/src/corrector.rs

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum TextStyle {
    Grammar,
    Professional,
    Polite,
    Casual,
}

impl TextStyle {
    pub fn instruction(&self) -> &'static str {
        match self {
            TextStyle::Grammar => "Corrija apenas erros ortográficos e gramaticais do texto a seguir, mantendo o idioma original. Retorne APENAS o texto corrigido.",
            TextStyle::Professional => "Reescreva o texto a seguir com um tom estritamente profissional, claro e corporativo. Retorne APENAS o texto reescrito.",
            TextStyle::Polite => "Reescreva o texto a seguir tornando-o extremamente educado e cordial. Retorne APENAS o texto reescrito.",
            TextStyle::Casual => "Reescreva o texto a seguir de forma despojada e informal para redes sociais. Retorne APENAS o texto reescrito.",
        }
    }
}

pub struct CorrectorService {
    client: Client,
}

impl CorrectorService {
    pub fn new() -> Self {
        Self {
            // Timeout defensivo de 10s para evitar travamento de recursos
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("Falha ao construir o cliente HTTP"),
        }
    }

    /// Método que se comunica com a API da IA (Exemplo usando uma estrutura genérica de LLM)
    pub async fn process_text(&self, text: &str, style: &TextStyle, api_key: &str) -> Result<String, String> {
        let system_prompt = style.instruction();
        
        // Exemplo genérico apontando para a API do Google Gemini (ou OpenAI)
        // Você pode adaptar a URL e o payload conforme a LLM escolhida.
        let payload = json!({
            "contents": [{
                "parts": [
                    {"text": system_prompt},
                    {"text": format!("Texto: {}", text)}
                ]
            }]
        });

        // Simulação de chamada HTTP (Substitua pela URL real da API em produção)
        // Retornamos um mock formatado para testarmos a arquitetura localmente agora
        let mock_response = format!("[{}]: {}", match style {
            TextStyle::Grammar => "Corrigido",
            TextStyle::Professional => "Profissional",
            TextStyle::Polite => "Educado",
            TextStyle::Casual => "Despojado",
        }, text);

        Ok(mock_response)
    }
}

// Exemplo de Teste Unitário (pode ser adicionado ao final do corrector.rs)
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instruction_generation() {
        let style = TextStyle::Professional;
        assert!(style.instruction().contains("estritamente profissional"));
    }
}