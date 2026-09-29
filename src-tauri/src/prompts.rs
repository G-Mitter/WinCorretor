// src-tauri/src/prompts.rs
//
// Estilos de reescrita e as instruções enviadas para a IA.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TextStyle {
    Grammar,
    Professional,
    Polite,
    Casual,
}

impl TextStyle {
    /// Instrução de sistema para o estilo escolhido.
    // Usada pelo GeminiProvider na issue #5.
    #[allow(dead_code)]
    pub fn instruction(self) -> &'static str {
        match self {
            TextStyle::Grammar => "Corrija apenas erros ortográficos e gramaticais do texto a seguir, mantendo o idioma original. Retorne APENAS o texto corrigido.",
            TextStyle::Professional => "Reescreva o texto a seguir com um tom estritamente profissional, claro e corporativo. Retorne APENAS o texto reescrito.",
            TextStyle::Polite => "Reescreva o texto a seguir tornando-o extremamente educado e cordial. Retorne APENAS o texto reescrito.",
            TextStyle::Casual => "Reescreva o texto a seguir de forma despojada e informal. Retorne APENAS o texto reescrito.",
        }
    }

    /// Rótulo curto, usado em logs e no provedor simulado.
    pub fn label(self) -> &'static str {
        match self {
            TextStyle::Grammar => "Corrigido",
            TextStyle::Professional => "Profissional",
            TextStyle::Polite => "Educado",
            TextStyle::Casual => "Despojado",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn todo_estilo_pede_apenas_o_texto_final() {
        for style in [
            TextStyle::Grammar,
            TextStyle::Professional,
            TextStyle::Polite,
            TextStyle::Casual,
        ] {
            assert!(
                style.instruction().contains("APENAS"),
                "{style:?} sem a regra APENAS"
            );
        }
    }

    #[test]
    fn estilo_profissional_tem_instrucao_correta() {
        assert!(TextStyle::Professional
            .instruction()
            .contains("estritamente profissional"));
    }

    #[test]
    fn estilo_chega_do_front_em_camel_case() {
        let style: TextStyle = serde_json::from_str("\"professional\"").unwrap();
        assert_eq!(style, TextStyle::Professional);
    }
}
