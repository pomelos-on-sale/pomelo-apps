//! Environment configuration loader for Pomelo UI.
//!
//! Automatically loads environment variables from `AppData/.env` (or project root `.env`),
//! injecting system settings (such as `TZ=CST-8`) into `std::env`.

use std::fs;
use std::path::PathBuf;

#[cfg(unix)]
extern "C" {
    fn tzset();
}

/// Loads environment configuration from `AppData/.env` into the process environment.
pub fn load_environment() {
    let mut loaded_files = 0;
    let mut loaded_paths = Vec::new();

    // 1. Check Flash NVS / KV storage for environment settings
    if let Ok(Some(bytes)) = pomelo_hal::storage::kv_get_raw("SYSTEM", "env") {
        if let Ok(content) = std::str::from_utf8(&bytes) {
            println!("[env] loading configuration from NVS ('SYSTEM/env')");
            parse_and_inject_env(content);
            loaded_files += 1;
        }
    }

    let env_paths = candidate_env_paths();
    for path in env_paths {
        if path.is_file() {
            let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
            if !loaded_paths.contains(&canonical) {
                if let Ok(content) = fs::read_to_string(&path) {
                    println!("[env] loading configuration from {:?}", path);
                    parse_and_inject_env(&content);
                    loaded_files += 1;
                    loaded_paths.push(canonical);
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

    println!(
        "[env] loaded {loaded_files} configuration file(s), active TZ={:?}",
        std::env::var("TZ").unwrap_or_default()
    );
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

/// Returns potential file paths where `.env` may reside for developer convenience on host.
fn candidate_env_paths() -> Vec<PathBuf> {
    #[allow(unused_mut)]
    let mut paths = Vec::new();

    // Developer convenience on desktop simulator: check relative asset directories & project root
    #[cfg(not(target_os = "espidf"))]
    {
        for candidate in &[
            "assets/.env",
            "../assets/.env",
            "../../assets/.env",
            ".env",
            "../.env",
            "../../.env",
        ] {
            let p = PathBuf::from(candidate);
            if !paths.contains(&p) {
                paths.push(p);
            }
        }
    }

    paths
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
