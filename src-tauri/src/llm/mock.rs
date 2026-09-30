// src-tauri/src/llm/mock.rs
//
// Provedor simulado: não chama nenhuma API. Usado até a issue do Gemini
// e, depois, nos testes.

use super::LlmProvider;
use crate::error::AppResult;
use crate::prompts::TextStyle;

pub struct MockProvider;

#[async_trait::async_trait]
impl LlmProvider for MockProvider {
    async fn rewrite(&self, text: &str, style: TextStyle) -> AppResult<String> {
        Ok(format!("[{}]: {}", style.label(), text))
    }

    fn describe(&self) -> String {
        "Simulado".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_marca_o_texto_com_o_estilo() {
        let result =
            tauri::async_runtime::block_on(MockProvider.rewrite("ola mundo", TextStyle::Grammar));
        assert_eq!(result.unwrap(), "[Corrigido]: ola mundo");
    }
}
