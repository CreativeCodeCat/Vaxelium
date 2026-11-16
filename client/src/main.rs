//! Vaxelium desktop client, built with Iced. The networking and the account are in `backend.rs`;
//! the window is in `app.rs`, `update.rs` and `view/`.

mod app;
mod auth_client;
mod backend;
mod bio;
mod discord_login;
mod discover;
mod icons;
mod keychain;
mod model;
mod net;
mod prefs;
mod search;
#[cfg(test)]
mod flows;
#[cfg(test)]
mod screens;
mod theme;
mod tray;
mod trust;
mod update;
mod util;
mod view;

use std::collections::HashMap;
use std::sync::{mpsc, Mutex};

use iced::futures::{SinkExt, Stream};
use iced::{window, Font, Size, Task};

use app::{offload, App, Message};
use backend::{Backend, Event};

/// The receiving end of the backend's events. Taken once by the subscription that feeds the app.
static EVENTS: Mutex<Option<mpsc::Receiver<Event>>> = Mutex::new(None);

/// Forwards the backend's events into the app's message loop, as they arrive.
pub fn events_stream() -> impl Stream<Item = Event> {
    iced::stream::channel(64, |mut output: iced::futures::channel::mpsc::Sender<Event>| async move {
        let rx = EVENTS.lock().unwrap_or_else(|e| e.into_inner()).take();
        let Some(rx) = rx else {
            return;
        };
        // The receiver blocks, so it reads on its own thread and hands events to the async side.
        let (tx, mut from_thread) = iced::futures::channel::mpsc::unbounded();
        std::thread::spawn(move || {
            while let Ok(event) = rx.recv() {
                if tx.unbounded_send(event).is_err() {
                    break;
                }
            }
        });
        use iced::futures::StreamExt;
        while let Some(event) = from_thread.next().await {
            if output.send(event).await.is_err() {
                break;
            }
        }
    })
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

fn main() -> iced::Result {
    common::env::load();
    let config = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("com.creativecodecat.vaxelium");
    if let Err(e) = std::fs::create_dir_all(&config) {
        eprintln!("could not create {}: {e}", config.display());
    }
    trust::init(config.join("pins.json"));
    tray::start();

    let (tx, rx) = mpsc::channel();
    *EVENTS.lock().unwrap_or_else(|e| e.into_inner()) = Some(rx);
    let backend = Backend::new(tx, config.clone());
    let prefs = prefs::Prefs::load(&config);
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
    let img = image::load_from_memory(include_bytes!("../icons/icon.png")).ok()?.to_rgba8();
    let (w, h) = img.dimensions();
    window::icon::from_rgba(img.into_raw(), w, h).ok()
}

/// Checks a login saved from an earlier run, in the background.
fn restore_task(app: &App) -> Task<Message> {
    match app.prefs.session.clone() {
        Some(session) => {
            let backend = app.backend.clone();
            Task::perform(offload(move || backend.restore(&session.name)), Message::Restored)
        }
        None => Task::none(),
    }
}
