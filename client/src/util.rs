//! Small helpers shared by the views: names and their colours, times, and the emoji artwork path.

use chrono::{DateTime, Datelike, Local, NaiveDate, TimeZone};
use iced::Color;

use iced::Padding;

use crate::theme::Pal;

/// Padding for top, right, bottom and left, in pixels.
pub fn pad(top: f32, right: f32, bottom: f32, left: f32) -> Padding {
    Padding {
        top,
        right,
        bottom,
        left,
    }
}

/// Padding for vertical and horizontal, in pixels.
pub fn pad2(vertical: f32, horizontal: f32) -> Padding {
    Padding {
        top: vertical,
        right: horizontal,
        bottom: vertical,
        left: horizontal,
    }
}

/// The first letter of a name, upper case.
pub fn initial(name: &str) -> String {
    name.chars()
        .next()
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_else(|| "?".into())
}

/// A stable colour for a name, so the same person always looks the same.
pub fn name_color(pal: &Pal, name: &str) -> Color {
    let mut hash: u32 = 5381;
    for b in name.bytes() {
        hash = hash.wrapping_mul(33) ^ u32::from(b);
    }
    pal.avatars[(hash % 8) as usize]
}

/// Where a saved login is kept, for the words beside "Keep me logged in".
pub const ON_DEVICE: &str = if cfg!(target_arch = "wasm32") {
    "on this browser"
} else {
    "on this computer"
};

/// Waits for `duration` without blocking the app.
#[cfg(not(target_arch = "wasm32"))]
pub async fn sleep(duration: std::time::Duration) {
    crate::app::offload(move || std::thread::sleep(duration)).await
}

/// Waits for `duration` without blocking the app, on the browser's timer.
#[cfg(target_arch = "wasm32")]
pub async fn sleep(duration: std::time::Duration) {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        if let Some(window) = web_sys::window() {
            let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, duration.as_millis() as i32);
        }
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

/// The current time, in Unix seconds.
pub fn now() -> u64 {
    web_time::SystemTime::now()
        .duration_since(web_time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// A join date such as "3 March 2024", or "Unknown" when the account's creation time is not known.
pub fn joined_label(ts: u64) -> String {
    if ts == 0 {
        return "Unknown".into();
    }
    local(ts).format("%-d %B %Y").to_string()
}

/// Opens a web address in the browser. Nothing is run through a shell, so the address is passed as it is.
pub fn open_url(url: &str) {
    #[cfg(target_os = "windows")]
    let result = std::process::Command::new("cmd").args(["/C", "start", "", url]).spawn();
    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open").arg(url).spawn();
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let result = std::process::Command::new("xdg-open").arg(url).spawn();
    let _ = result;
}

/// The local time for a Unix timestamp in seconds.
fn local(ts: u64) -> DateTime<Local> {
    Local
        .timestamp_opt(ts as i64, 0)
        .earliest()
        .unwrap_or_else(|| Local.timestamp_opt(0, 0).unwrap())
}

/// Short time, e.g. "14:05", in local time.
pub fn time_label(ts: u64) -> String {
    local(ts).format("%H:%M").to_string()
}

/// Full stamp next to an author, e.g. "Today at 14:05".
pub fn stamp_label(ts: u64, now: u64) -> String {
    let time = time_label(ts);
    let day = local_day(ts);
    let today = local_day(now);
    if day == today {
        format!("Today at {time}")
    } else if day + 1 == today {
        format!("Yesterday at {time}")
    } else {
        local(ts).format("%m/%d/%Y ").to_string() + &time
    }
}

/// Date divider label: "Today", "Yesterday", or the full date.
pub fn day_label(ts: u64, now: u64) -> String {
    let day = local_day(ts);
    let today = local_day(now);
    if day == today {
        "Today".into()
    } else if day + 1 == today {
        "Yesterday".into()
    } else {
        local(ts).format("%A, %B %-d, %Y").to_string()
    }
}

/// The local calendar day of a timestamp, as days since 1970-01-01. Used to compare dates.
pub fn local_day(ts: u64) -> i64 {
    local(ts).date_naive().num_days_from_ce() as i64
}

/// The local calendar day of a timestamp, for filters written as YYYY-MM-DD.
pub fn day_of(date: NaiveDate) -> i64 {
    date.num_days_from_ce() as i64
}

/// Path to the Twemoji artwork for an emoji, under the emoji folder of the app's assets.
#[cfg(not(target_arch = "wasm32"))]
pub fn emoji_path(emoji: &str) -> std::path::PathBuf {
    std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/ui/emoji")).join(emoji_file(emoji))
}

/// The file name of an emoji's artwork, such as `1f44d.svg` for 👍.
pub fn emoji_file(emoji: &str) -> String {
    let code: Vec<String> = emoji
        .chars()
        .filter(|&c| c as u32 != 0xFE0F)
        .map(|c| format!("{:x}", c as u32))
        .collect();
    format!("{}.svg", code.join("-"))
}

/// The colour of a `#rrggbb` string, or `None` when it is empty or not in that form.
pub fn hex_color(hex: &str) -> Option<Color> {
    let digits = hex.strip_prefix('#')?;
    if digits.len() != 6 {
        return None;
    }
    let value = u32::from_str_radix(digits, 16).ok()?;
    Some(Color::from_rgb8((value >> 16) as u8, (value >> 8) as u8, value as u8))
}

/// A name as plain lower-case letters, for sorting: styled Unicode letters such as 𝙲𝚛𝚎𝚊𝚝𝚎 (bold,
/// italic, monospace and the like) and full-width letters count as the plain letters they look like.
pub fn sort_key(name: &str) -> String {
    name.chars().map(plain_letter).collect::<String>().to_lowercase()
}

/// The plain letter or digit a styled one stands for, or the character as it is.
fn plain_letter(c: char) -> char {
    let code = c as u32;
    let from = |base: u32, offset: u32| char::from_u32(base + offset).unwrap_or(c);
    match code {
        // The mathematical letters come in sets of 52: A to Z, then a to z.
        0x1D400..=0x1D6A3 => {
            let i = (code - 0x1D400) % 52;
            if i < 26 {
                from('A' as u32, i)
            } else {
                from('a' as u32, i - 26)
            }
        }
        // The mathematical digits come in sets of 10.
        0x1D7CE..=0x1D7FF => from('0' as u32, (code - 0x1D7CE) % 10),
        0xFF21..=0xFF3A => from('A' as u32, code - 0xFF21),
        0xFF41..=0xFF5A => from('a' as u32, code - 0xFF41),
        0xFF10..=0xFF19 => from('0' as u32, code - 0xFF10),
        _ => c,
    }
}

#[cfg(test)]
#[path = "tests/util.rs"]
mod tests;
