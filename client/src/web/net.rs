//! Connections to the chat server in the browser, over the WebSocket the server offers to web clients.
//! Only the server that served the page can be reached.

use std::cell::RefCell;
use std::rc::Rc;

use common::{ClientMsg, ServerMsg};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{CloseEvent, MessageEvent, WebSocket};

use crate::web::Local;

pub enum NetEvent {
    Server(ServerMsg),
    Disconnected(String),
}

struct Shared {
    socket: WebSocket,
    /// Lines queued before the socket opened.
    pending: RefCell<Vec<String>>,
    /// The socket's handlers, kept alive for as long as the connection.
    handlers: RefCell<Vec<Box<dyn std::any::Any>>>,
}

pub struct Connection {
    shared: Local<Rc<Shared>>,
}

impl Connection {
    /// Sends a message, or queues it until the socket has opened. Fails once the connection has ended.
    pub fn send(&self, msg: ClientMsg) -> Result<(), ()> {
        let line = serde_json::to_string(&msg).map_err(|_| ())?;
        let shared = &self.shared.0;
        match shared.socket.ready_state() {
            WebSocket::CONNECTING => {
                shared.pending.borrow_mut().push(line);
                Ok(())
            }
            WebSocket::OPEN => shared.socket.send_with_str(&line).map_err(|_| ()),
            _ => Err(()),
        }
    }

    /// Sends one last message, then closes the connection.
    pub fn send_last(self, msg: ClientMsg) {
        let _ = self.send(msg);
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        let shared = &self.shared.0;
        shared.socket.set_onmessage(None);
        shared.socket.set_onclose(None);
        shared.socket.set_onopen(None);
        let _ = shared.socket.close();
        shared.handlers.borrow_mut().clear();
    }
}

/// Opens the server's WebSocket and sends `first` (the login) once it is open. What the server sends
/// is passed to `on_event`, ending with a `Disconnected` event.
pub fn connect(
    addr: &str,
    first: ClientMsg,
    on_event: impl FnMut(NetEvent) + Send + 'static,
) -> Result<Connection, String> {
    if addr != crate::web::page_host() {
        return Err("the web client can only reach the server it was opened from".into());
    }
    let socket = WebSocket::new(&crate::web::socket_url()).map_err(|e| format!("could not connect: {e:?}"))?;
    let first = serde_json::to_string(&first).map_err(|e| e.to_string())?;
    let shared = Rc::new(Shared {
        socket: socket.clone(),
        pending: RefCell::new(vec![first]),
        handlers: RefCell::new(Vec::new()),
    });
    let on_event = Rc::new(RefCell::new(on_event));

    let opened = {
        let shared = Rc::downgrade(&shared);
        Closure::<dyn FnMut()>::new(move || {
            if let Some(shared) = shared.upgrade() {
                for line in shared.pending.borrow_mut().drain(..) {
                    let _ = shared.socket.send_with_str(&line);
                }
            }
        })
    };
    let message = {
        let on_event = on_event.clone();
        Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
            let Some(text) = event.data().as_string() else { return };
            if let Ok(msg) = serde_json::from_str::<ServerMsg>(&text) {
                (on_event.borrow_mut())(NetEvent::Server(msg));
            }
        })
    };
    let closed = {
        let on_event = on_event.clone();
        Closure::<dyn FnMut(CloseEvent)>::new(move |event: CloseEvent| {
            let reason = match event.reason() {
                r if r.is_empty() => "connection closed by server".to_string(),
                r => r,
            };
            (on_event.borrow_mut())(NetEvent::Disconnected(reason));
        })
    };
    socket.set_onopen(Some(opened.as_ref().unchecked_ref()));
    socket.set_onmessage(Some(message.as_ref().unchecked_ref()));
    socket.set_onclose(Some(closed.as_ref().unchecked_ref()));
    shared.handlers.borrow_mut().extend([
        Box::new(opened) as Box<dyn std::any::Any>,
        Box::new(message),
        Box::new(closed),
    ]);
    Ok(Connection { shared: Local(shared) })
}
