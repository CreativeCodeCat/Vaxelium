//! Opens web pages in the browser, for logging in with Discord and for the legal pages.

use std::process::{Command, Stdio};

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
