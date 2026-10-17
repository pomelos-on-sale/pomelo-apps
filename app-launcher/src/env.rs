//! Environment configuration loader for Pomelo UI.
//!
//! Automatically loads environment variables from all configuration files located in
//! `/internal/AppData/env/` (or `~/.pomelo/AppData/env/` on Desktop), enabling distinct
//! applications to configure their environment independently (e.g. `system.env`,
//! `daily_schedule.env`).

use std::fs;
use std::path::PathBuf;

#[cfg(unix)]
extern "C" {
    fn tzset();
}

/// Scans and loads all environment configuration files from `AppData/env/` into the process environment.
pub fn load_environment() {
    let env_dirs = candidate_env_dirs();
    let mut loaded_files = 0;

    for dir in &env_dirs {
        if dir.is_dir() {
            if let Ok(entries) = fs::read_dir(dir) {
                let mut files: Vec<PathBuf> = entries
                    .filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .filter(|p| p.is_file())
                    .collect();
                files.sort();

                for file_path in files {
                    if let Ok(content) = fs::read_to_string(&file_path) {
                        parse_and_inject_env(&content);
                        loaded_files += 1;
                    }
                }
            }
        }
    }

    // On PC development / desktop simulator, also check `./.env` or `../.env` if present
    #[cfg(not(target_os = "espidf"))]
    {
        for candidate in &[".env", "../.env"] {
            let p = std::path::Path::new(candidate);
            if p.is_file() {
                if let Ok(content) = fs::read_to_string(p) {
                    parse_and_inject_env(&content);
                    loaded_files += 1;
                }
            }
        }
    }

    // Fallback: If no environment file configured TZ, default to China Standard Time (UTC+8)
    if std::env::var("TZ").is_err() {
        std::env::set_var("TZ", "CST-8");
    }

    #[cfg(unix)]
    unsafe {
        tzset();
    }

    println!("[env] loaded {loaded_files} configuration file(s), active TZ={:?}", std::env::var("TZ").unwrap_or_default());
}

/// Parses the contents of an .env configuration file and injects them into `std::env`.
pub fn parse_and_inject_env(content: &str) {
    for line in content.lines() {
        let trimmed = line.trim();
        // Ignore empty lines and comment lines
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if let Some((key, val)) = trimmed.split_once('=') {
            let key = key.trim();
            let val = val.trim().trim_matches('"').trim_matches('\'');
            if !key.is_empty() {
                std::env::set_var(key, val);
                if key == "TZ" {
                    #[cfg(unix)]
                    unsafe {
                        tzset();
                    }
                }
            }
        } else if trimmed.starts_with("webcal://")
            || trimmed.starts_with("http://")
            || trimmed.starts_with("https://")
        {
            // If the user pasted a raw subscription link into the file without key=
            std::env::set_var("SCHEDULE_URL", trimmed);
        }
    }
}

/// Returns potential directory paths where `AppData/env` may reside.
fn candidate_env_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    // Standard AppData/env (e.g. /internal/AppData/env on board, ~/.pomelo/AppData/env on PC)
    let app_data = pomelo_widgets::preferences::resolve_app_data_root();
    dirs.push(app_data.join("env"));

    // Also check hardware board path directly if distinct
    let board_path = PathBuf::from("/internal/AppData/env");
    if !dirs.contains(&board_path) {
        dirs.push(board_path);
    }

    // Developer convenience on desktop: check relative asset directories
    #[cfg(not(target_os = "espidf"))]
    {
        let rel_assets = PathBuf::from("assets/internal/AppData/env");
        if !dirs.contains(&rel_assets) {
            dirs.push(rel_assets);
        }
        let parent_assets = PathBuf::from("../assets/internal/AppData/env");
        if !dirs.contains(&parent_assets) {
            dirs.push(parent_assets);
        }
        let grandparent_assets = PathBuf::from("../../assets/internal/AppData/env");
        if !dirs.contains(&grandparent_assets) {
            dirs.push(grandparent_assets);
        }
    }

    dirs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_env_content() {
        let sample = r#"
# System settings
TZ=CST-8
APP_ENV=testing
QUOTED_VAL="hello world"
SINGLE_QUOTED='single'
webcal://example.com/calendar.ics
"#;
        parse_and_inject_env(sample);

        assert_eq!(std::env::var("TZ").unwrap(), "CST-8");
        assert_eq!(std::env::var("APP_ENV").unwrap(), "testing");
        assert_eq!(std::env::var("QUOTED_VAL").unwrap(), "hello world");
        assert_eq!(std::env::var("SINGLE_QUOTED").unwrap(), "single");
        assert_eq!(std::env::var("SCHEDULE_URL").unwrap(), "webcal://example.com/calendar.ics");
    }
}
