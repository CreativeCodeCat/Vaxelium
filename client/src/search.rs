//! The message search: free text plus the filters from:, in:, has:, mentions:, before:, after: and
//! during:. Messages carry no files or embeds, so only `has:link` can match.

use chrono::NaiveDate;
use common::ChatMessage;

use crate::util::{day_of, local_day};

#[derive(Debug, Default, PartialEq)]
pub struct Search {
    /// The free text, lower case, words joined by single spaces.
    pub text: String,
    pub from: Vec<String>,
    pub mentions: Vec<String>,
    pub channels: Vec<String>,
    pub has: Vec<String>,
    /// Dates from the before:, after: and during: filters. None when the date is not valid.
    pub before: Vec<Option<NaiveDate>>,
    pub after: Vec<Option<NaiveDate>>,
    pub during: Vec<Option<NaiveDate>>,
}

impl Search {
    pub fn parse(raw: &str) -> Self {
        let mut s = Search::default();
        let mut words = Vec::new();
        for word in raw.split_whitespace() {
            let Some((key, value)) = word.split_once(':') else {
                words.push(word.to_lowercase());
                continue;
            };
            if value.is_empty() {
                words.push(word.to_lowercase());
                continue;
            }
            let value = value.to_lowercase();
            match key.to_lowercase().as_str() {
                "from" => s.from.push(value),
                "mentions" => s.mentions.push(value),
                "in" => s.channels.push(value),
                "has" => s.has.push(value),
                "before" => s.before.push(parse_day(&value)),
                "after" => s.after.push(parse_day(&value)),
                "during" => s.during.push(parse_day(&value)),
                _ => words.push(word.to_lowercase()),
            }
        }
        s.text = words.join(" ");
        s
    }

    /// True when the search has no text and no filters, so every message is shown.
    pub fn is_empty(&self) -> bool {
        self == &Search::default()
    }

    /// True when a chat message matches the search in the given channel.
    pub fn matches(&self, m: &ChatMessage, channel: &str) -> bool {
        let author = m.author.to_lowercase();
        let text = m.text.to_lowercase();
        if !self.text.is_empty() && !text.contains(&self.text) && !author.contains(&self.text) {
            return false;
        }
        if !self.from.iter().all(|name| author.contains(name)) {
            return false;
        }
        // A mention is the name as a whole word after @, so @bo does not match @bob.
        if !self.mentions.iter().all(|name| mention_in(&text, name)) {
            return false;
        }
        let channel = channel.to_lowercase();
        if !self.channels.iter().all(|name| *name == channel) {
            return false;
        }
        if !self.has.iter().all(|kind| kind == "link" && has_link(&text)) {
            return false;
        }
        let day = local_day(m.timestamp);
        let fits = |d: &Option<NaiveDate>| d.map(day_of);
        if !self.before.iter().all(|d| fits(d).is_some_and(|d| day < d)) {
            return false;
        }
        if !self.after.iter().all(|d| fits(d).is_some_and(|d| day >= d)) {
            return false;
        }
        self.during.iter().all(|d| fits(d).is_some_and(|d| day == d))
    }
}

fn parse_day(text: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(text, "%Y-%m-%d").ok()
}

pub(crate) fn mention_in(text: &str, name: &str) -> bool {
    let needle = format!("@{name}");
    let mut from = 0;
    while let Some(at) = text[from..].find(&needle) {
        let end = from + at + needle.len();
        let next = text[end..].chars().next();
        if !next.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            return true;
        }
        from = end;
    }
    false
}

fn has_link(text: &str) -> bool {
    text.contains("http://") || text.contains("https://")
}
