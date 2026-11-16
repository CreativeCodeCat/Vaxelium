//! Opens the Discord login page from the browser, in a new tab.

pub fn open_browser(url: &str) -> std::io::Result<()> {
    let window = web_sys::window().ok_or_else(|| std::io::Error::other("no browser window"))?;
    window
        .open_with_url_and_target(url, "_blank")
        .map(|_| ())
        .map_err(|_| std::io::Error::other("the browser blocked the new tab"))
}
