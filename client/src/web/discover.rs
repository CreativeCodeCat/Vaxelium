//! Finding servers on the local network is not possible from a browser, so the web client finds none.

/// A server that answered a discovery request.
#[derive(Debug, Clone)]
pub struct FoundServer {
    pub server_id: String,
    pub addr: String,
    pub name: String,
    pub description: String,
    pub icon: Option<String>,
    pub online: u32,
    pub public: bool,
    pub self_signed: bool,
}

pub fn start(_via: String, _events: crate::backend::EventSender) {}
