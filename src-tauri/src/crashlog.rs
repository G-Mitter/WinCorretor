// src-tauri/src/crashlog.rs
//
// Na versão instalada não existe terminal: se o app quebrar, a mensagem some.
// Este módulo grava qualquer pânico em
// %LOCALAPPDATA%\com.guilherme-ti.text-corrector\crash.log para podermos investigar.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const APP_DIR: &str = "com.guilherme-ti.text-corrector";

pub fn log_path() -> Option<PathBuf> {
    let base = std::env::var_os("LOCALAPPDATA").or_else(|| std::env::var_os("HOME"))?;
    Some(PathBuf::from(base).join(APP_DIR).join("crash.log"))
}

pub fn install() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if let Some(path) = log_path() {
            if let Some(dir) = path.parent() {
                let _ = fs::create_dir_all(dir);
            }
            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
                let when = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let backtrace = std::backtrace::Backtrace::force_capture();
                let _ = writeln!(
                    file,
                    "--- pânico (unix {when}), versão {} ---\n{info}\n{backtrace}\n",
                    env!("CARGO_PKG_VERSION")
                );
            }
        }
        default_hook(info);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panico_fica_registrado_no_arquivo() {
        let dir = std::env::temp_dir().join(format!("wincorretor-crash-{}", std::process::id()));
        std::env::set_var("LOCALAPPDATA", &dir);
        install();

        let _ = std::panic::catch_unwind(|| panic!("falha de teste"));

        let log = fs::read_to_string(log_path().unwrap()).unwrap();
        assert!(log.contains("falha de teste"));
        let _ = fs::remove_dir_all(dir);
    }
}
