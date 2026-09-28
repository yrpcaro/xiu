//! Screen recording thumbnail generation and cache management.
//!
//! Replaces rec-thumbs.sh.

use crate::helpers::home_path;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub fn rec_thumbs(recdir: Option<&str>) -> i32 {
    let Some(dir_str) = recdir else {
        return 0;
    };
    if dir_str.is_empty() {
        return 0;
    }

    let rec_path = Path::new(dir_str);
    if !rec_path.is_dir() {
        return 0;
    }

    let cache_dir = std::env::var("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home_path(&[".cache"]))
        .join("ricelin")
        .join("rec-thumbs");

    let _ = fs::create_dir_all(&cache_dir);

    // 1. Prune thumbnails whose source recording no longer exists
    if let Ok(entries) = fs::read_dir(&cache_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() && p.extension().and_then(|e| e.to_str()) == Some("jpg") {
                if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                    let src = rec_path.join(format!("{stem}.mp4"));
                    if !src.exists() {
                        let _ = fs::remove_file(p);
                    }
                }
            }
        }
    }

    // 2. Generate missing or outdated thumbnails
    if let Ok(entries) = fs::read_dir(rec_path) {
        for entry in entries.flatten() {
            let src = entry.path();
            if !src.is_file() {
                continue;
            }
            let Some(file_name) = src.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if !file_name.starts_with("recording_") || !file_name.ends_with(".mp4") {
                continue;
            }

            let Some(stem) = src.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            let thumb = cache_dir.join(format!("{stem}.jpg"));

            let needs_update = match (fs::metadata(&src), fs::metadata(&thumb)) {
                (Ok(sm), Ok(tm)) => tm.len() == 0 || sm.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH) > tm.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH),
                (Ok(_), Err(_)) => true,
                _ => false,
            };

            if needs_update {
                let tmp = cache_dir.join(format!("{stem}.tmp.jpg"));
                let _ = fs::remove_file(&tmp);

                let run_ffmpeg = |ss: &str| {
                    Command::new("ffmpeg")
                        .args([
                            "-y",
                            "-ss",
                            ss,
                            "-i",
                            &src.to_string_lossy(),
                            "-frames:v",
                            "1",
                            "-vf",
                            "scale=200:-1",
                            "-q:v",
                            "4",
                            &tmp.to_string_lossy(),
                        ])
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .status()
                };

                let _ = run_ffmpeg("1");
                let valid = fs::metadata(&tmp).map(|m| m.len() > 0).unwrap_or(false);
                if !valid {
                    let _ = run_ffmpeg("0");
                }

                if fs::metadata(&tmp).map(|m| m.len() > 0).unwrap_or(false) {
                    let _ = fs::rename(&tmp, &thumb);
                } else {
                    let _ = fs::remove_file(&tmp);
                }
            }
        }
    }

    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rec_thumbs_none_recdir() {
        assert_eq!(rec_thumbs(None), 0);
    }

    #[test]
    fn test_rec_thumbs_nonexistent_dir() {
        assert_eq!(rec_thumbs(Some("/definitely/not/a/real/dir")), 0);
    }
}
