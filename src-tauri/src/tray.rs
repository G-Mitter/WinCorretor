// src-tauri/src/tray.rs
//
// Ícone na bandeja (perto do relógio). O app vive aqui: a janela principal
// fica escondida e só aparece quando o usuário pede.

use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Runtime};

use crate::hotkey;
use crate::notify;

const TRAY_ID: &str = "main";
const MENU_OPEN: &str = "open";
const MENU_PAUSE: &str = "pause";
const MENU_QUIT: &str = "quit";

pub fn create<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, MENU_OPEN, "Abrir WinCorretor", true, None::<&str>)?;
    let pause =
        CheckMenuItem::with_id(app, MENU_PAUSE, "Pausar atalho", true, false, None::<&str>)?;
    let quit = MenuItem::with_id(app, MENU_QUIT, "Sair", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&open, &pause, &separator, &quit])?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip(tooltip(false))
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            MENU_OPEN => show_main_window(app),
            MENU_PAUSE => {
                let paused = pause.is_checked().unwrap_or(false);
                set_paused(app, paused);
            }
            MENU_QUIT => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // Clique simples com o botão esquerdo abre a janela.
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

/// Mostra e traz para frente a janela principal.
pub fn show_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn set_paused<R: Runtime>(app: &AppHandle<R>, paused: bool) {
    let result = if paused {
        hotkey::unregister_default(app)
    } else {
        hotkey::register_default(app)
    };

    match result {
        Ok(()) => {
            if let Some(tray) = app.tray_by_id(TRAY_ID) {
                let _ = tray.set_tooltip(Some(tooltip(paused)));
            }
        }
        Err(err) => notify::show(app, &format!("Não foi possível alterar o atalho: {err}")),
    }
}

fn tooltip(paused: bool) -> String {
    if paused {
        "WinCorretor (atalho pausado)".to_string()
    } else {
        format!(
            "WinCorretor: selecione um texto e aperte {}",
            hotkey::DEFAULT_SHORTCUT
        )
    }
}
