//! Window resizer and Picture-in-Picture manager.
//!
//! Native Rust implementation of the Hyprland window resizer and PiP event daemon.

use crate::helpers::config_file;
use crate::json::Json;
use regex::Regex;
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::Command;
use std::time::SystemTime;

extern "C" {
    fn getuid() -> u32;
}

#[derive(Debug, Clone, PartialEq)]
pub struct WindowRule {
    pub name: String,
    pub match_type: String,
    pub width: String,
    pub height: String,
    pub actions: Vec<String>,
}

pub struct RateLimiter {
    pub tracker: HashMap<String, f64>,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            tracker: HashMap::new(),
        }
    }

    fn current_time() -> f64 {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0)
    }

    pub fn is_rate_limited(&mut self, key: &str) -> bool {
        let now = Self::current_time();
        if self.tracker.len() > 50 {
            let cutoff = now - 60.0;
            self.tracker.retain(|_, &mut v| v > cutoff);
        }

        let last_time = self.tracker.get(key).copied().unwrap_or(0.0);
        if now < last_time + 1.0 {
            return true;
        }

        self.tracker.insert(key.to_string(), now);
        false
    }
}

fn socket2_path() -> PathBuf {
    let runtime = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| {
        let uid = unsafe { getuid() };
        format!("/run/user/{uid}")
    });
    let signature = std::env::var("HYPRLAND_INSTANCE_SIGNATURE").unwrap_or_default();
    PathBuf::from(runtime)
        .join("hypr")
        .join(signature)
        .join(".socket2.sock")
}

pub fn default_rules() -> Vec<WindowRule> {
    vec![
        WindowRule {
            name: "(Bitwarden".to_string(),
            match_type: "titleContains".to_string(),
            width: "20%".to_string(),
            height: "54%".to_string(),
            actions: vec!["float".to_string(), "center".to_string()],
        },
        WindowRule {
            name: "^[Pp]icture(-| )in(-| )[Pp]icture$".to_string(),
            match_type: "titleRegex".to_string(),
            width: String::new(),
            height: String::new(),
            actions: vec!["pip".to_string()],
        },
    ]
}

pub fn load_window_rules() -> Vec<WindowRule> {
    let config_path = config_file(&["xiu", "resizer.json"]);
    if !config_path.is_file() {
        return default_rules();
    }

    let content = match std::fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(_) => {
            eprintln!("xiu-resizer: invalid resizer.json, falling back to default rules");
            return default_rules();
        }
    };

    let parsed = match crate::json::parse(&content) {
        Ok(j) => j,
        Err(_) => {
            eprintln!("xiu-resizer: invalid resizer.json, falling back to default rules");
            return default_rules();
        }
    };

    if let Some(resizer) = parsed.get("resizer") {
        if let Some(rules_arr) = resizer.get("rules").and_then(Json::as_arr) {
            let mut rules = Vec::new();
            for r in rules_arr {
                let name = r.get("name").and_then(Json::as_str).unwrap_or("").to_string();
                let match_type = r
                    .get("matchType")
                    .or_else(|| r.get("match_type"))
                    .and_then(Json::as_str)
                    .unwrap_or("")
                    .to_string();
                let width = r.get("width").and_then(Json::as_str).unwrap_or("").to_string();
                let height = r.get("height").and_then(Json::as_str).unwrap_or("").to_string();
                let actions = r
                    .get("actions")
                    .and_then(Json::as_arr)
                    .map(|arr| {
                        arr.iter()
                            .filter_map(Json::as_str)
                            .map(String::from)
                            .collect()
                    })
                    .unwrap_or_default();

                rules.push(WindowRule {
                    name,
                    match_type,
                    width,
                    height,
                    actions,
                });
            }
            return rules;
        }
    }

    eprintln!("xiu-resizer: invalid resizer.json, falling back to default rules");
    default_rules()
}

pub fn match_window_rule<'a>(
    rules: &'a [WindowRule],
    window_title: &str,
    initial_title: &str,
) -> Option<&'a WindowRule> {
    for rule in rules {
        match rule.match_type.as_str() {
            "initialTitle" => {
                if initial_title == rule.name {
                    return Some(rule);
                }
            }
            "titleContains" => {
                if window_title.contains(&rule.name) {
                    return Some(rule);
                }
            }
            "titleExact" => {
                if window_title == rule.name {
                    return Some(rule);
                }
            }
            "titleRegex" => match Regex::new(&rule.name) {
                Ok(re) => {
                    if re.is_match(window_title) {
                        return Some(rule);
                    }
                }
                Err(_) => {
                    eprintln!("xiu-resizer: invalid regex pattern in rule '{}'", rule.name);
                }
            },
            _ => {}
        }
    }
    None
}

fn hyprctl(args: &[&str]) -> String {
    Command::new("hyprctl")
        .args(args)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default()
}

fn hyprctl_json(query: &str) -> Option<Json> {
    let out = hyprctl(&["-j", query]);
    crate::json::parse(&out).ok()
}

fn batch(commands: &[&str]) {
    if commands.is_empty() {
        return;
    }
    let joined = commands.join(";");
    let _ = Command::new("hyprctl")
        .args(["--batch", &joined])
        .output();
}

fn make_resize_cmd(width: &str, height: &str, address: &str) -> String {
    format!("dispatch hl.dsp.window.resize({{x = {width}, y = {height}, exact = true, window = \"address:{address}\"}})")
}

fn make_move_cmd(x: i32, y: i32, address: &str) -> String {
    format!("dispatch hl.dsp.window.move({{x = {x}, y = {y}, window = \"address:{address}\"}})")
}

fn make_float_cmd(address: &str) -> String {
    format!("dispatch hl.dsp.window.float({{action = \"toggle\", window = \"address:{address}\"}})")
}

fn make_center_cmd() -> String {
    "dispatch hl.dsp.window.center()".to_string()
}

fn get_window_info(window_id: &str) -> Option<Json> {
    let clients = hyprctl_json("clients")?;
    let address = format!("0x{window_id}");
    if let Json::Arr(arr) = clients {
        for client in arr {
            if client.get("address").and_then(Json::as_str) == Some(&address) {
                return Some(client);
            }
        }
    }
    None
}

fn apply_pip_action(window_id: &str) {
    let address = format!("0x{window_id}");
    let clients_result = match hyprctl_json("clients") {
        Some(Json::Arr(arr)) => arr,
        _ => return,
    };

    let window = match clients_result
        .into_iter()
        .find(|c| c.get("address").and_then(Json::as_str) == Some(&address))
    {
        Some(w) => w,
        None => return,
    };

    if window.get("floating").and_then(Json::as_bool) != Some(true) {
        return;
    }

    let workspaces_result = match hyprctl_json("workspaces") {
        Some(Json::Arr(arr)) => arr,
        _ => return,
    };

    let workspace_info = match window.get("workspace") {
        Some(w) => w,
        None => return,
    };

    let workspace_name = match workspace_info.get("name").and_then(Json::as_str) {
        Some(name) => name,
        None => return,
    };

    let workspace = match workspaces_result
        .into_iter()
        .find(|w| w.get("name").and_then(Json::as_str) == Some(workspace_name))
    {
        Some(w) => w,
        None => return,
    };

    let monitor_id = match workspace
        .get("monitorID")
        .and_then(|m| m.as_i64().or_else(|| m.as_f64().map(|v| v as i64)))
    {
        Some(id) => id,
        None => return,
    };

    let monitors_result = match hyprctl_json("monitors") {
        Some(Json::Arr(arr)) => arr,
        _ => return,
    };

    let monitor = match monitors_result.into_iter().find(|m| {
        m.get("id")
            .and_then(|id| id.as_i64().or_else(|| id.as_f64().map(|v| v as i64)))
            == Some(monitor_id)
    }) {
        Some(m) => m,
        None => return,
    };

    let size_arr = match window.get("size").and_then(Json::as_arr) {
        Some(arr) if arr.len() >= 2 => arr,
        _ => return,
    };

    let width = match size_arr[0]
        .as_f64()
        .or_else(|| size_arr[0].as_i64().map(|v| v as f64))
    {
        Some(w) => w,
        None => return,
    };
    let height = match size_arr[1]
        .as_f64()
        .or_else(|| size_arr[1].as_i64().map(|v| v as f64))
    {
        Some(h) if h > 0.0 => h,
        _ => return,
    };

    let m_height = match monitor
        .get("height")
        .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|x| x as f64)))
    {
        Some(h) => h,
        None => return,
    };
    let m_width = match monitor
        .get("width")
        .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|x| x as f64)))
    {
        Some(w) => w,
        None => return,
    };
    let m_scale = match monitor
        .get("scale")
        .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|x| x as f64)))
    {
        Some(s) if s > 0.0 => s,
        _ => return,
    };
    let m_x = match monitor
        .get("x")
        .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|x| x as f64)))
    {
        Some(x) => x,
        None => return,
    };
    let m_y = match monitor
        .get("y")
        .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|x| x as f64)))
    {
        Some(y) => y,
        None => return,
    };

    let monitor_height = m_height / m_scale;
    let monitor_width = m_width / m_scale;

    let scale_factor = monitor_height / 4.0 / height;
    let mut scaled_width = (width * scale_factor) as i32;
    let mut scaled_height = (height * scale_factor) as i32;

    let min_width = 200;
    let min_height = 150;
    scaled_width = scaled_width.max(min_width);
    scaled_height = scaled_height.max(min_height);

    let offset = monitor_width.min(monitor_height) * 0.03;

    let move_x = m_x + monitor_width - (scaled_width as f64) - offset;
    let move_y = m_y + monitor_height - (scaled_height as f64) - offset;

    let cmd1 = make_resize_cmd(&scaled_width.to_string(), &scaled_height.to_string(), &address);
    let cmd2 = make_move_cmd(move_x as i32, move_y as i32, &address);
    batch(&[&cmd1, &cmd2]);

    println!(
        "xiu-resizer: Applied PiP action to window {address}: {scaled_width}x{scaled_height} at ({move_x}, {move_y})"
    );
}

fn apply_window_actions(window_id: &str, width: &str, height: &str, actions: &[String]) -> bool {
    let mut dispatch_commands = Vec::new();

    if actions.iter().any(|a| a == "float") {
        if let Some(window_info) = get_window_info(window_id) {
            if window_info.get("floating").and_then(Json::as_bool) != Some(true) {
                dispatch_commands.push(make_float_cmd(&format!("0x{window_id}")));
            }
        }
    }

    if actions.iter().any(|a| a == "pip") {
        apply_pip_action(window_id);
        return true;
    }

    dispatch_commands.push(make_resize_cmd(width, height, &format!("0x{window_id}")));

    if actions.iter().any(|a| a == "center") {
        dispatch_commands.push(make_center_cmd());
    }

    let cmd_refs: Vec<&str> = dispatch_commands.iter().map(String::as_str).collect();
    batch(&cmd_refs);
    println!(
        "xiu-resizer: Applied actions to window 0x{window_id}: {width} x {height} ({})",
        actions.join(", ")
    );
    true
}

pub fn handle_title_event(event: &str, rules: &[WindowRule], limiter: &mut RateLimiter) {
    let data = if let Some(idx) = event.find(">>>") {
        &event[idx + 3..]
    } else if let Some(idx) = event.find(">>") {
        &event[idx + 2..]
    } else {
        return;
    };

    let mut parts = data.splitn(2, ',');
    let window_id = parts.next().unwrap_or("").trim_start_matches('>');
    let window_title = parts.next().unwrap_or("");

    if !window_id.chars().all(|c| c.is_ascii_hexdigit()) || window_id.is_empty() {
        eprintln!("xiu-resizer: invalid window ID format: {window_id}");
        return;
    }

    let mut matched_rule = match_window_rule(rules, window_title, "");
    if matched_rule.is_none() && rules.iter().any(|r| r.match_type == "initialTitle") {
        if let Some(info) = get_window_info(window_id) {
            let initial_title = info.get("initialTitle").and_then(Json::as_str).unwrap_or("");
            matched_rule = match_window_rule(rules, window_title, initial_title);
        }
    }

    if let Some(rule) = matched_rule {
        if limiter.is_rate_limited(window_id) {
            return;
        }

        println!("xiu-resizer: Matched rule '{}' for window 0x{window_id}", rule.name);
        apply_window_actions(window_id, &rule.width, &rule.height, &rule.actions);
    }
}

pub fn handle_open_event(event: &str, rules: &[WindowRule], limiter: &mut RateLimiter) {
    let data = if let Some(stripped) = event.strip_prefix("openwindow>>>") {
        stripped
    } else if let Some(stripped) = event.strip_prefix("openwindow>>") {
        stripped
    } else {
        return;
    };

    let parts: Vec<&str> = data.splitn(4, ',').collect();
    if parts.len() < 4 {
        eprintln!("xiu-resizer: failed to parse window open event: insufficient parts");
        return;
    }

    let window_id = parts[0].trim_start_matches('>');
    let _workspace = parts[1];
    let _window_class = parts[2];
    let title = parts[3];

    if !window_id.chars().all(|c| c.is_ascii_hexdigit()) || window_id.is_empty() {
        eprintln!("xiu-resizer: invalid window ID format: {window_id}");
        return;
    }

    if let Some(rule) = match_window_rule(rules, title, title) {
        if limiter.is_rate_limited(window_id) {
            return;
        }

        println!("xiu-resizer: Matched rule '{}' for new window 0x{window_id}", rule.name);
        apply_window_actions(window_id, &rule.width, &rule.height, &rule.actions);
    }
}

fn run_daemon(rules: &[WindowRule]) -> i32 {
    println!("xiu-resizer: Hyprland window resizer started");
    println!("xiu-resizer: Loaded {} window rules", rules.len());

    let path = socket2_path();
    if !path.exists() {
        eprintln!("xiu-resizer: Hyprland socket not found at {}", path.display());
        return 1;
    }

    let stream = match UnixStream::connect(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("xiu-resizer: failed to connect to Hyprland socket: {e}");
            return 1;
        }
    };

    println!("xiu-resizer: Connected to Hyprland socket, listening for events...");

    let mut limiter = RateLimiter::new();
    let reader = BufReader::new(stream);

    for line in reader.lines() {
        match line {
            Ok(line) => {
                let trimmed = line.trim();
                if trimmed.starts_with("windowtitle") {
                    handle_title_event(trimmed, rules, &mut limiter);
                } else if trimmed.starts_with("openwindow") {
                    handle_open_event(trimmed, rules, &mut limiter);
                }
            }
            Err(e) => {
                eprintln!("xiu-resizer: socket read error: {e}");
                break;
            }
        }
    }

    0
}

fn run_pip_mode() -> i32 {
    let active = match hyprctl_json("activewindow") {
        Some(v @ Json::Obj(_)) => v,
        _ => {
            eprintln!("xiu-resizer: no active window found");
            return 1;
        }
    };

    let address = match active.get("address").and_then(Json::as_str) {
        Some(a) if a.starts_with("0x") => a,
        _ => {
            eprintln!("xiu-resizer: invalid window address");
            return 1;
        }
    };

    let window_id = &address[2..];
    let title = active.get("title").and_then(Json::as_str).unwrap_or("");

    if active.get("floating").and_then(Json::as_bool) != Some(true) {
        eprintln!("xiu-resizer: window '{title}' is not floating; PiP only works on floating windows.");
        return 1;
    }

    println!("xiu-resizer: Applying PiP to active window: '{title}'");
    apply_pip_action(window_id);
    println!("xiu-resizer: PiP applied successfully");
    0
}

fn find_matching_windows(rule: &WindowRule) -> Vec<Json> {
    let clients = match hyprctl_json("clients") {
        Some(Json::Arr(arr)) => arr,
        _ => return Vec::new(),
    };

    let compiled_regex = if rule.match_type == "titleRegex" {
        match Regex::new(&rule.name) {
            Ok(r) => Some(r),
            Err(_) => {
                eprintln!("xiu-resizer: invalid regex pattern '{}'", rule.name);
                return Vec::new();
            }
        }
    } else {
        None
    };

    let mut matches = Vec::new();
    for window in clients {
        let title = window.get("title").and_then(Json::as_str).unwrap_or("");
        let initial_title = window.get("initialTitle").and_then(Json::as_str).unwrap_or("");

        let matched = match rule.match_type.as_str() {
            "initialTitle" => initial_title == rule.name,
            "titleContains" => title.contains(&rule.name),
            "titleExact" => title == rule.name,
            "titleRegex" => {
                if let Some(re) = &compiled_regex {
                    re.is_match(title)
                } else {
                    false
                }
            }
            _ => false,
        };

        if matched {
            matches.push(window);
        }
    }

    matches
}

fn apply_to_active_window(temp_rule: &WindowRule) -> i32 {
    let active = match hyprctl_json("activewindow") {
        Some(v @ Json::Obj(_)) => v,
        _ => {
            eprintln!("xiu-resizer: no active window found");
            return 1;
        }
    };

    let title = active.get("title").and_then(Json::as_str).unwrap_or("");
    let address = match active.get("address").and_then(Json::as_str) {
        Some(a) if a.starts_with("0x") => a,
        _ => {
            eprintln!("xiu-resizer: invalid window address");
            return 1;
        }
    };

    let window_id = &address[2..];
    println!("xiu-resizer: Applying rule to active window 0x{window_id}: '{title}'");
    if apply_window_actions(
        window_id,
        &temp_rule.width,
        &temp_rule.height,
        &temp_rule.actions,
    ) {
        println!("xiu-resizer: Rule applied successfully");
        0
    } else {
        eprintln!("xiu-resizer: failed to apply rule");
        1
    }
}

fn run_active_mode(
    pattern: &str,
    match_type: &str,
    width: &str,
    height: &str,
    actions_str: &str,
) -> i32 {
    let actions: Vec<String> = if actions_str.is_empty() {
        Vec::new()
    } else {
        actions_str.split(',').map(|s| s.trim().to_string()).collect()
    };

    let temp_rule = WindowRule {
        name: pattern.to_string(),
        match_type: match_type.to_string(),
        width: width.to_string(),
        height: height.to_string(),
        actions,
    };

    if temp_rule.name.eq_ignore_ascii_case("active") {
        return apply_to_active_window(&temp_rule);
    }

    let matching = find_matching_windows(&temp_rule);
    if matching.is_empty() {
        eprintln!(
            "xiu-resizer: no windows found matching pattern '{}' with match type '{}'",
            temp_rule.name, temp_rule.match_type
        );
        return 0;
    }

    println!("xiu-resizer: Found {} matching window(s)", matching.len());
    let mut success_count = 0;
    for window in &matching {
        if let Some(addr) = window.get("address").and_then(Json::as_str) {
            let window_id = addr.trim_start_matches("0x");
            let title = window.get("title").and_then(Json::as_str).unwrap_or("");
            println!("xiu-resizer: Applying rule to window 0x{window_id}: '{title}'");
            if apply_window_actions(
                window_id,
                &temp_rule.width,
                &temp_rule.height,
                &temp_rule.actions,
            ) {
                success_count += 1;
            }
        }
    }
    println!(
        "xiu-resizer: Successfully applied rule to {success_count}/{} windows",
        matching.len()
    );
    0
}

fn usage() {
    println!(
        "xiu-resizer — window resizer / PiP daemon.\n\n\
Usage:\n  \
xiu resizer [-d|--daemon]                    watch windowtitle/openwindow events and apply rules\n  \
xiu resizer pip                              quick PiP on the active floating window\n  \
xiu resizer PATTERN MATCH_TYPE WIDTH HEIGHT ACTIONS\n                                               \
one-shot: apply to every matching window (\"active\" targets focused window)"
    );
}

pub fn resizer(args: &[String]) -> i32 {
    let rules = load_window_rules();

    if args.iter().any(|a| a == "-h" || a == "--help") {
        usage();
        return 0;
    }

    if args.iter().any(|a| a == "-d" || a == "--daemon") {
        return run_daemon(&rules);
    }

    if args.first().map(|s| s.as_str()) == Some("pip") {
        return run_pip_mode();
    }

    if args.len() >= 5 {
        return run_active_mode(&args[0], &args[1], &args[2], &args[3], &args[4]);
    }

    if args.is_empty() {
        println!(
            "xiu-resizer: use --daemon to start, 'pip' for quick pip mode, or provide pattern, match_type, width, height, and actions"
        );
        return 0;
    }

    println!(
        "xiu-resizer: use --daemon to start, 'pip' for quick pip mode, or provide pattern, match_type, width, height, and actions"
    );
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeout_tracker_pruning() {
        let mut limiter = RateLimiter::new();
        let now = RateLimiter::current_time();
        for i in 0..60 {
            limiter.tracker.insert(format!("old_{i}"), now - 75.0);
        }
        assert_eq!(limiter.tracker.len(), 60);

        assert!(!limiter.is_rate_limited("new_window"));
        assert!(!limiter.tracker.contains_key("old_0"));
        assert!(limiter.tracker.contains_key("new_window"));
        assert_eq!(limiter.tracker.len(), 1);
    }

    #[test]
    fn test_rate_limiter_rate_limits_within_one_second() {
        let mut limiter = RateLimiter::new();
        assert!(!limiter.is_rate_limited("win1"));
        assert!(limiter.is_rate_limited("win1"));
    }

    #[test]
    fn test_match_window_rule() {
        let rules = default_rules();

        let r1 = match_window_rule(&rules, "(Bitwarden Password Manager) - Mozilla Firefox", "");
        assert!(r1.is_some());
        assert_eq!(r1.unwrap().name, "(Bitwarden");

        let r2 = match_window_rule(&rules, "Picture-in-Picture", "");
        assert!(r2.is_some());

        let r3 = match_window_rule(&rules, "Random Browser Tab", "");
        assert!(r3.is_none());
    }

    #[test]
    fn test_match_window_rule_exact_and_initial() {
        let rules = vec![
            WindowRule {
                name: "Exact Title".to_string(),
                match_type: "titleExact".to_string(),
                width: "50%".to_string(),
                height: "50%".to_string(),
                actions: vec!["center".to_string()],
            },
            WindowRule {
                name: "Initial Only".to_string(),
                match_type: "initialTitle".to_string(),
                width: "30%".to_string(),
                height: "30%".to_string(),
                actions: vec!["float".to_string()],
            },
        ];

        assert!(match_window_rule(&rules, "Exact Title", "").is_some());
        assert!(match_window_rule(&rules, "Exact Title Extra", "").is_none());
        assert!(match_window_rule(&rules, "Different Title", "Initial Only").is_some());
    }

    #[test]
    fn test_handle_title_event_fast_no_match() {
        let rules = default_rules();
        let mut limiter = RateLimiter::new();
        let event = "windowtitle>>1234abcd,Random Unmatched Window Title";
        handle_title_event(event, &rules, &mut limiter);
    }

    #[test]
    fn test_handle_open_event_fast_no_match() {
        let rules = default_rules();
        let mut limiter = RateLimiter::new();
        let event = "openwindow>>1234abcd,1,kitty,Random Kitty Title";
        handle_open_event(event, &rules, &mut limiter);
    }

    #[test]
    fn test_resizer_cli_help() {
        let args = vec!["--help".to_string()];
        assert_eq!(resizer(&args), 0);
    }

    #[test]
    fn test_resizer_cli_empty_args() {
        assert_eq!(resizer(&[]), 0);
    }
}
