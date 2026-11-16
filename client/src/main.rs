//! Vaxelium desktop client, built with Iced. The networking and the account are in `backend.rs`;
//! the window is in `app.rs`, `update.rs` and `view/`.

mod app;
#[cfg(not(target_arch = "wasm32"))]
mod auth_client;
#[cfg(target_arch = "wasm32")]
#[path = "web/auth_client.rs"]
mod auth_client;
mod backend;
mod bio;
#[cfg(not(target_arch = "wasm32"))]
mod discord_login;
#[cfg(target_arch = "wasm32")]
#[path = "web/discord_login.rs"]
mod discord_login;
#[cfg(not(target_arch = "wasm32"))]
mod discover;
#[cfg(target_arch = "wasm32")]
#[path = "web/discover.rs"]
mod discover;
#[cfg(test)]
#[path = "tests/flows.rs"]
mod flows;
mod highlight;
mod icons;
#[cfg(not(target_arch = "wasm32"))]
mod keychain;
#[cfg(target_arch = "wasm32")]
#[path = "web/keychain.rs"]
mod keychain;
mod latex;
mod markdown;
mod model;
#[cfg(not(target_arch = "wasm32"))]
mod net;
#[cfg(target_arch = "wasm32")]
#[path = "web/net.rs"]
mod net;
mod prefs;
#[cfg(test)]
#[path = "tests/screens.rs"]
mod screens;
mod search;
mod theme;
mod tray;
#[cfg(not(target_arch = "wasm32"))]
mod trust;
#[cfg(target_arch = "wasm32")]
#[path = "web/trust.rs"]
mod trust;
mod update;
mod util;
mod view;
#[cfg(target_arch = "wasm32")]
mod web;

use std::collections::HashMap;
use std::sync::Mutex;

use iced::futures::{SinkExt, Stream};
use iced::{window, Font, Size, Task};

use app::{App, Message};
use backend::{Backend, Event};

/// The receiving end of the backend's events. Taken once by the subscription that feeds the app.
static EVENTS: Mutex<Option<backend::EventReceiver>> = Mutex::new(None);

/// Forwards the backend's events into the app's message loop, as they arrive.
pub fn events_stream() -> impl Stream<Item = Event> {
    iced::stream::channel(
        64,
        |mut output: iced::futures::channel::mpsc::Sender<Event>| async move {
            let rx = EVENTS.lock().unwrap_or_else(|e| e.into_inner()).take();
            let Some(mut rx) = rx else {
                return;
            };
            use iced::futures::StreamExt;
            while let Some(event) = rx.next().await {
                if output.send(event).await.is_err() {
                    break;
                }
            }
        },
    )
}

/// The emoji catalog for the picker: (category, name, base emoji), and the skin tone variants.
fn emoji_catalog() -> (Vec<(String, String, String)>, HashMap<String, [String; 5]>) {
    let emoji = common::emoji_data::EMOJI
        .iter()
        .map(|(c, n, b)| (c.to_string(), n.to_string(), b.to_string()))
        .collect();
    let skin = common::emoji_data::SKIN_TONES
        .iter()
        .map(|(base, variants)| (base.to_string(), variants.map(|v| v.to_string())))
        .collect();
    (emoji, skin)
}

/// Where the settings live: a folder in the user's config directory on the desktop. The browser keeps
/// them in the site's storage, so it has no folder.
#[cfg(not(target_arch = "wasm32"))]
fn config_dir() -> std::path::PathBuf {
    let config = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("com.creativecodecat.vaxelium");
    if let Err(e) = std::fs::create_dir_all(&config) {
        eprintln!("could not create {}: {e}", config.display());
    }
    trust::init(config.join("pins.json"));
    tray::start();
    config
}

#[cfg(target_arch = "wasm32")]
fn config_dir() -> std::path::PathBuf {
    console_error_panic_hook::set_once();
    std::path::PathBuf::new()
}

/// The web client can only reach the server that served it, so that server is always in its list.
#[cfg(target_arch = "wasm32")]
fn with_page_server(mut prefs: prefs::Prefs) -> prefs::Prefs {
    let host = web::page_host();
    prefs.servers.retain(|s| s.addr == host);
    if prefs.servers.is_empty() {
        prefs.servers.push(prefs::ServerPref {
            name: host.clone(),
            addr: host,
            id: None,
            icon: None,
            ignored: Vec::new(),
            protected: Vec::new(),
            linked: false,
        });
    }
    prefs
}

#[cfg(not(target_arch = "wasm32"))]
fn with_page_server(prefs: prefs::Prefs) -> prefs::Prefs {
    prefs
}

fn main() -> iced::Result {
    common::env::load();
    let config = config_dir();

    let (tx, rx) = backend::event_channel();
    *EVENTS.lock().unwrap_or_else(|e| e.into_inner()) = Some(rx);
    #[cfg(target_arch = "wasm32")]
    web::set_redraw(tx.clone());
    let backend = Backend::new(tx, config.clone());
    let prefs = with_page_server(prefs::Prefs::load(&config));
    let (emoji, skin) = emoji_catalog();

    // A daemon, not an application: the app keeps running with no window, so the tray can bring the
    // window back after it has been hidden.
    iced::daemon(
        move || {
            let app = App::new(backend.clone(), prefs.clone(), emoji.clone(), skin.clone());
            let (_, open) = window::open(main_window_settings());
            let task = Task::batch([open.map(app::Message::WindowOpened), restore_task(&app)]);
            (app, task)
        },
        App::update,
        view_window,
    )
    .title("Vaxelium")
    .subscription(App::subscription)
    .theme(|app: &App, _window: window::Id| theme::iced_theme(&app.prefs.theme))
    // Noto Sans has the small capitals some names use, and all the Latin letters. Open Sans lacked
    // the small capitals, so text containing them was drawn with the wrong glyphs.
    .font(include_bytes!("../ui/fonts/NotoSans-Subset.ttf").as_slice())
    // The styled letters some names use (𝙲𝚛𝚎𝚊𝚝𝚎, 𝐁𝐨𝐥𝐝, ℌ and the like). The browser has no system
    // fonts to fall back on, so without this such names are drawn as nothing there.
    .font(include_bytes!("../ui/fonts/NotoSansMath-Letters.ttf").as_slice())
    .default_font(Font::with_name("Noto Sans"))
    .run()
}

/// The main window's contents: the app's view, whichever window asks.
fn view_window(app: &App, _window: window::Id) -> iced::Element<'_, app::Message> {
    view::view(app)
}

/// How the main window opens, and again after it has been hidden to the tray.
pub fn main_window_settings() -> window::Settings {
    window::Settings {
        size: Size::new(1100.0, 700.0),
        min_size: Some(Size::new(820.0, 480.0)),
        decorations: false,
        icon: app_icon(),
        ..window::Settings::default()
    }
}

/// The window icon, from the app's icon file.
fn app_icon() -> Option<window::Icon> {
    let img = image::load_from_memory(include_bytes!("../icons/icon.png"))
        .ok()?
        .to_rgba8();
    let (w, h) = img.dimensions();
    window::icon::from_rgba(img.into_raw(), w, h).ok()
}

/// Checks a login saved from an earlier run, in the background.
fn restore_task(app: &App) -> Task<Message> {
    match app.prefs.session.clone() {
        Some(session) => {
            let backend = app.backend.clone();
            Task::perform(async move { backend.restore(&session.name).await }, Message::Restored)
        }
        None => Task::none(),
    }
}
