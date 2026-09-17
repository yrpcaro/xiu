//! Direct XKB layout switching and abbreviation formatting.

use crate::helpers::run_status;
use crate::json;
use std::process::Command;

pub fn layout(action: &str, target: Option<&str>) -> i32 {
    match action {
        "switch" => switch(target),
        "get" => get(),
        "format" => {
            if let Some(name) = target {
                println!("{}", format_keymap(name));
                0
            } else {
                eprintln!("xiu layout format: missing layout name argument");
                2
            }
        }
        other => {
            eprintln!("xiu layout: unknown action '{other}' (switch, get, format)");
            2
        }
    }
}

pub fn format_keymap(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    // 1. Strict 2-3 letter parenthetical filter ("English (US)" -> "US", "German (DE)" -> "DE")
    // Descriptors with 4+ letters ("Persian (Windows)", "Russian (phonetic)") are variants, not country codes.
    if let Some(open) = trimmed.rfind('(') {
        if let Some(close) = trimmed.rfind(')') {
            if close > open + 1 && close == trimmed.len() - 1 {
                let inside = trimmed[open + 1..close].trim();
                if (inside.len() == 2 || inside.len() == 3)
                    && inside.chars().all(|c| c.is_ascii_alphabetic())
                {
                    return inside.to_ascii_uppercase();
                }
            }
        }
    }

    // 2. Base name resolution: strip parenthetical variants ("Persian (Windows)" -> "Persian")
    let base = if let Some(idx) = trimmed.find('(') {
        trimmed[..idx].trim()
    } else {
        trimmed
    };
    let n = base.to_ascii_lowercase();

    // 3. Named dictionary resolution
    let mapped = match n.as_str() {
        "persian" | "farsi" => "FA",
        "arabic" | "arab" => "AR",
        "russian" => "RU",
        "polish" => "PL",
        "ukrainian" => "UA",
        "greek" => "GR",
        "turkish" => "TR",
        "hebrew" => "HE",
        "thai" => "TH",
        "japanese" => "JA",
        "korean" => "KO",
        "chinese" => "ZH",
        "french" => "FR",
        "german" => "DE",
        "spanish" => "ES",
        "italian" => "IT",
        "swedish" => "SV",
        "norwegian" => "NO",
        "danish" => "DA",
        "finnish" => "FI",
        "portuguese" => "PT",
        "czech" => "CS",
        "hungarian" => "HU",
        "romanian" => "RO",
        "english" => "US",
        "dutch" => "NL",
        _ => "",
    };

    if !mapped.is_empty() {
        return mapped.to_string();
    }

    // 4. Strict abbreviation invariant: clamp/truncate strictly to 2-3 uppercase letters
    let letters: String = base.chars().filter(|c| c.is_ascii_alphabetic()).collect();
    if letters.len() >= 2 {
        letters.chars().take(3).collect::<String>().to_ascii_uppercase()
    } else {
        trimmed.chars().take(3).collect::<String>().to_ascii_uppercase()
    }
}

fn get() -> i32 {
    let output = Command::new("hyprctl").args(["devices", "-j"]).output();
    let Ok(out) = output else {
        eprintln!("xiu layout get: hyprctl unavailable");
        return 1;
    };

    let text = String::from_utf8_lossy(&out.stdout);
    let Ok(data) = json::parse(&text) else {
        eprintln!("xiu layout get: unable to parse hyprctl devices");
        return 1;
    };

    let keyboards = data.get("keyboards").and_then(json::Json::as_arr);
    let Some(kb_list) = keyboards else {
        eprintln!("xiu layout get: no keyboards reported by hyprctl");
        return 1;
    };

    // Find main keyboard or first keyboard with an active_keymap
    let mut chosen_map = None;
    for kb in kb_list {
        let is_main = kb.get("main").and_then(json::Json::as_bool).unwrap_or(false);
        if let Some(map) = kb.get("active_keymap").and_then(json::Json::as_str) {
            if is_main {
                chosen_map = Some(map);
                break;
            }
            if chosen_map.is_none() {
                chosen_map = Some(map);
            }
        }
    }

    if let Some(km) = chosen_map {
        let code = format_keymap(km);
        println!("{code}");
        0
    } else {
        eprintln!("xiu layout get: no active keymap found");
        1
    }
}

fn switch(target: Option<&str>) -> i32 {
    let mut cmd = Command::new("hyprctl");
    cmd.args(["switchxkblayout", "current"]);
    match target {
        Some("next") | None => cmd.arg("next"),
        Some("prev") | Some("previous") => cmd.arg("prev"),
        Some(name) => cmd.arg(name),
    };
    run_status(&mut cmd)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_handles_parenthetical_codes() {
        assert_eq!(format_keymap("English (US)"), "US");
        assert_eq!(format_keymap("German (DE)"), "DE");
    }

    #[test]
    fn format_rejects_variant_windows_bug() {
        assert_eq!(format_keymap("Persian (Windows)"), "FA");
        assert_eq!(format_keymap("Persian (with Persian keypad)"), "FA");
        assert_eq!(format_keymap("Russian (phonetic)"), "RU");
        assert_eq!(format_keymap("English (Dvorak)"), "US");
    }

    #[test]
    fn format_enforces_abbreviation_invariant() {
        let code = format_keymap("SomethingLongWithoutParens");
        assert!(code.len() <= 3);
        assert_eq!(code, "SOM");
    }
}
