use std::io;

use chrono::{DateTime, Local, NaiveDate};
use pomelo_hal::PersistentConfig;
use serde::{Deserialize, Serialize};

/// Directory name under AppData for daily schedule storage.
pub const APP_DATA_SUBDIR: &str = "DAILY_SCHEDULE";
/// Key under Flash NVS / KV storage for today's parsed events cache.
pub const TODAY_CACHE_KEY: &str = "today_cache";

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

/// Persistent cache of today's and tomorrow's schedule events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TodayCache {
    /// Date of the cached events (today) in "YYYY-MM-DD" format.
    pub date: String,
    /// Timestamp when this cache was last fetched and updated.
    pub updated_at: DateTime<Local>,
    /// The parsed events occurring on today.
    #[serde(alias = "today_events", default)]
    pub events: Vec<ScheduleEvent>,
    /// The parsed events occurring on tomorrow.
    #[serde(default)]
    pub tomorrow_events: Vec<ScheduleEvent>,
}

impl Default for TodayCache {
    fn default() -> Self {
        Self {
            date: String::new(),
            updated_at: Local::now(),
            events: Vec::new(),
            tomorrow_events: Vec::new(),
        }
    }
}

impl PersistentConfig for TodayCache {
    const APP_ID: &'static str = APP_DATA_SUBDIR;
    const KEY: &'static str = TODAY_CACHE_KEY;
}

/// Type alias for schedule cache storing today and tomorrow.
pub type ScheduleCache = TodayCache;

use std::collections::BTreeMap;
use serde_json::Value;

/// A custom keyword-to-color preset rule for schedule event titles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColorRule {
    /// Keyword to match against event title (e.g. "会议").
    pub keyword: String,
    /// Preset color index (0..7).
    pub color_index: usize,
}

/// Configuration for the daily schedule application.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ScheduleConfig {
    /// The iCal (.ics) or webcal subscription URL.
    pub subscription_url: Option<String>,
    /// Custom keyword-to-color preset rules.
    #[serde(default)]
    pub color_rules: Vec<ColorRule>,
    /// Whether reminder chime sounds are enabled (defaults to false).
    #[serde(default)]
    pub reminder_sound: bool,
    #[serde(default, flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl PersistentConfig for ScheduleConfig {
    const APP_ID: &'static str = APP_DATA_SUBDIR;
}

/// Load configuration from KV storage, returning default if not found or invalid.
/// If subscription_url is not set in config, checks SCHEDULE_URL / DAILY_SCHEDULE_URL from env.
pub fn load_config() -> ScheduleConfig {
    let mut config = ScheduleConfig::load();
    if config.subscription_url.is_none() {
        if let Ok(url) = std::env::var("SCHEDULE_URL").or_else(|_| std::env::var("DAILY_SCHEDULE_URL")) {
            let trimmed = url.trim();
            if !trimmed.is_empty() {
                config.subscription_url = Some(trimmed.to_string());
            }
        }
    }
    config
}

/// Save configuration to persistent KV storage.
pub fn save_config(config: &ScheduleConfig) -> io::Result<()> {
    config.save()
}

/// Check and load schedule cache for today and tomorrow from Flash NVS / KV storage.
///
/// Returns `Some((today_events, tomorrow_events))` if:
/// 1. The cache was saved for `today` directly; OR
/// 2. The cache was saved yesterday AND has `tomorrow_events`, in which case yesterday's
///    tomorrow events are seamlessly promoted to today's events without requiring network access.
pub fn load_schedule_cache(today: NaiveDate) -> Option<(Vec<ScheduleEvent>, Vec<ScheduleEvent>)> {
    let cache = TodayCache::try_load().ok()?;

    let today_str = today.format("%Y-%m-%d").to_string();
    if cache.date == today_str {
        Some((cache.events, cache.tomorrow_events))
    } else if let Ok(cache_date) = NaiveDate::parse_from_str(&cache.date, "%Y-%m-%d") {
        if cache_date + chrono::Duration::days(1) == today && !cache.tomorrow_events.is_empty() {
            // 昨天的明天即是今天：无缝复用昨天的 tomorrow_events 作为今天的日程
            Some((cache.tomorrow_events, Vec::new()))
        } else {
            None
        }
    } else {
        None
    }
}

/// Check and load today's schedule cache from Flash NVS / KV storage.
pub fn load_today_cache(today: NaiveDate) -> Option<Vec<ScheduleEvent>> {
    load_schedule_cache(today).map(|(today_events, _)| today_events)
}

/// Save today's and tomorrow's events to Flash NVS / KV storage.
pub fn save_schedule_cache(
    today: NaiveDate,
    today_events: &[ScheduleEvent],
    tomorrow_events: &[ScheduleEvent],
) -> io::Result<()> {
    let cache = TodayCache {
        date: today.format("%Y-%m-%d").to_string(),
        updated_at: Local::now(),
        events: today_events.to_vec(),
        tomorrow_events: tomorrow_events.to_vec(),
    };

    cache.save()
}

/// Save today's events to Flash NVS / KV storage.
pub fn save_today_cache(today: NaiveDate, events: &[ScheduleEvent]) -> io::Result<()> {
    save_schedule_cache(today, events, &[])
}

/// Clears the today's schedule cache from Flash NVS / KV storage.
pub fn clear_today_cache() -> io::Result<()> {
    TodayCache::forget()
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
            tomorrow_events: vec![event.clone()],
        };

        let encoded = serde_json::to_string_pretty(&cache).expect("encode json");
        let decoded: TodayCache = serde_json::from_str(&encoded).expect("decode json");

        assert_eq!(decoded.date, "2026-10-14");
        assert_eq!(decoded.events.len(), 1);
        assert_eq!(decoded.tomorrow_events.len(), 1);
        assert_eq!(decoded.events[0].title, "团队周会");
    }

    #[test]
    fn schedule_config_preserves_unknown_fields() {
        let raw_json = r#"{
            "subscription_url": "https://example.com/cal.ics",
            "refresh_interval_mins": 30,
            "custom_category": "work"
        }"#;
        let config: ScheduleConfig = serde_json::from_str(raw_json).expect("decode json");
        assert_eq!(config.subscription_url, Some("https://example.com/cal.ics".to_string()));
        assert_eq!(config.extra.get("refresh_interval_mins"), Some(&serde_json::json!(30)));
        assert_eq!(config.extra.get("custom_category"), Some(&serde_json::json!("work")));

        let serialized = serde_json::to_string(&config).expect("encode json");
        assert!(serialized.contains("refresh_interval_mins"));
        assert!(serialized.contains("custom_category"));
    }
}
