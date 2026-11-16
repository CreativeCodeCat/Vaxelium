//! Saved logins in the browser, kept in the site's local storage in place of the system keychain.

fn key(name: &str) -> String {
    format!("vaxelium-login:{name}")
}

pub fn store(name: &str, token: &str) -> Result<(), String> {
    let storage = crate::web::storage().ok_or("this browser does not allow saving logins")?;
    storage
        .set_item(&key(name), token)
        .map_err(|_| "could not save the login".to_string())
}

pub fn load(name: &str) -> Option<String> {
    crate::web::storage()?.get_item(&key(name)).ok()?
}

pub fn forget(name: &str) {
    if let Some(storage) = crate::web::storage() {
        let _ = storage.remove_item(&key(name));
    }
}
