// src-tauri/src/llm/fallback.rs
//
// Provedor "com reserva": tenta o principal e, se ele falhar por qualquer
// motivo que não seja do próprio texto, tenta o secundário.

use std::time::Instant;

use super::LlmProvider;
use crate::error::{AppError, AppResult};
use crate::prompts::TextStyle;

pub struct FallbackProvider {
    primary: Box<dyn LlmProvider>,
    secondary: Box<dyn LlmProvider>,
}

impl FallbackProvider {
    pub fn new(primary: Box<dyn LlmProvider>, secondary: Box<dyn LlmProvider>) -> Self {
        Self { primary, secondary }
    }
}

#[async_trait::async_trait]
impl LlmProvider for FallbackProvider {
    async fn rewrite(&self, text: &str, style: TextStyle) -> AppResult<String> {
        let started = Instant::now();
        match self.primary.rewrite(text, style).await {
            Ok(result) => Ok(result),
            // Texto grande demais falharia também na reserva.
            Err(err @ AppError::TextTooLong(..)) => Err(err),
            Err(err) => {
                eprintln!(
                    "{} falhou após {} ms ({err}). Tentando {}...",
                    self.primary.describe(),
                    started.elapsed().as_millis(),
                    self.secondary.describe()
                );
                self.secondary.rewrite(text, style).await
            }
        }
    }

    async fn warm_up(&self) {
        // Aquece o principal primeiro; a reserva em seguida, para ficar pronta se precisar.
        // Roda em segundo plano enquanto o usuário escolhe o tom.
        self.primary.warm_up().await;
        self.secondary.warm_up().await;
    }

    fn describe(&self) -> String {
        format!(
            "{} com reserva {}",
            self.primary.describe(),
            self.secondary.describe()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixed(Result<&'static str, fn() -> AppError>);

    #[async_trait::async_trait]
    impl LlmProvider for Fixed {
        async fn rewrite(&self, _text: &str, _style: TextStyle) -> AppResult<String> {
            match &self.0 {
                Ok(text) => Ok(text.to_string()),
                Err(make) => Err(make()),
            }
        }
        fn describe(&self) -> String {
            "Fixo".into()
        }
    }

    fn run(provider: FallbackProvider) -> AppResult<String> {
        tauri::async_runtime::block_on(provider.rewrite("texto", TextStyle::Grammar))
    }

    #[test]
    fn usa_o_principal_quando_ele_funciona() {
        let p = FallbackProvider::new(
            Box::new(Fixed(Ok("principal"))),
            Box::new(Fixed(Ok("reserva"))),
        );
        assert_eq!(run(p).unwrap(), "principal");
    }

    #[test]
    fn usa_a_reserva_quando_o_principal_falha() {
        let p = FallbackProvider::new(
            Box::new(Fixed(Err(|| AppError::RateLimited))),
            Box::new(Fixed(Ok("reserva"))),
        );
        assert_eq!(run(p).unwrap(), "reserva");
    }

    #[test]
    fn texto_longo_nao_vai_para_a_reserva() {
        let p = FallbackProvider::new(
            Box::new(Fixed(Err(|| AppError::TextTooLong(9_000, 8_000)))),
            Box::new(Fixed(Ok("reserva"))),
        );
        assert!(matches!(run(p), Err(AppError::TextTooLong(..))));
    }
}
