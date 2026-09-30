// src-tauri/src/state.rs
//
// Estado compartilhado, criado uma única vez na inicialização.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use crate::capture::Selection;
use crate::llm::{self, LlmProvider};
use crate::secrets::{self, ApiKey};
use crate::settings::Settings;

pub struct AppState {
    /// Provedor de IA atual. Trocado na hora quando o usuário muda chave ou modelo.
    llm: RwLock<Arc<dyn LlmProvider>>,
    settings: Mutex<Settings>,
    /// Onde as preferências são salvas (None nos testes).
    settings_path: Option<PathBuf>,
    /// Atalho pausado pelo menu da bandeja.
    paused: AtomicBool,
    /// Mensagem para o usuário quando o atalho global não pôde ser registrado.
    hotkey_error: Mutex<Option<String>>,
    /// Verdadeiro enquanto uma captura está em andamento.
    busy: AtomicBool,
    /// Correção aberta no popup: o texto capturado e, depois, o resultado da IA.
    session: Mutex<Option<Session>>,
    /// Contador para identificar cada sessão (evita resposta atrasada cair na sessão errada).
    next_session_id: AtomicU64,
}

#[derive(Clone)]
pub struct Session {
    pub id: u64,
    pub selection: Selection,
    pub result: Option<String>,
}

/// Enquanto existir, marca o app como ocupado. Ao sair de escopo, libera.
pub struct BusyGuard<'a>(&'a AtomicBool);

impl Drop for BusyGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

impl AppState {
    pub fn new(settings: Settings, settings_path: Option<PathBuf>) -> Self {
        let llm = build_llm(&settings);
        println!("Provedor de IA: {}", llm.describe());
        Self {
            llm: RwLock::new(llm),
            settings: Mutex::new(settings),
            settings_path,
            paused: AtomicBool::new(false),
            hotkey_error: Mutex::new(None),
            busy: AtomicBool::new(false),
            session: Mutex::new(None),
            next_session_id: AtomicU64::new(1),
        }
    }

    // ---------- IA ----------

    /// Provedor atual (cópia barata do ponteiro; não segura o lock durante a chamada).
    pub fn llm(&self) -> Arc<dyn LlmProvider> {
        self.llm
            .read()
            .map(|l| l.clone())
            .unwrap_or_else(|p| p.into_inner().clone())
    }

    /// Recria o provedor com as chaves e modelos atuais.
    pub fn rebuild_llm(&self) {
        let llm = build_llm(&self.settings());
        println!("Provedor de IA: {}", llm.describe());
        if let Ok(mut slot) = self.llm.write() {
            *slot = llm;
        }
    }

    // ---------- Preferências ----------

    pub fn settings(&self) -> Settings {
        self.settings.lock().map(|s| s.clone()).unwrap_or_default()
    }

    pub fn settings_path(&self) -> Option<&PathBuf> {
        self.settings_path.as_ref()
    }

    pub fn set_settings(&self, settings: Settings) {
        if let Ok(mut slot) = self.settings.lock() {
            *slot = settings;
        }
    }

    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::SeqCst)
    }

    pub fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::SeqCst);
    }

    pub fn set_hotkey_error(&self, message: Option<String>) {
        if let Ok(mut slot) = self.hotkey_error.lock() {
            *slot = message;
        }
    }

    pub fn hotkey_error(&self) -> Option<String> {
        self.hotkey_error.lock().ok().and_then(|slot| slot.clone())
    }

    // ---------- Sessão do popup ----------

    /// Abre uma sessão no popup com o texto capturado.
    pub fn start_session(&self, selection: Selection) {
        if let Ok(mut slot) = self.session.lock() {
            *slot = Some(Session {
                id: self.next_session_id.fetch_add(1, Ordering::SeqCst),
                selection,
                result: None,
            });
        }
    }

    pub fn has_session(&self) -> bool {
        self.session.lock().map(|s| s.is_some()).unwrap_or(false)
    }

    /// Cópia da sessão atual (para ler o texto sem segurar o lock durante a IA).
    pub fn session(&self) -> Option<Session> {
        self.session.lock().ok().and_then(|s| s.clone())
    }

    /// Guarda o resultado da IA, desde que a sessão ainda seja a mesma
    /// que fez o pedido. Devolve `false` se ela foi fechada ou trocada.
    pub fn set_session_result(&self, id: u64, result: String) -> bool {
        if let Ok(mut slot) = self.session.lock() {
            if let Some(session) = slot.as_mut().filter(|s| s.id == id) {
                session.result = Some(result);
                return true;
            }
        }
        false
    }

    /// Encerra a sessão e devolve o que havia nela.
    pub fn take_session(&self) -> Option<Session> {
        self.session.lock().ok().and_then(|mut s| s.take())
    }

    /// Começa uma captura, ou devolve `None` se já houver uma em andamento.
    pub fn try_begin_correction(&self) -> Option<BusyGuard<'_>> {
        if self.busy.swap(true, Ordering::SeqCst) {
            None
        } else {
            Some(BusyGuard(&self.busy))
        }
    }
}

fn build_llm(settings: &Settings) -> Arc<dyn LlmProvider> {
    let (groq_key, _) = secrets::resolve(ApiKey::Groq);
    let (gemini_key, _) = secrets::resolve(ApiKey::Gemini);
    llm::build(
        groq_key,
        gemini_key,
        &settings.groq_model,
        &settings.gemini_model,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_state() -> AppState {
        AppState::new(Settings::default(), None)
    }

    fn selection() -> Selection {
        Selection {
            text: "ola".into(),
            source_window: 42,
        }
    }

    #[test]
    fn so_uma_correcao_por_vez() {
        let state = test_state();
        let first = state.try_begin_correction();
        assert!(first.is_some());
        assert!(state.try_begin_correction().is_none());

        drop(first);
        assert!(state.try_begin_correction().is_some());
    }

    #[test]
    fn sessao_guarda_texto_e_resultado() {
        let state = test_state();
        assert!(!state.has_session());

        state.start_session(selection());
        let id = state.session().expect("sessão aberta").id;
        assert!(state.set_session_result(id, "Olá".into()));

        let session = state.take_session().expect("sessão aberta");
        assert_eq!(session.selection.text, "ola");
        assert_eq!(session.result.as_deref(), Some("Olá"));
        assert!(!state.has_session());
    }

    #[test]
    fn resposta_atrasada_nao_entra_em_outra_sessao() {
        let state = test_state();
        state.start_session(selection());
        let old_id = state.session().unwrap().id;
        state.take_session(); // usuário cancelou
        state.start_session(selection()); // e abriu de novo

        assert!(!state.set_session_result(old_id, "atrasada".into()));
        assert_eq!(state.session().unwrap().result, None);
    }

    #[test]
    fn preferencias_podem_ser_trocadas() {
        let state = test_state();
        let mut settings = state.settings();
        settings.shortcut = "Ctrl+Alt+K".into();
        state.set_settings(settings);
        assert_eq!(state.settings().shortcut, "Ctrl+Alt+K");
    }
}
