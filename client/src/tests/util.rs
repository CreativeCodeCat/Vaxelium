use super::*;

#[test]
fn styled_letters_sort_as_plain_ones() {
    assert_eq!(sort_key("𝙲𝚛𝚎𝚊𝚝𝚒𝚟𝚎𝙲𝚘𝚍𝚎𝙲𝚊𝚝"), "creativecodecat", "monospace");
    assert_eq!(sort_key("𝐁𝐨𝐥𝐝"), "bold");
    assert_eq!(sort_key("𝟏𝟐𝟑"), "123", "bold digits");
    assert_eq!(sort_key("Ｗｉｄｅ"), "wide", "full width");
    assert_eq!(sort_key("Dummy"), "dummy");
    let mut names = vec!["Dummy", "𝙲𝚛𝚎𝚊𝚝𝚒𝚟𝚎𝙲𝚘𝚍𝚎𝙲𝚊𝚝", "alice"];
    names.sort_by_key(|n| sort_key(n));
    assert_eq!(names, vec!["alice", "𝙲𝚛𝚎𝚊𝚝𝚒𝚟𝚎𝙲𝚘𝚍𝚎𝙲𝚊𝚝", "Dummy"]);
}
