//! The settings saved between runs: servers, theme, skin tone, the remembered login, panel widths
//! and the last channel per server. The file keeps the JSON layout the old client wrote.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ServerPref {
    pub name: String,
    pub addr: String,
    /// The server's own id, learned when it first answers. Stays the same when the address changes.
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    /// Account ids whose messages are hidden on this server only (see the member menu's Ignore).
    #[serde(default)]
    pub ignored: Vec<String>,
    /// Account ids with Bypass ignore (or who own) this server, as the server last said. Blocks do not
    /// apply to them here.
    #[serde(default)]
    pub protected: Vec<String>,
    /// True once another server has got in touch with this one, as the server last said. Shows a
    /// house beside its name.
    #[serde(default)]
    pub linked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionPref {
    pub name: String,
    #[serde(default)]
    pub server: Option<String>,
}

/// Widths of the side panels, in pixels.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Layout {
    pub channels: f32,
    pub dms: f32,
    pub members: f32,
}

impl Default for Layout {
    fn default() -> Self {
        Self {
            channels: 250.0,
            dms: 250.0,
            members: 250.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Prefs {
    pub servers: Vec<ServerPref>,
    pub theme: String,
    pub skin_tone: usize,
    pub session: Option<SessionPref>,
    pub layout: Layout,
    pub last_channels: HashMap<String, String>,
    /// Account ids blocked on every server: their messages and direct messages are hidden.
    #[serde(default)]
    pub blocked: Vec<String>,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            servers: Vec::new(),
            theme: "breeze-dark".into(),
            skin_tone: 0,
            session: None,
            layout: Layout::default(),
            last_channels: HashMap::new(),
            blocked: Vec::new(),
        }
    }
}

impl Prefs {
    /// Loads the saved settings, or the defaults when there are none or the file cannot be read.
    pub fn load(dir: &Path) -> Self {
        let mut prefs: Prefs = read(dir)
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        prefs.layout.channels = prefs.layout.channels.clamp(250.0, 420.0);
        prefs.layout.dms = prefs.layout.dms.clamp(250.0, 420.0);
        prefs.layout.members = prefs.layout.members.clamp(250.0, 420.0);
        if !crate::theme::IDS.contains(&prefs.theme.as_str()) {
            prefs.theme = "breeze-dark".into();
        }
        prefs
    }

    pub fn save(&self, dir: &Path) {
        let result = serde_json::to_string_pretty(self)
            .map_err(|e| e.to_string())
            .and_then(|text| write(dir, &text));
        if let Err(e) = result {
            eprintln!("could not save settings: {e}");
        }
    }
}

/// The saved settings: a file in the config folder on the desktop.
#[cfg(not(target_arch = "wasm32"))]
fn read(dir: &Path) -> Option<String> {
    std::fs::read_to_string(dir.join("prefs.json")).ok()
}

#[cfg(not(target_arch = "wasm32"))]
fn write(dir: &Path, text: &str) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("prefs.json"), text).map_err(|e| e.to_string())
}

/// The saved settings: the site's local storage in the browser.
#[cfg(target_arch = "wasm32")]
fn read(_dir: &Path) -> Option<String> {
    crate::web::storage()?.get_item("vaxelium-prefs").ok()?
}

#[cfg(target_arch = "wasm32")]
fn write(_dir: &Path, text: &str) -> Result<(), String> {
    let storage = crate::web::storage().ok_or("this browser does not allow saving settings")?;
    storage
        .set_item("vaxelium-prefs", text)
        .map_err(|_| "could not save settings".to_string())
}
