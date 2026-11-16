//! The browser side of logging in with Discord. The browser opens Discord; Discord sends the browser
//! back to a listener on this computer with a code. The code is then handed to the auth service.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// What one request to the listener means.
#[derive(Debug, PartialEq)]
pub enum Step {
    /// Not the callback, or a callback for some other login. Keep waiting.
    Ignore,
    /// Discord's code for this login.
    Code(String),
    /// Discord said no, or the callback was broken. The login ends with this message.
    Failed(String),
}

/// Opens the address in the default browser.
pub fn open_browser(url: &str) -> std::io::Result<()> {
    let mut command = if cfg!(target_os = "windows") {
        // `start` through cmd would split the address at its `&`s, so use the URL handler directly.
        let mut c = Command::new("rundll32");
        c.args(["url.dll,FileProtocolHandler", url]);
        c
    } else if cfg!(target_os = "macos") {
        let mut c = Command::new("open");
        c.arg(url);
        c
    } else {
        let mut c = Command::new("xdg-open");
        c.arg(url);
        c
    };
    // The browser inherits these streams, so its own log messages would print into this terminal.
    command.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    let mut child = command.spawn()?;
    // Wait for the opener in the background so it does not linger as a finished process.
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

/// Waits for Discord to send the browser back with a code for this login, and answers the browser.
pub fn wait_for_code(listener: &TcpListener, state: &str, timeout: Duration) -> Result<String, String> {
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let deadline = Instant::now() + timeout;
    loop {
        if Instant::now() >= deadline {
            return Err("The Discord login took too long. Try again.".into());
        }
        let socket = match listener.accept() {
            Ok((socket, _)) => socket,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }
            Err(e) => return Err(e.to_string()),
        };
        let step = answer(socket, state);
        match step {
            Step::Ignore => continue,
            Step::Code(code) => return Ok(code),
            Step::Failed(text) => return Err(text),
        }
    }
}

/// Reads one request, replies to the browser, and says what the request meant.
fn answer(mut socket: TcpStream, state: &str) -> Step {
    // The accepted socket can inherit the listener's non-blocking mode on some systems.
    let _ = socket.set_nonblocking(false);
    let _ = socket.set_read_timeout(Some(Duration::from_secs(5)));
    let mut line = String::new();
    if BufReader::new(&socket).read_line(&mut line).is_err() {
        return Step::Ignore;
    }
    let step = parse_request_line(&line, state);
    let (status, body) = match &step {
        Step::Code(_) => ("200 OK", "<p>You are logged in to Vaxelium. You can close this tab.</p>".to_string()),
        Step::Failed(text) => ("200 OK", format!("<p>{}</p><p>You can close this tab and try again in Vaxelium.</p>", escape(text))),
        Step::Ignore => ("404 Not Found", "<p>Not found.</p>".to_string()),
    };
    let page = format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>Vaxelium</title></head>\
         <body style=\"font-family:sans-serif;padding:2em\">{body}</body></html>"
    );
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{page}",
        page.len()
    );
    let _ = socket.write_all(response.as_bytes());
    let _ = socket.flush();
    // Drain whatever else the browser sent, so it does not see a reset.
    let mut rest = [0u8; 512];
    let _ = socket.read(&mut rest);
    step
}

/// Reads a request line such as `GET /callback?code=abc&state=xyz HTTP/1.1`.
pub fn parse_request_line(line: &str, state: &str) -> Step {
    let mut parts = line.split_whitespace();
    let (Some("GET"), Some(target)) = (parts.next(), parts.next()) else {
        return Step::Ignore;
    };
    let Some(query) = target.strip_prefix("/callback?").or_else(|| (target == "/callback").then_some("")) else {
        return Step::Ignore;
    };
    let mut code = None;
    let mut got_state = None;
    let mut error = false;
    for pair in query.split('&') {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        match key {
            "code" => code = Some(value.to_string()),
            "state" => got_state = Some(value.to_string()),
            "error" => error = true,
            _ => {}
        }
    }
    // A callback that does not carry this login's state is not ours, so it is ignored rather than
    // allowed to end the login.
    if got_state.as_deref() != Some(state) {
        return Step::Ignore;
    }
    if error {
        return Step::Failed("The Discord login was cancelled.".into());
    }
    match code.filter(|c| !c.is_empty()) {
        Some(code) => Step::Code(code),
        None => Step::Failed("Discord did not send a login code. Try again.".into()),
    }
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_callback_with_the_right_state_gives_the_code() {
        assert_eq!(
            parse_request_line("GET /callback?code=abc123&state=s1 HTTP/1.1", "s1"),
            Step::Code("abc123".into())
        );
        // The state can come before the code.
        assert_eq!(
            parse_request_line("GET /callback?state=s1&code=abc123 HTTP/1.1", "s1"),
            Step::Code("abc123".into())
        );
    }

    #[test]
    fn a_callback_with_the_wrong_state_is_ignored() {
        assert_eq!(parse_request_line("GET /callback?code=abc&state=other HTTP/1.1", "s1"), Step::Ignore);
        assert_eq!(parse_request_line("GET /callback?code=abc HTTP/1.1", "s1"), Step::Ignore);
    }

    #[test]
    fn other_requests_are_ignored() {
        assert_eq!(parse_request_line("GET /favicon.ico HTTP/1.1", "s1"), Step::Ignore);
        assert_eq!(parse_request_line("POST /callback?code=x&state=s1 HTTP/1.1", "s1"), Step::Ignore);
    }

    #[test]
    fn a_cancelled_login_ends_with_a_message() {
        assert!(matches!(
            parse_request_line("GET /callback?error=access_denied&state=s1 HTTP/1.1", "s1"),
            Step::Failed(_)
        ));
    }
}
