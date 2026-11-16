//! The system tray icon. Clicking it, or choosing "Show Vaxelium", brings the window back; "Quit"
//! closes the app. It uses the StatusNotifierItem protocol, which desktops such as KDE, and GNOME with
//! the AppIndicator extension, show. Other platforms have no tray icon yet.

use std::sync::{mpsc, Mutex};

use iced::futures::{SinkExt, Stream};

use crate::app::Message;

/// What the tray icon asks the app to do.
#[derive(Debug, Clone, Copy)]
pub enum TrayEvent {
    /// Hide the window if it is showing, or show it if it is hidden.
    Toggle,
    Quit,
}

static TRAY_EVENTS: Mutex<Option<mpsc::Receiver<TrayEvent>>> = Mutex::new(None);
/// The running tray icon, kept so its menu can say Show or Hide as the window changes.
#[cfg(target_os = "linux")]
static TRAY: Mutex<Option<ksni::blocking::Handle<Tray>>> = Mutex::new(None);

/// Tells the tray whether the window is showing, so its menu offers Hide or Show.
#[cfg(target_os = "linux")]
pub fn set_shown(shown: bool) {
    if let Some(handle) = TRAY.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
        handle.update(|tray| tray.shown = shown);
    }
}

#[cfg(not(target_os = "linux"))]
pub fn set_shown(_shown: bool) {}

/// Starts the tray icon. A desktop without a tray host simply gets no icon.
#[cfg(target_os = "linux")]
pub fn start() {
    use ksni::blocking::TrayMethods;

    let (tx, rx) = mpsc::channel();
    *TRAY_EVENTS.lock().unwrap_or_else(|e| e.into_inner()) = Some(rx);
    let Some(icon) = icon_pixmap() else {
        return;
    };
    let tray = Tray {
        events: tx,
        icon,
        shown: true,
    };
    match tray.spawn() {
        Ok(handle) => *TRAY.lock().unwrap_or_else(|e| e.into_inner()) = Some(handle),
        Err(e) => eprintln!("no system tray icon: {e}"),
    }
}

#[cfg(not(target_os = "linux"))]
pub fn start() {}

/// The tray's messages, as app messages, for the app's subscription.
pub fn tray_stream() -> impl Stream<Item = Message> {
    iced::stream::channel(
        8,
        |mut output: iced::futures::channel::mpsc::Sender<Message>| async move {
            let rx = TRAY_EVENTS.lock().unwrap_or_else(|e| e.into_inner()).take();
            let Some(rx) = rx else {
                return;
            };
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
                let message = match event {
                    TrayEvent::Toggle => Message::TrayToggle,
                    TrayEvent::Quit => Message::TrayQuit,
                };
                if output.send(message).await.is_err() {
                    break;
                }
            }
        },
    )
}

#[cfg(target_os = "linux")]
struct Tray {
    events: mpsc::Sender<TrayEvent>,
    icon: Vec<ksni::Icon>,
    /// True while the window is showing.
    shown: bool,
}

#[cfg(target_os = "linux")]
impl ksni::Tray for Tray {
    fn id(&self) -> String {
        "com.creativecodecat.vaxelium".into()
    }

    fn title(&self) -> String {
        "Vaxelium".into()
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        self.icon.clone()
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        let _ = self.events.send(TrayEvent::Toggle);
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::StandardItem;
        vec![
            StandardItem {
                label: if self.shown { "Hide Vaxelium" } else { "Show Vaxelium" }.into(),
                activate: Box::new(|tray: &mut Self| {
                    let _ = tray.events.send(TrayEvent::Toggle);
                }),
                ..Default::default()
            }
            .into(),
            ksni::MenuItem::Separator,
            StandardItem {
                label: "Quit".into(),
                activate: Box::new(|tray: &mut Self| {
                    let _ = tray.events.send(TrayEvent::Quit);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

/// The app icon as the tray's picture: ARGB32, most significant byte first, as the protocol asks.
#[cfg(target_os = "linux")]
fn icon_pixmap() -> Option<Vec<ksni::Icon>> {
    let img = image::load_from_memory(include_bytes!("../icons/tray.png"))
        .ok()?
        .to_rgba8();
    let size = 48;
    let img = image::imageops::resize(&img, size, size, image::imageops::FilterType::Lanczos3);
    let mut data = Vec::with_capacity((size * size * 4) as usize);
    for px in img.pixels() {
        let [r, g, b, a] = px.0;
        data.extend_from_slice(&[a, r, g, b]);
    }
    Some(vec![ksni::Icon {
        width: size as i32,
        height: size as i32,
        data,
    }])
}
