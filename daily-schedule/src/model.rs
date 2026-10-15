use std::io;
use std::path::PathBuf;

use chrono::{DateTime, Local, NaiveDate};
use pomelo_hal::storage::{self, PersistentConfig};
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

use std::collections::BTreeMap;
use toml::Value;

/// Configuration for the daily schedule application.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ScheduleConfig {
    /// The iCal (.ics) or webcal subscription URL.
    pub subscription_url: Option<String>,
    #[serde(default, flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl PersistentConfig for ScheduleConfig {
    const APP_ID: &'static str = APP_DATA_SUBDIR;
    const FILE_NAME: &'static str = CONFIG_FILE;
}

/// Resolves the storage directory for daily-schedule app data.
pub fn storage_dir() -> PathBuf {
    storage::app_dir(APP_DATA_SUBDIR)
}

/// Path to config.toml.
pub fn config_path() -> PathBuf {
    ScheduleConfig::config_path()
}

/// Path to today_cache.toml.
pub fn cache_path() -> PathBuf {
    storage::app_file_path(APP_DATA_SUBDIR, TODAY_CACHE_FILE)
}

/// Load configuration from disk, returning default if not found or invalid.
pub fn load_config() -> ScheduleConfig {
    ScheduleConfig::load()
}

/// Save configuration to disk atomically.
pub fn save_config(config: &ScheduleConfig) -> io::Result<()> {
    config.save()
}

/// Check and load today's schedule cache.
///
/// Returns `Some(events)` strictly if the cache exists AND its cached date
/// matches `today`. If the cache is missing, corrupted, or belongs to another
/// day (yesterday or older), returns `None`.
pub fn load_today_cache(today: NaiveDate) -> Option<Vec<ScheduleEvent>> {
    let cache: TodayCache = storage::load_toml(cache_path()).ok()?;

    let today_str = today.format("%Y-%m-%d").to_string();
    if cache.date == today_str {
        Some(cache.events)
    } else {
        // Cache exists but is expired / not for today
        None
    }
}

/// Save today's events to the cache atomically.
pub fn save_today_cache(today: NaiveDate, events: &[ScheduleEvent]) -> io::Result<()> {
    let cache = TodayCache {
        date: today.format("%Y-%m-%d").to_string(),
        updated_at: Local::now(),
        events: events.to_vec(),
    };

    storage::save_toml_atomic(cache_path(), &cache)
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

    #[test]
    fn schedule_config_preserves_unknown_fields() {
        let raw_toml = r#"
subscription_url = "https://example.com/cal.ics"
refresh_interval_mins = 30
custom_category = "work"
"#;
        let config: ScheduleConfig = toml::from_str(raw_toml).expect("decode toml");
        assert_eq!(config.subscription_url, Some("https://example.com/cal.ics".to_string()));
        assert_eq!(config.extra.get("refresh_interval_mins"), Some(&toml::Value::Integer(30)));
        assert_eq!(config.extra.get("custom_category"), Some(&toml::Value::String("work".to_string())));

        let serialized = toml::to_string(&config).expect("encode toml");
        assert!(serialized.contains("refresh_interval_mins = 30"));
        assert!(serialized.contains("custom_category = \"work\""));
    }
}
