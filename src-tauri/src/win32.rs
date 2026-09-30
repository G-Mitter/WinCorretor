// src-tauri/src/win32.rs
//
// Chamadas diretas à API do Windows usadas na captura do texto.
// Tudo aqui é pequeno e sem estado; a lógica fica em `capture.rs`.

use std::thread::sleep;
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::GlobalFree;
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardSequenceNumber, OpenClipboard,
    RegisterClipboardFormatW, SetClipboardData,
};
use windows_sys::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
    KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_C, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT, VK_V,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, SetForegroundWindow};

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

/// Traz de volta para a frente a janela onde o texto foi selecionado.
/// Funciona porque, neste momento, o WinCorretor (popup) é a janela ativa.
pub fn focus_window(hwnd: isize) -> bool {
    if foreground_window() == hwnd {
        return true;
    }
    unsafe { SetForegroundWindow(hwnd as HWND) != 0 }
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

/// Formato padrão de texto Unicode do Windows (CF_UNICODETEXT).
const CF_UNICODETEXT: u32 = 13;

/// Formatos que pedem ao Windows para NÃO guardar o conteúdo no histórico
/// do Win+V nem sincronizar com a nuvem.
/// Documentação: "Cloud Clipboard and Clipboard History Formats" (learn.microsoft.com).
const PRIVATE_FORMATS: [&str; 3] = [
    "ExcludeClipboardContentFromMonitorProcessing",
    "CanIncludeInClipboardHistory",
    "CanUploadToCloudClipboard",
];

/// Coloca texto no clipboard sem aparecer no histórico do Win+V.
/// Usado para as trocas temporárias do app (resultado antes do Ctrl+V e
/// restauração do conteúdo original). Devolve `false` se não conseguiu.
pub fn set_clipboard_text_private(text: &str) -> bool {
    unsafe {
        if !open_clipboard() {
            return false;
        }
        EmptyClipboard();

        let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        let ok = set_data(CF_UNICODETEXT, wide.as_ptr().cast(), wide.len() * 2);

        // Valor 0 (DWORD) = "não incluir"; para o formato Exclude, qualquer dado basta.
        let zero: u32 = 0;
        for name in PRIVATE_FORMATS {
            let wide_name: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
            let format = RegisterClipboardFormatW(wide_name.as_ptr());
            if format != 0 {
                set_data(format, (&zero as *const u32).cast(), 4);
            }
        }

        CloseClipboard();
        ok
    }
}

/// O clipboard pode estar aberto por outro programa por alguns milissegundos.
unsafe fn open_clipboard() -> bool {
    for _ in 0..10 {
        if OpenClipboard(std::ptr::null_mut()) != 0 {
            return true;
        }
        sleep(Duration::from_millis(10));
    }
    false
}

/// Copia `len` bytes para memória global e entrega ao clipboard (que passa a ser o dono).
unsafe fn set_data(format: u32, data: *const u8, len: usize) -> bool {
    let handle = GlobalAlloc(GMEM_MOVEABLE, len);
    if handle.is_null() {
        return false;
    }
    let target = GlobalLock(handle) as *mut u8;
    if target.is_null() {
        GlobalFree(handle);
        return false;
    }
    std::ptr::copy_nonoverlapping(data, target, len);
    GlobalUnlock(handle);

    if SetClipboardData(format, handle).is_null() {
        GlobalFree(handle);
        return false;
    }
    true
}
