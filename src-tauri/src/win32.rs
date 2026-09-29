// src-tauri/src/win32.rs
//
// Chamadas diretas à API do Windows usadas na captura do texto.
// Tudo aqui é pequeno e sem estado; a lógica fica em `capture.rs`.

use std::thread::sleep;
use std::time::{Duration, Instant};

use windows_sys::Win32::System::DataExchange::GetClipboardSequenceNumber;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
    KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_C, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT, VK_V,
};
use windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

/// Modificadores que podem continuar pressionados logo após o atalho.
const HELD_KEYS: [VIRTUAL_KEY; 5] = [VK_CONTROL, VK_MENU, VK_SHIFT, VK_LWIN, VK_RWIN];

/// Espera o usuário soltar as teclas do atalho. Devolve `false` se o tempo acabar.
pub fn wait_keys_released(timeout: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if !HELD_KEYS.iter().any(|&vk| is_key_down(vk)) {
            return true;
        }
        sleep(Duration::from_millis(10));
    }
    false
}

fn is_key_down(vk: VIRTUAL_KEY) -> bool {
    // O bit mais alto indica tecla pressionada agora.
    unsafe { (GetAsyncKeyState(vk as i32) as u16 & 0x8000) != 0 }
}

/// Identificador da janela em primeiro plano (onde o texto está selecionado).
/// Guardado como número para poder circular entre threads.
pub fn foreground_window() -> isize {
    unsafe { GetForegroundWindow() as isize }
}

/// Contador do Windows que muda sempre que algo é copiado.
pub fn clipboard_sequence() -> u32 {
    unsafe { GetClipboardSequenceNumber() }
}

/// Simula Ctrl+C na janela ativa.
pub fn send_ctrl_c() -> bool {
    send_ctrl_plus(VK_C)
}

/// Simula Ctrl+V na janela ativa.
pub fn send_ctrl_v() -> bool {
    send_ctrl_plus(VK_V)
}

fn send_ctrl_plus(letter: VIRTUAL_KEY) -> bool {
    let inputs = [
        key(VK_CONTROL, 0),
        key(letter, 0),
        key(letter, KEYEVENTF_KEYUP),
        key(VK_CONTROL, KEYEVENTF_KEYUP),
    ];
    let sent = unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            std::mem::size_of::<INPUT>() as i32,
        )
    };
    sent as usize == inputs.len()
}

fn key(vk: VIRTUAL_KEY, flags: KEYBD_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}
