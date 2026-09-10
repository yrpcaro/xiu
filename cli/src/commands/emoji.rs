//! `xiu emoji` — the everyday glyphs plus the composite classics, copied to
//! the Wayland clipboard by name. An exact name wins; a fuzzy query lists its
//! matches; -p asks through the native dialog tool.

use crate::helpers::copy_text;
use std::process::Command;

/// Pairs of (name, text); a query matches when it appears anywhere in the
/// name.
const EMOJI: &[(&str, &str)] = &[
    ("shrug", "¯\\_(ツ)_/¯"),
    ("tableflip", "(╯°□°)╯︵ ┻━┻"),
    ("unflip", "┬─┬ ノ( ゜-゜ノ)"),
    ("lol", "😂"),
    ("joy", "😂"),
    ("smile", "🙂"),
    ("grin", "😀"),
    ("wink", "😉"),
    ("thinking", "🤔"),
    ("shrugging", "🤷"),
    ("ok", "👌"),
    ("thumbsup", "👍"),
    ("thumbsdown", "👎"),
    ("clap", "👏"),
    ("wave", "👋"),
    ("pray", "🙏"),
    ("muscle", "💪"),
    ("point", "👉"),
    ("eyes", "👀"),
    ("heart", "❤️"),
    ("sparkle", "✨"),
    ("fire", "🔥"),
    ("star", "⭐"),
    ("boom", "💥"),
    ("100", "💯"),
    ("check", "✅"),
    ("cross", "❌"),
    ("question", "❓"),
    ("warning", "⚠️"),
    ("bulb", "💡"),
    ("rocket", "🚀"),
    ("party", "🎉"),
    ("cake", "🎂"),
    ("coffee", "☕"),
    ("pizza", "🍕"),
    ("beer", "🍺"),
    ("moon", "🌙"),
    ("sun", "☀️"),
    ("zap", "⚡"),
    ("snowflake", "❄️"),
    ("bug", "🐛"),
    ("skull", "💀"),
    ("ghost", "👻"),
    ("alien", "👽"),
    ("robot", "🤖"),
    ("cat", "🐱"),
    ("dog", "🐶"),
    ("fox", "🦊"),
    ("panda", "🐼"),
    ("poop", "💩"),
];

fn emoji_matches(query: &str) -> Vec<(&'static str, &'static str)> {
    let q = query.to_lowercase();
    EMOJI.iter().filter(|(name, _)| name.contains(&q)).copied().collect()
}

pub fn emoji(pick: bool, list: bool, query_parts: &[String]) -> i32 {
    let mut query = query_parts.join(" ");

    if pick && query.is_empty() {
        // Ask for a query with the native dialog tool the rice already ships,
        // then run the same match-and-copy path as a typed query.
        match Command::new("kdialog")
            .args(["--inputbox", "Emoji — type a name (shrug, fire, heart, ...)"])
            .output()
        {
            Ok(out) if out.status.success() => {
                query = String::from_utf8_lossy(&out.stdout).trim().to_string();
            }
            Ok(_) => return 1,
            Err(e) => {
                eprintln!("xiu emoji: kdialog unavailable ({e})");
                return 127;
            }
        }
    }

    if list || query.is_empty() {
        let hits: Vec<(&str, &str)> = if query.is_empty() {
            EMOJI.to_vec()
        } else {
            emoji_matches(&query)
        };
        for (name, glyph) in hits {
            println!("{name}\t{glyph}");
        }
        return 0;
    }

    // An exact name always wins; only a fuzzy query is ambiguous.
    if let Some((_, glyph)) = EMOJI.iter().find(|(name, _)| *name == query) {
        return copy_text(glyph);
    }

    let hits = emoji_matches(&query);
    match hits.len() {
        0 => {
            eprintln!("xiu emoji: no match for '{query}' (`xiu emoji -l` lists the set)");
            1
        }
        1 => copy_text(hits[0].1),
        _ => {
            eprintln!("xiu emoji: ambiguous, matches:");
            for (name, glyph) in hits {
                eprintln!("  {name}\t{glyph}");
            }
            2
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_lookup_finds_shrug() {
        assert!(EMOJI.iter().any(|(name, _)| *name == "shrug"));
    }

    #[test]
    fn fuzzy_match_is_case_insensitive() {
        assert!(!emoji_matches("FIRE").is_empty());
    }

    #[test]
    fn nonsense_query_matches_nothing() {
        assert!(emoji_matches("qq-nothing-xyz").is_empty());
    }
}
