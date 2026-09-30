// src-tauri/src/secrets.rs
//
// Chaves de API guardadas no Gerenciador de Credenciais do Windows
// (Painel de Controle → Gerenciador de Credenciais → Credenciais do Windows,
// entradas "WinCorretor"). Nunca são gravadas em arquivo nem enviadas à tela.
//
// Ordem de busca: cofre do Windows → variável de ambiente / .env (desenvolvimento).

use keyring::Entry;
use serde::Serialize;

use crate::error::{AppError, AppResult};

const SERVICE: &str = "WinCorretor";

#[derive(Debug, Clone, Copy)]
pub enum ApiKey {
    Groq,
    Gemini,
}

impl ApiKey {
    fn account(self) -> &'static str {
        match self {
            ApiKey::Groq => "groq_api_key",
            ApiKey::Gemini => "gemini_api_key",
        }
    }

    fn env_var(self) -> &'static str {
        match self {
            ApiKey::Groq => "GROQ_API_KEY",
            ApiKey::Gemini => "GEMINI_API_KEY",
        }
    }
}

/// De onde veio a chave (mostrado na tela, sem revelar o valor).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum KeySource {
    Vault,
    EnvFile,
    Missing,
}

/// Busca a chave e informa de onde ela veio.
pub fn resolve(key: ApiKey) -> (Option<String>, KeySource) {
    if let Some(value) = read_vault(key) {
        return (Some(value), KeySource::Vault);
    }
    match std::env::var(key.env_var())
        .ok()
        .map(|v| v.trim().to_string())
    {
        Some(value) if !value.is_empty() => (Some(value), KeySource::EnvFile),
        _ => (None, KeySource::Missing),
    }
}

pub fn store(key: ApiKey, value: &str) -> AppResult<()> {
    entry(key)?
        .set_password(value.trim())
        .map_err(|e| AppError::Settings(format!("não foi possível salvar a chave: {e}")))
}

pub fn remove(key: ApiKey) -> AppResult<()> {
    match entry(key)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(AppError::Settings(format!(
            "não foi possível remover a chave: {e}"
        ))),
    }
}

fn read_vault(key: ApiKey) -> Option<String> {
    entry(key)
        .ok()?
        .get_password()
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn entry(key: ApiKey) -> AppResult<Entry> {
    Entry::new(SERVICE, key.account()).map_err(|e| AppError::Settings(e.to_string()))
}
