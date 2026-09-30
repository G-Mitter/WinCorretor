// src-tauri/src/commands.rs
//
// Comandos expostos ao front-end via `invoke`.

use std::time::Instant;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use crate::error::{AppError, AppResult};
use crate::flow;
use crate::hotkey;
use crate::prompts::TextStyle;
use crate::secrets::{self, ApiKey, KeySource};
use crate::settings::{self, Settings};
use crate::state::AppState;
use crate::tray;

/// Devolve a mensagem de erro do atalho global, se ele não pôde ser registrado.
#[tauri::command]
pub fn hotkey_error(state: State<'_, AppState>) -> Option<String> {
    state.hotkey_error()
}

// ---------- Configurações ----------

/// O que a tela de configurações mostra. As chaves nunca saem do Rust:
/// a tela só sabe se cada uma existe e de onde veio.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsView {
    #[serde(flatten)]
    settings: Settings,
    groq_key: KeySource,
    gemini_key: KeySource,
    provider: String,
}

/// O que a tela envia ao salvar. Para cada chave:
/// ausente = manter como está; texto vazio = remover; texto = salvar no cofre.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsInput {
    shortcut: String,
    default_style: TextStyle,
    groq_model: String,
    gemini_model: String,
    groq_key: Option<String>,
    gemini_key: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiTest {
    provider: String,
    millis: u128,
    sample: String,
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> SettingsView {
    view(&state)
}

#[tauri::command]
pub fn save_settings(app: AppHandle, input: SettingsInput) -> AppResult<SettingsView> {
    let state = app.state::<AppState>();
    let old = state.settings();

    let new = Settings {
        shortcut: hotkey::validate(&input.shortcut)?,
        default_style: input.default_style,
        groq_model: non_empty(input.groq_model, &Settings::default().groq_model),
        gemini_model: non_empty(input.gemini_model, &Settings::default().gemini_model),
    };

    // 1. Chaves no cofre do Windows (independem do resto).
    update_key(ApiKey::Groq, input.groq_key)?;
    update_key(ApiKey::Gemini, input.gemini_key)?;

    // 2. Atalho: se o novo estiver em uso por outro programa, para aqui e mantém o antigo.
    //    Se estiver pausado, só guarda: será registrado ao despausar.
    if !state.is_paused() {
        if state.hotkey_error().is_some() {
            // O atalho antigo nunca chegou a ser registrado.
            hotkey::register(&app, &new.shortcut)?;
        } else {
            hotkey::change(&app, &old.shortcut, &new.shortcut)?;
        }
    }
    state.set_hotkey_error(None);

    // 3. Preferências em disco.
    if let Some(path) = state.settings_path() {
        settings::save(path, &new)?;
    }
    state.set_settings(new);

    // 4. Recria o provedor de IA com as chaves e modelos novos.
    state.rebuild_llm();
    tray::refresh_tooltip(&app);

    Ok(view(&state))
}

/// Faz uma correção de teste para o usuário ver se a chave funciona e quanto demora.
#[tauri::command]
pub async fn test_ai(state: State<'_, AppState>) -> AppResult<AiTest> {
    let llm = state.llm();
    let started = Instant::now();
    let sample = llm
        .rewrite("isso e um teste do wincorretor", TextStyle::Grammar)
        .await?;
    Ok(AiTest {
        provider: llm.describe(),
        millis: started.elapsed().as_millis(),
        sample,
    })
}

fn view(state: &AppState) -> SettingsView {
    SettingsView {
        settings: state.settings(),
        groq_key: secrets::resolve(ApiKey::Groq).1,
        gemini_key: secrets::resolve(ApiKey::Gemini).1,
        provider: state.llm().describe(),
    }
}

fn update_key(key: ApiKey, change: Option<String>) -> AppResult<()> {
    match change.map(|v| v.trim().to_string()) {
        None => Ok(()),
        Some(v) if v.is_empty() => secrets::remove(key),
        Some(v) => secrets::store(key, &v),
    }
}

fn non_empty(value: String, fallback: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        fallback.to_string()
    } else {
        value.to_string()
    }
}

// ---------- Popup ----------

/// Popup: gera a prévia do texto no tom escolhido.
#[tauri::command]
pub async fn popup_rewrite(app: AppHandle, style: TextStyle) -> AppResult<String> {
    flow::rewrite(&app, style).await
}

/// Popup: aplica a prévia no lugar do texto selecionado.
#[tauri::command]
pub async fn popup_apply(app: AppHandle) -> AppResult<()> {
    tauri::async_runtime::spawn_blocking(move || flow::apply(&app))
        .await
        .map_err(|e| AppError::Popup(e.to_string()))?
}

/// Popup: só copia a prévia para o clipboard.
#[tauri::command]
pub async fn popup_copy(app: AppHandle) -> AppResult<()> {
    tauri::async_runtime::spawn_blocking(move || flow::copy(&app))
        .await
        .map_err(|e| AppError::Popup(e.to_string()))?
}

/// Popup: fecha sem alterar nada e devolve o foco à janela de origem.
#[tauri::command]
pub async fn popup_cancel(app: AppHandle) {
    let _ = tauri::async_runtime::spawn_blocking(move || flow::cancel(&app, true)).await;
}
