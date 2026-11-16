//! The saved login token, kept in the system keychain (Keychain on macOS, Credential Manager on
//! Windows, Secret Service on Linux) rather than in the client's data file.
//!
//! The keychain library waits on its own async runtime, which panics on a thread that is already
//! running one, as the async login code does. So each call is made on a thread of its own.

const SERVICE: &str = "Vaxelium";

/// Runs keychain work on its own thread and waits for it.
fn apart<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::spawn(work).join().expect("keychain thread panicked")
}

/// Saves the token for an account, replacing any earlier one.
pub fn store(name: &str, token: &str) -> Result<(), String> {
    let (name, token) = (name.to_string(), token.to_string());
    apart(move || {
        keyring::Entry::new(SERVICE, &name)
            .and_then(|entry| entry.set_password(&token))
            .map_err(|e| e.to_string())
    })
}

/// The saved token for an account, if the keychain has one.
pub fn load(name: &str) -> Option<String> {
    let name = name.to_string();
    apart(move || keyring::Entry::new(SERVICE, &name).ok()?.get_password().ok())
}

/// Removes the saved token for an account. Does nothing if there is none.
pub fn forget(name: &str) {
    let name = name.to_string();
    apart(move || {
        if let Ok(entry) = keyring::Entry::new(SERVICE, &name) {
            let _ = entry.delete_credential();
        }
    })
}

#[cfg(test)]
#[path = "tests/keychain.rs"]
mod tests;
