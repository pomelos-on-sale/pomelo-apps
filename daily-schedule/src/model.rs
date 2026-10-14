use std::fs;
use std::io;
use std::path::PathBuf;

use chrono::{DateTime, Local, NaiveDate};
use pomelo_widgets::preferences::resolve_app_data_root;
use serde::{Deserialize, Serialize};

/// Directory name under AppData for daily schedule storage.
pub const APP_DATA_SUBDIR: &str = "DAILY_SCHEDULE";
/// Configuration file storing subscription URL and settings.
pub const CONFIG_FILE: &str = "config.toml";
/// Cache file storing today's parsed events.
pub const TODAY_CACHE_FILE: &str = "today_cache.toml";

/// A single calendar event to be displayed on the daily timeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduleEvent {
    pub id: String,
    pub title: String,
    pub start: DateTime<Local>,
    pub end: DateTime<Local>,
    pub is_all_day: bool,
    /// 0..4 color index for color-coding different events on the timeline
    pub color_index: usize,
}

/// Persistent cache of today's schedule events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TodayCache {
    /// Date of the cached events in "YYYY-MM-DD" format.
    pub date: String,
    /// Timestamp when this cache was last fetched and updated.
    pub updated_at: DateTime<Local>,
    /// The parsed events occurring on this date.
    pub events: Vec<ScheduleEvent>,
}

/// Configuration for the daily schedule application.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduleConfig {
    /// The iCal (.ics) or webcal subscription URL.
    pub subscription_url: Option<String>,
}

/// Resolves the storage directory for daily-schedule app data.
pub fn storage_dir() -> PathBuf {
    if let Ok(custom) = std::env::var("POMELO_APP_DATA") {
        return PathBuf::from(custom).join(APP_DATA_SUBDIR);
    }
    let base = resolve_app_data_root();
    let dir = base.join(APP_DATA_SUBDIR);
    if fs::create_dir_all(&dir).is_ok() {
        dir
    } else {
        std::env::temp_dir().join("pomelo").join(APP_DATA_SUBDIR)
    }
}

/// Path to config.toml.
pub fn config_path() -> PathBuf {
    storage_dir().join(CONFIG_FILE)
}

/// Path to today_cache.toml.
pub fn cache_path() -> PathBuf {
    storage_dir().join(TODAY_CACHE_FILE)
}

/// Load configuration from disk, returning default if not found or invalid.
pub fn load_config() -> ScheduleConfig {
    let path = config_path();
    if let Ok(content) = fs::read_to_string(&path) {
        if let Ok(config) = toml::from_str(&content) {
            return config;
        }
    }
    ScheduleConfig::default()
}

/// Save configuration to disk.
pub fn save_config(config: &ScheduleConfig) -> io::Result<()> {
    let dir = storage_dir();
    fs::create_dir_all(&dir)?;
    let content = toml::to_string_pretty(config).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    fs::write(config_path(), content)
}

/// Check and load today's schedule cache.
///
/// Returns `Some(events)` strictly if the cache exists AND its cached date
/// matches `today`. If the cache is missing, corrupted, or belongs to another
/// day (yesterday or older), returns `None`.
pub fn load_today_cache(today: NaiveDate) -> Option<Vec<ScheduleEvent>> {
    let path = cache_path();
    let content = fs::read_to_string(&path).ok()?;
    let cache: TodayCache = toml::from_str(&content).ok()?;

    let today_str = today.format("%Y-%m-%d").to_string();
    if cache.date == today_str {
        Some(cache.events)
    } else {
        // Cache exists but is expired / not for today
        None
    }
}

/// Save today's events to the cache.
pub fn save_today_cache(today: NaiveDate, events: &[ScheduleEvent]) -> io::Result<()> {
    let dir = storage_dir();
    fs::create_dir_all(&dir)?;

    let cache = TodayCache {
        date: today.format("%Y-%m-%d").to_string(),
        updated_at: Local::now(),
        events: events.to_vec(),
    };

    let content = toml::to_string_pretty(&cache).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    fs::write(cache_path(), content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn schedule_event_serialization_roundtrip() {
        let dt1 = Local.with_ymd_and_hms(2026, 10, 14, 9, 30, 0).unwrap();
        let dt2 = Local.with_ymd_and_hms(2026, 10, 14, 10, 30, 0).unwrap();

        let event = ScheduleEvent {
            id: "evt-001".to_string(),
            title: "团队周会".to_string(),
            start: dt1,
            end: dt2,
            is_all_day: false,
            color_index: 1,
        };

        let cache = TodayCache {
            date: "2026-10-14".to_string(),
            updated_at: dt1,
            events: vec![event.clone()],
        };

        let encoded = toml::to_string_pretty(&cache).expect("encode toml");
        let decoded: TodayCache = toml::from_str(&encoded).expect("decode toml");

        assert_eq!(decoded.date, "2026-10-14");
        assert_eq!(decoded.events.len(), 1);
        assert_eq!(decoded.events[0].title, "团队周会");
    }
}
