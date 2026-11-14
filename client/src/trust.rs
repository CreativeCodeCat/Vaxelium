//! Remembered server certificates (trust on first use). Saved to a file between runs, so a server
//! that later presents a different certificate is refused instead of silently trusted.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

/// Certificate fingerprints by address, for example "server:127.0.0.1:7878" or "auth:127.0.0.1:7900".
static PINS: Mutex<BTreeMap<String, String>> = Mutex::new(BTreeMap::new());
/// Where the certificates are saved. Set once at startup by `init`.
static FILE: OnceLock<PathBuf> = OnceLock::new();

/// Loads the remembered certificates from `path` and saves future changes there.
pub fn init(path: PathBuf) {
    if let Some(pins) = std::fs::read_to_string(&path)
        .ok()
        .and_then(|text| serde_json::from_str::<BTreeMap<String, String>>(&text).ok())
    {
        *PINS.lock().unwrap() = pins;
    }
    let _ = FILE.set(path);
}

fn save(pins: &BTreeMap<String, String>) {
    let Some(path) = FILE.get() else { return };
    let Ok(text) = serde_json::to_string_pretty(pins) else { return };
    if let Err(e) = std::fs::write(path, text) {
        eprintln!("could not save remembered certificates: {e}");
    }
}

/// Forgets the certificate remembered for `key`, so the next connection remembers the new one.
pub fn forget(key: &str) {
    let mut pins = PINS.lock().unwrap();
    if pins.remove(key).is_some() {
        save(&pins);
    }
}

/// Checks the certificate fingerprint seen for `key`. The first one seen is remembered.
pub fn check(key: &str, fingerprint: &str) -> Result<(), String> {
    let mut pins = PINS.lock().unwrap();
    let first = common::tls::check_pin(pins.get(key).map(String::as_str), fingerprint)?;
    if first {
        pins.insert(key.to_string(), fingerprint.to_string());
        save(&pins);
    }
    Ok(())
}
