//! Keybinds inspector: binds.lua inspection and scan code to US label resolution.

use crate::helpers::config_file;
use crate::ui::{gap, row, sec, skin};
use std::fs;
use std::path::Path;

const US_KEYS: &[(u32, &str)] = &[
    (10, "1"), (11, "2"), (12, "3"), (13, "4"), (14, "5"),
    (15, "6"), (16, "7"), (17, "8"), (18, "9"), (19, "0"),
    (20, "-"), (21, "="), (22, "Backspace"), (23, "Tab"),
    (24, "Q"), (25, "W"), (26, "E"), (27, "R"), (28, "T"),
    (29, "Y"), (30, "U"), (31, "I"), (32, "O"), (33, "P"),
    (34, "["), (35, "]"), (36, "Return"),
    (38, "A"), (39, "S"), (40, "D"), (41, "F"), (42, "G"),
    (43, "H"), (44, "J"), (45, "K"), (46, "L"),
    (47, ";"), (48, "'"), (49, "`"),
    (50, "Shift"), (51, "\\"),
    (52, "Z"), (53, "X"), (54, "C"), (55, "V"), (56, "B"),
    (57, "N"), (58, "M"),
    (59, ","), (60, "."), (61, "/"), (62, "Shift"),
    (64, "Alt"), (65, "Space"), (108, "Alt"),
    (111, "Up"), (113, "Left"), (114, "Right"), (116, "Down"),
    (119, "Delete"),
];

pub fn label_for_code(code: u32) -> Option<&'static str> {
    US_KEYS.iter().find(|(c, _)| *c == code).map(|(_, name)| *name)
}

pub fn format_combo(raw: &str) -> String {
    let mut s = raw.to_string();

    // Clean up lua string concatenation artifacts
    s = s.replace("mod .. \"", "SUPER");
    s = s.replace("mod .. '", "SUPER");
    s = s.replace("mod", "SUPER");
    s = s.replace('"', "");
    s = s.replace('\'', "");
    s = s.replace("..", "");

    // Mouse replacements
    s = s.replace("mouse:272", "LMB");
    s = s.replace("mouse:273", "RMB");
    s = s.replace("mouse_up", "Scroll ↑");
    s = s.replace("mouse_down", "Scroll ↓");

    // Replace code:NNN with clean US label
    let mut result = String::new();
    let mut remaining = s.as_str();

    while let Some(idx) = remaining.find("code:") {
        result.push_str(&remaining[..idx]);
        let after = &remaining[idx + 5..];
        let num_len = after.chars().take_while(|c| c.is_ascii_digit()).count();
        if num_len > 0 {
            if let Ok(code) = after[..num_len].parse::<u32>() {
                if let Some(lbl) = label_for_code(code) {
                    result.push_str(lbl);
                } else {
                    result.push_str(&format!("Key{code}"));
                }
            } else {
                result.push_str(&after[..num_len]);
            }
            remaining = &after[num_len..];
        } else {
            result.push_str("code:");
            remaining = after;
        }
    }
    result.push_str(remaining);

    // Normalize spacing around +
    let parts: Vec<&str> = result
        .split('+')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();

    parts.join(" + ")
}

pub struct KeybindEntry {
    pub combo: String,
    pub raw_combo: String,
    pub action: String,
    pub comment: String,
}

pub fn parse_binds_file() -> Result<Vec<KeybindEntry>, String> {
    let candidate = config_file(&["hypr", "modules", "binds.lua"]);
    let path = if candidate.is_file() {
        candidate
    } else if Path::new("configs/hypr/modules/binds.lua").is_file() {
        std::path::PathBuf::from("configs/hypr/modules/binds.lua")
    } else if Path::new("../configs/hypr/modules/binds.lua").is_file() {
        std::path::PathBuf::from("../configs/hypr/modules/binds.lua")
    } else {
        candidate
    };

    let text = fs::read_to_string(&path)
        .map_err(|e| format!("unable to read {}: {e}", path.display()))?;

    let mut entries = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("hl.bind(") {
            continue;
        }

        // Extract comment if present
        let (bind_part, comment) = if let Some(idx) = trimmed.find("--") {
            (trimmed[..idx].trim(), trimmed[idx + 2..].trim().to_string())
        } else {
            (trimmed, String::new())
        };

        // Extract inside of hl.bind(...)
        if let Some(open) = bind_part.find('(') {
            if let Some(close) = bind_part.rfind(')') {
                let inside = &bind_part[open + 1..close];
                // Split top-level comma
                let args = split_lua_args(inside);
                if !args.is_empty() {
                    let raw_combo = args[0].clone();
                    let combo = format_combo(&raw_combo);
                    let action = if args.len() > 1 {
                        args[1].clone()
                    } else {
                        String::new()
                    };

                    entries.push(KeybindEntry {
                        combo,
                        raw_combo,
                        action,
                        comment,
                    });
                }
            }
        }
    }

    Ok(entries)
}

fn split_lua_args(text: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut depth = 0;
    let mut in_quote = false;
    let mut quote_char = ' ';
    let mut start = 0;

    for (i, c) in text.char_indices() {
        if in_quote {
            if c == quote_char {
                in_quote = false;
            }
            continue;
        }
        if c == '"' || c == '\'' {
            in_quote = true;
            quote_char = c;
            continue;
        }
        if c == '(' || c == '{' || c == '[' {
            depth += 1;
        } else if c == ')' || c == '}' || c == ']' {
            if depth > 0 {
                depth -= 1;
            }
        } else if c == ',' && depth == 0 {
            args.push(text[start..i].trim().to_string());
            start = i + 1;
        }
    }
    if start < text.len() {
        args.push(text[start..].trim().to_string());
    }
    args
}

pub fn keybinds(action: &str, combo: Option<&str>) -> i32 {
    match action {
        "format" => {
            if let Some(c) = combo {
                println!("{}", format_combo(c));
                0
            } else {
                eprintln!("xiu keybinds format: missing combo string");
                2
            }
        }
        "list" => list_binds(),
        "export" => export_binds(),
        other => {
            eprintln!("xiu keybinds: unknown action '{other}' (list, format, export)");
            2
        }
    }
}

fn list_binds() -> i32 {
    let entries = match parse_binds_file() {
        Ok(e) => e,
        Err(err) => {
            eprintln!("xiu keybinds: {err}");
            return 1;
        }
    };

    let k = skin();
    sec(&k, "keybinds");
    gap(&k);

    for e in &entries {
        let desc = if !e.comment.is_empty() {
            &e.comment
        } else {
            &e.action
        };
        row(
            &k,
            &format!(
                "{}{:<22}{} {}{}{}",
                k.cream, e.combo, k.rst, k.dim, desc, k.rst
            ),
        );
    }
    gap(&k);
    0
}

fn export_binds() -> i32 {
    let entries = match parse_binds_file() {
        Ok(e) => e,
        Err(err) => {
            eprintln!("xiu keybinds: {err}");
            return 1;
        }
    };

    println!("[");
    for (i, e) in entries.iter().enumerate() {
        let comma = if i + 1 < entries.len() { "," } else { "" };
        let clean_action = e.action.replace('\\', "\\\\").replace('"', "\\\"");
        let clean_comment = e.comment.replace('\\', "\\\\").replace('"', "\\\"");
        let clean_combo = e.combo.replace('\\', "\\\\").replace('"', "\\\"");
        let clean_raw = e.raw_combo.replace('\\', "\\\\").replace('"', "\\\"");
        println!(
            "  {{\"combo\": \"{clean_combo}\", \"raw\": \"{clean_raw}\", \"action\": \"{clean_action}\", \"description\": \"{clean_comment}\"}}{comma}"
        );
    }
    println!("]");
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_resolves_clean_us_labels() {
        assert_eq!(format_combo("mod .. \" + code:24\""), "SUPER + Q");
        assert_eq!(format_combo("mod .. \" + code:28\""), "SUPER + T");
        assert_eq!(format_combo("mod .. \" + code:60\""), "SUPER + .");
        assert_eq!(format_combo("mod .. \" + code:65\""), "SUPER + Space");
        assert_eq!(format_combo("ALT + code:50"), "ALT + Shift");
        assert_eq!(format_combo("CTRL + ALT + code:119"), "CTRL + ALT + Delete");
    }

    #[test]
    fn format_never_outputs_raw_keycode() {
        let formatted = format_combo("mod .. \" + code:46\"");
        assert_eq!(formatted, "SUPER + L");
        assert!(!formatted.contains("code:"));
    }
}
