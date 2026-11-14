//! Vaxelium desktop client, built with Iced. The networking and the account are in `backend.rs`;
//! the window is in `app.rs`, `update.rs` and `view/`.

mod app;
mod auth_client;
mod backend;
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

    let (tx, rx) = mpsc::channel();
    *EVENTS.lock().unwrap_or_else(|e| e.into_inner()) = Some(rx);
    let backend = Backend::new(tx, config.clone());
    let prefs = prefs::Prefs::load(&config);
    let (emoji, skin) = emoji_catalog();

    iced::application(
        move || {
            let app = App::new(backend.clone(), prefs.clone(), emoji.clone(), skin.clone());
            let task = restore_task(&app);
            (app, task)
        },
        App::update,
        view::view,
    )
    .title("Vaxelium")
    .subscription(App::subscription)
    .theme(|app: &App| theme::iced_theme(&app.prefs.theme))
    .font(include_bytes!("../ui/fonts/OpenSans-Regular.ttf").as_slice())
    .default_font(Font::with_name("Open Sans"))
    .window(window::Settings {
        size: Size::new(1100.0, 700.0),
        min_size: Some(Size::new(820.0, 480.0)),
        decorations: false,
        icon: app_icon(),
        ..window::Settings::default()
    })
    .run()
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
