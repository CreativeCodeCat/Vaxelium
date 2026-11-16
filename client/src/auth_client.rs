//! Requests to the auth service (register, log in, log out and the rest), over TLS. Each runs on a
//! background thread, so the app can wait for it without blocking.
//! Set the `VAXELIUM_AUTH` environment variable to use a different auth service.

use std::io::{BufRead, BufReader, Write};
use std::time::Duration;

use common::{AuthReply, AuthRequest, DEFAULT_AUTH_ADDR};

use crate::trust;

pub fn auth_addr() -> String {
    std::env::var("VAXELIUM_AUTH").unwrap_or_else(|_| DEFAULT_AUTH_ADDR.to_string())
}

/// Whether the auth service accepts a connection right now. Used to hide the offline banner again.
pub async fn reachable() -> bool {
    crate::app::offload(|| common::tls::connect_blocking(&auth_addr(), Duration::from_secs(3)).is_ok()).await
}

/// Sends one request and returns the reply.
pub async fn request(req: AuthRequest) -> Result<AuthReply, String> {
    crate::app::offload(move || request_blocking(&req)).await
}

fn request_blocking(req: &AuthRequest) -> Result<AuthReply, String> {
    let addr = auth_addr();
    let (mut stream, fingerprint) =
        common::tls::connect_blocking(&addr, Duration::from_secs(4)).map_err(|e| connect_error_message(&e))?;
    // Check the certificate before sending anything, such as a password.
    trust::check(&format!("auth:{addr}"), &fingerprint)?;
    stream
        .sock
        .set_read_timeout(Some(Duration::from_secs(8)))
        .map_err(|e| e.to_string())?;

    let mut line = serde_json::to_string(req).map_err(|e| e.to_string())?;
    line.push('\n');
    stream.write_all(line.as_bytes()).map_err(|e| e.to_string())?;

    let mut reply = String::new();
    BufReader::new(stream)
        .read_line(&mut reply)
        .map_err(|e| e.to_string())?;
    serde_json::from_str(&reply).map_err(|e| format!("bad reply from auth service: {e}"))
}

/// Turns a connection error into a message that says what to do about it.
fn connect_error_message(e: &std::io::Error) -> String {
    match e.kind() {
        std::io::ErrorKind::ConnectionRefused => {
            "The account service is offline right now. Try again in a little while.".into()
        }
        std::io::ErrorKind::TimedOut => {
            "The account service stopped responding. Check your connection and try again.".into()
        }
        _ => "Could not reach the account service. Check your connection and try again.".into(),
    }
}
