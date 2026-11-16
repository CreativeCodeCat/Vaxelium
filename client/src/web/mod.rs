//! Shared pieces of the browser build: where the server is, and a wrapper for browser objects.
//!
//! The browser runs this app on one thread, so browser objects (which cannot move between threads)
//! may be kept in the app's shared, thread-safe state. `Local` marks them as such.

/// The WebSocket address of the server that served this page. Everything the web client does goes
/// through it, including logging in.
pub fn socket_url() -> String {
    let location = web_sys::window().map(|w| w.location());
    let host = location.as_ref().and_then(|l| l.host().ok()).unwrap_or_default();
    let secure = location.and_then(|l| l.protocol().ok()).is_some_and(|p| p == "https:");
    format!("{}://{host}/ws", if secure { "wss" } else { "ws" })
}

/// The address the web client shows for its server: the page's own host and port.
pub fn page_host() -> String {
    web_sys::window()
        .and_then(|w| w.location().host().ok())
        .unwrap_or_default()
}

/// The browser's storage for this site, if it allows it.
pub fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

/// A browser object kept in thread-safe state. The browser build has a single thread, so it is never
/// actually shared between threads.
pub struct Local<T>(pub T);

// SAFETY: wasm32-unknown-unknown without the atomics feature runs on one thread only.
unsafe impl<T> Send for Local<T> {}
unsafe impl<T> Sync for Local<T> {}

thread_local! {
    /// Emoji artwork by file name: `Some` once fetched, `None` while it is being fetched or if the
    /// server has none.
    static EMOJI: std::cell::RefCell<std::collections::HashMap<String, Option<iced::widget::svg::Handle>>> =
        Default::default();
    /// Where to say that new artwork has arrived, so the window is drawn again.
    static REDRAW: std::cell::RefCell<Option<crate::backend::EventSender>> = const { std::cell::RefCell::new(None) };
}

/// Lets fetched artwork ask for the window to be drawn again.
pub fn set_redraw(events: crate::backend::EventSender) {
    REDRAW.with(|r| *r.borrow_mut() = Some(events));
}

/// An emoji's artwork, if it has been fetched. The first call for one starts fetching it.
pub fn emoji(file: &str) -> Option<iced::widget::svg::Handle> {
    let known = EMOJI.with(|cache| cache.borrow().get(file).cloned());
    match known {
        Some(handle) => handle,
        None => {
            EMOJI.with(|cache| cache.borrow_mut().insert(file.to_string(), None));
            let file = file.to_string();
            wasm_bindgen_futures::spawn_local(async move {
                if let Some(bytes) = fetch(&format!("/emoji/{file}")).await {
                    let handle = iced::widget::svg::Handle::from_memory(bytes);
                    EMOJI.with(|cache| cache.borrow_mut().insert(file, Some(handle)));
                    REDRAW.with(|r| {
                        if let Some(events) = r.borrow().as_ref() {
                            let _ = events.unbounded_send(crate::backend::Event::Redraw);
                        }
                    });
                }
            });
            None
        }
    }
}

/// A file from the server that served this page, or None if it could not be fetched.
async fn fetch(path: &str) -> Option<Vec<u8>> {
    use wasm_bindgen::JsCast;
    let window = web_sys::window()?;
    let response = wasm_bindgen_futures::JsFuture::from(window.fetch_with_str(path))
        .await
        .ok()?;
    let response: web_sys::Response = response.dyn_into().ok()?;
    if !response.ok() {
        return None;
    }
    let buffer = wasm_bindgen_futures::JsFuture::from(response.array_buffer().ok()?)
        .await
        .ok()?;
    Some(js_sys::Uint8Array::new(&buffer).to_vec())
}
