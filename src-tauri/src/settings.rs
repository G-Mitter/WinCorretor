// src-tauri/src/settings.rs
//
// Preferências do usuário, salvas em JSON na pasta de configuração do app
// (%APPDATA%\com.guilherme-ti.text-corrector\settings.json).
// As chaves de API NÃO ficam aqui: vão para o cofre do Windows (secrets.rs).

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::llm::{gemini, groq};
use crate::prompts::TextStyle;

pub const DEFAULT_SHORTCUT: &str = "Ctrl+Alt+O";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// Atalho global, no formato "Ctrl+Alt+O".
    pub shortcut: String,
    /// Tom que já vem selecionado ao abrir o popup.
    pub default_style: TextStyle,
    pub groq_model: String,
    pub gemini_model: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            shortcut: DEFAULT_SHORTCUT.to_string(),
            default_style: TextStyle::Grammar,
            groq_model: groq::DEFAULT_MODEL.to_string(),
            gemini_model: gemini::DEFAULT_MODEL.to_string(),
        }
    }
}

/// Lê as preferências. Arquivo ausente ou inválido → valores padrão (o app sempre abre).
pub fn load(path: &Path) -> Settings {
    match fs::read_to_string(path) {
        Ok(json) => serde_json::from_str(&json).unwrap_or_else(|err| {
            eprintln!(
                "Configurações inválidas em {}: {err}. Usando padrão.",
                path.display()
            );
            Settings::default()
        }),
        Err(_) => Settings::default(),
    }
}

pub fn save(path: &Path, settings: &Settings) -> AppResult<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| AppError::Settings(e.to_string()))?;
    }
    let json =
        serde_json::to_string_pretty(settings).map_err(|e| AppError::Settings(e.to_string()))?;
    fs::write(path, json).map_err(|e| AppError::Settings(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("wincorretor-teste-{}", std::process::id()));
        dir.join(name)
    }

    #[test]
    fn salva_e_le_de_volta() {
        let path = temp_file("salva.json");
        let settings = Settings {
            shortcut: "Ctrl+Alt+K".into(),
            default_style: TextStyle::Professional,
            ..Settings::default()
        };
        save(&path, &settings).unwrap();
        assert_eq!(load(&path), settings);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn arquivo_ausente_ou_quebrado_usa_padrao() {
        assert_eq!(load(&temp_file("nao-existe.json")), Settings::default());

        let path = temp_file("quebrado.json");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "{ isso não é json").unwrap();
        assert_eq!(load(&path), Settings::default());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn campo_faltando_recebe_valor_padrao() {
        // Arquivo de uma versão antiga, sem o campo defaultStyle.
        let settings: Settings = serde_json::from_str(r#"{ "shortcut": "Ctrl+Alt+K" }"#).unwrap();
        assert_eq!(settings.shortcut, "Ctrl+Alt+K");
        assert_eq!(settings.default_style, TextStyle::Grammar);
    }
}
