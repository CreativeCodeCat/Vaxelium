//! Small helpers shared by the views: names and their colours, times, and the emoji artwork path.

use chrono::{DateTime, Datelike, Local, NaiveDate, TimeZone};
use iced::Color;

use iced::Padding;

use crate::theme::Pal;

/// Padding for top, right, bottom and left, in pixels.
pub fn pad(top: f32, right: f32, bottom: f32, left: f32) -> Padding {
    Padding { top, right, bottom, left }
}

/// Padding for vertical and horizontal, in pixels.
pub fn pad2(vertical: f32, horizontal: f32) -> Padding {
    Padding { top: vertical, right: horizontal, bottom: vertical, left: horizontal }
}

/// The first letter of a name, upper case.
pub fn initial(name: &str) -> String {
    name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_else(|| "?".into())
}

/// A stable colour for a name, so the same person always looks the same.
pub fn name_color(pal: &Pal, name: &str) -> Color {
    let mut hash: u32 = 5381;
    for b in name.bytes() {
        hash = hash.wrapping_mul(33) ^ u32::from(b);
    }
    pal.avatars[(hash % 8) as usize]
}

/// The current time, in Unix seconds.
pub fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// The local time for a Unix timestamp in seconds.
fn local(ts: u64) -> DateTime<Local> {
    Local.timestamp_opt(ts as i64, 0).earliest().unwrap_or_else(|| Local.timestamp_opt(0, 0).unwrap())
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
pub fn emoji_path(emoji: &str) -> std::path::PathBuf {
    let code: Vec<String> = emoji
        .chars()
        .filter(|&c| c as u32 != 0xFE0F)
        .map(|c| format!("{:x}", c as u32))
        .collect();
    std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/ui/emoji")).join(format!("{}.svg", code.join("-")))
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
