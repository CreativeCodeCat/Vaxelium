//! Requests to the auth service from the browser. The browser cannot reach the auth service, so each
//! request goes to the chat server's WebSocket as `{"auth": request}`, and the server passes it on.

use std::cell::RefCell;
use std::rc::Rc;

use common::{AuthReply, AuthRequest};
use iced::futures::channel::oneshot;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{MessageEvent, WebSocket};

/// The auth service the chat server uses; the web client cannot name another.
pub fn auth_addr() -> String {
    "the chat server".into()
}

/// Whether the auth service answers right now, asked through the chat server.
pub async fn reachable() -> bool {
    request(AuthRequest::Verify { token: String::new() }).await.is_ok()
}

/// Sends one request and returns the reply. A failure to reach the auth service is an `Err`; the auth
/// service turning the request down is an `AuthReply::Error`.
pub async fn request(req: AuthRequest) -> Result<AuthReply, String> {
    let socket = WebSocket::new(&crate::web::socket_url()).map_err(|e| format!("could not connect: {e:?}"))?;
    let line = serde_json::json!({ "auth": req }).to_string();
    let (done, answer) = oneshot::channel::<Result<AuthReply, String>>();
    let done = Rc::new(RefCell::new(Some(done)));
    let finish = {
        let done = done.clone();
        move |result: Result<AuthReply, String>| {
            if let Some(done) = done.borrow_mut().take() {
                let _ = done.send(result);
            }
        }
    };

    let opened = {
        let socket = socket.clone();
        Closure::<dyn FnMut()>::new(move || {
            let _ = socket.send_with_str(&line);
        })
    };
    let message = {
        let finish = finish.clone();
        Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
            let Some(text) = event.data().as_string() else { return };
            let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
                return;
            };
            if let Some(reply) = value.get("auth") {
                finish(serde_json::from_value(reply.clone()).map_err(|e| format!("bad reply from auth service: {e}")));
            } else if let Some(text) = value.get("auth_unavailable").and_then(|v| v.as_str()) {
                finish(Err(text.to_string()));
            }
        })
    };
    let failed = {
        let finish = finish.clone();
        Closure::<dyn FnMut()>::new(move || {
            finish(Err(
                "Could not reach the server. Check your connection and try again.".into()
            ))
        })
    };
    socket.set_onopen(Some(opened.as_ref().unchecked_ref()));
    socket.set_onmessage(Some(message.as_ref().unchecked_ref()));
    socket.set_onerror(Some(failed.as_ref().unchecked_ref()));
    socket.set_onclose(Some(failed.as_ref().unchecked_ref()));

    let result = answer.await.unwrap_or_else(|_| Err("the request was dropped".into()));
    socket.set_onclose(None);
    socket.set_onerror(None);
    socket.set_onmessage(None);
    let _ = socket.close();
    drop((opened, message, failed));
    result
}
