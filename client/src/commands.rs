//! The commands the page calls. Each is a thin wrapper over `state::AppState`. Anything that waits
//! on the network runs on a worker thread, so the window stays responsive.

use std::path::Path;

use base64::Engine;
use common::ClientMsg;
use serde_json::Value;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

use crate::state::{AppState, CmdError, Profile, Restored, SavedLogin};
use crate::{auth_client, discover, trust};

/// Runs blocking work on a worker thread and returns its result.
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, CmdError> + Send + 'static,
) -> Result<T, CmdError> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|e| CmdError::new(e.to_string()))?
}

fn prefs_path(state: &AppState) -> std::path::PathBuf {
    state.config_dir.join("prefs.json")
}

/// The saved settings (servers, theme, skin tone and the remembered login). Null when none are saved.
#[tauri::command]
pub fn prefs_load(state: State<'_, AppState>) -> Value {
    std::fs::read_to_string(prefs_path(&state))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or(Value::Null)
}

#[tauri::command]
pub fn prefs_save(state: State<'_, AppState>, prefs: Value) -> Result<(), String> {
    let text = serde_json::to_string_pretty(&prefs).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&state.config_dir).map_err(|e| e.to_string())?;
    std::fs::write(prefs_path(&state), text).map_err(|e| e.to_string())
}

/// The emoji for reactions, grouped by category, and the skin tone variants of the emoji that have them.
#[tauri::command]
pub fn emoji_catalog() -> Value {
    let emoji: Vec<(&str, &str, &str)> = common::emoji_data::EMOJI.to_vec();
    let skin: Vec<(&str, [&str; 5])> = common::emoji_data::SKIN_TONES.to_vec();
    serde_json::json!({ "emoji": emoji, "skin": skin })
}

#[tauri::command]
pub async fn restore_session(state: State<'_, AppState>, saved: Option<SavedLogin>) -> Result<Restored, CmdError> {
    let state = state.inner().clone();
    let Some(saved) = saved else {
        return Ok(Restored { user: None, auth_down: false, error: None, notice: None });
    };
    Ok(state.restore(saved).await)
}

#[tauri::command]
pub async fn login(
    state: State<'_, AppState>,
    email: String,
    password: String,
    name: Option<String>,
    remember: bool,
) -> Result<String, CmdError> {
    state.inner().clone().login(email, password, name, remember).await
}

#[tauri::command]
pub async fn logout(state: State<'_, AppState>) -> Result<(), CmdError> {
    state.inner().clone().logout().await;
    Ok(())
}

#[tauri::command]
pub async fn profile(state: State<'_, AppState>) -> Result<Profile, CmdError> {
    state.inner().clone().profile().await
}

#[tauri::command]
pub async fn update_profile(
    state: State<'_, AppState>,
    name: Option<String>,
    username: Option<String>,
) -> Result<Profile, CmdError> {
    state.inner().clone().update_profile(name, username).await
}

#[tauri::command]
pub async fn change_email(state: State<'_, AppState>, password: String, new_email: String) -> Result<String, CmdError> {
    state.inner().clone().change_email(password, new_email).await
}

#[tauri::command]
pub async fn connect_main(state: State<'_, AppState>, addr: String) -> Result<u64, CmdError> {
    let state = state.inner().clone();
    blocking(move || state.connect_main(&addr)).await
}

#[tauri::command]
pub fn disconnect_main(state: State<'_, AppState>) {
    state.disconnect_main();
}

/// Sends one protocol message (the JSON shape from `common::ClientMsg`) to the open server.
#[tauri::command]
pub fn send_main(state: State<'_, AppState>, msg: Value) -> Result<(), CmdError> {
    let msg: ClientMsg = serde_json::from_value(msg).map_err(|e| CmdError::new(e.to_string()))?;
    state.send_main(msg)
}

/// Sends one protocol message over the direct message connection to `addr`.
#[tauri::command]
pub fn send_dm(state: State<'_, AppState>, addr: String, msg: Value) -> Result<(), CmdError> {
    let msg: ClientMsg = serde_json::from_value(msg).map_err(|e| CmdError::new(e.to_string()))?;
    state.send_dm(&addr, msg)
}

#[tauri::command]
pub fn dm_connect_all(state: State<'_, AppState>, addrs: Vec<String>) {
    state.dm_connect_all(addrs);
}

#[tauri::command]
pub fn dm_disconnect(state: State<'_, AppState>, addr: String) {
    state.dm_disconnect(&addr);
}

/// Starts looking for servers through `via`. Each one found arrives as a `discovered` event.
#[tauri::command]
pub fn discover(app: AppHandle, via: String) {
    discover::start(via, app);
}

/// Lets the owner pick a picture. Returns it as a base64 PNG of 128x128, or None if cancelled.
#[tauri::command]
pub async fn pick_server_icon(app: AppHandle) -> Result<Option<String>, CmdError> {
    blocking(move || {
        let Some(file) = app
            .dialog()
            .file()
            .add_filter("Images", &["png", "jpg", "jpeg"])
            .blocking_pick_file()
        else {
            return Ok(None);
        };
        let path = file.into_path().map_err(|e| CmdError::new(e.to_string()))?;
        icon_png_base64(&path).map(Some).map_err(CmdError::new)
    })
    .await
}

/// Resizes an image file to a 128x128 PNG and returns it base64-encoded.
fn icon_png_base64(path: &Path) -> Result<String, String> {
    let img = image::open(path).map_err(|e| format!("Could not read that image: {e}"))?;
    let img = img.resize_to_fill(128, 128, image::imageops::FilterType::Lanczos3);
    let mut png = std::io::Cursor::new(Vec::new());
    img.write_to(&mut png, image::ImageFormat::Png)
        .map_err(|e| format!("Could not convert that image: {e}"))?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(png.into_inner());
    if encoded.len() > common::MAX_SERVER_ICON_LEN {
        return Err("That image is too large even after shrinking it.".into());
    }
    Ok(encoded)
}

/// Forgets the certificate remembered for the auth service, so the next login trusts the one it sees.
#[tauri::command]
pub fn trust_forget_auth() {
    trust::forget(&format!("auth:{}", auth_client::auth_addr()));
}

/// Forgets the certificate remembered for a server (`kind` "server") or the auth service ("auth"),
/// so the next connection trusts the one it sees.
#[tauri::command]
pub fn trust_forget(kind: String, addr: String) {
    trust::forget(&format!("{kind}:{addr}"));
}
