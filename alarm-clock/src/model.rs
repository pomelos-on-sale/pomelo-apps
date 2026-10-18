//! Alarm Clock data models, repeat rules, and persistent storage.

use chrono::{DateTime, Datelike, Local, Timelike, Weekday};
use pomelo_hal::storage::PersistentConfig;
use pomelo_widgets::Language;
use serde::{Deserialize, Serialize};

/// Alarm repetition schedule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlarmRepeat {
    /// Rings only once and turns off automatically after triggering.
    Once,
    /// Rings every day.
    Everyday,
    /// Rings Monday through Friday.
    Weekdays,
    /// Rings Saturday and Sunday.
    Weekends,
}

impl AlarmRepeat {
    /// Returns the localized name of the repeat rule.
    pub fn display_name(&self, lang: Language) -> &'static str {
        match (self, lang) {
            (Self::Once, Language::Chinese) => "仅一次",
            (Self::Once, Language::English) => "Once",
            (Self::Everyday, Language::Chinese) => "每天",
            (Self::Everyday, Language::English) => "Every day",
            (Self::Weekdays, Language::Chinese) => "工作日",
            (Self::Weekdays, Language::English) => "Weekdays",
            (Self::Weekends, Language::Chinese) => "周末",
            (Self::Weekends, Language::English) => "Weekends",
        }
    }

    /// Checks if this repeat schedule includes `weekday`.
    pub fn matches_weekday(&self, weekday: Weekday) -> bool {
        match self {
            Self::Once | Self::Everyday => true,
            Self::Weekdays => matches!(
                weekday,
                Weekday::Mon | Weekday::Tue | Weekday::Wed | Weekday::Thu | Weekday::Fri
            ),
            Self::Weekends => matches!(weekday, Weekday::Sat | Weekday::Sun),
        }
    }

    /// Cycles to the next repeat schedule option.
    pub fn next(&self) -> Self {
        match self {
            Self::Once => Self::Everyday,
            Self::Everyday => Self::Weekdays,
            Self::Weekdays => Self::Weekends,
            Self::Weekends => Self::Once,
        }
    }
}

/// Available alarm ringtone melodies.
pub const ALARM_SOUND_NAMES: [(&str, &str); 3] = [
    ("经典雷达", "Classic Radar"),
    ("清晨乐声", "Morning Chime"),
    ("轻快节奏", "Upbeat Rhythm"),
];

/// Returns the localized ringtone name for `sound_id`.
pub fn sound_name(sound_id: u8, lang: Language) -> &'static str {
    let idx = (sound_id as usize) % ALARM_SOUND_NAMES.len();
    match lang {
        Language::Chinese => ALARM_SOUND_NAMES[idx].0,
        Language::English => ALARM_SOUND_NAMES[idx].1,
    }
}

/// Preset quick alarm labels.
pub const PRESET_LABELS: [&str; 6] = ["闹钟", "起床", "上班", "会议", "吃药", "运动"];

/// An individual alarm record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlarmItem {
    pub id: u32,
    pub hour: u8,       // 0..23
    pub minute: u8,     // 0..59
    pub label: String,  // e.g. "起床闹钟"
    pub enabled: bool,
    pub repeat: AlarmRepeat,
    pub sound_id: u8,
}

impl AlarmItem {
    /// Creates a new alarm item with sanitized hour and minute.
    pub fn new(id: u32, hour: u8, minute: u8, label: impl Into<String>, repeat: AlarmRepeat) -> Self {
        Self {
            id,
            hour: hour % 24,
            minute: minute % 60,
            label: label.into(),
            enabled: true,
            repeat,
            sound_id: 0,
        }
    }

    /// Formats the time as "HH:MM" (e.g. "07:30").
    pub fn time_string(&self) -> String {
        format!("{:02}:{:02}", self.hour, self.minute)
    }

    /// Formats 12-hour AM/PM indicator if desired.
    pub fn am_pm_string(&self) -> &'static str {
        if self.hour < 12 {
            "AM"
        } else {
            "PM"
        }
    }

    /// Subtitle string displayed below the time in the alarm list.
    pub fn subtitle_string(&self, lang: Language) -> String {
        let repeat_str = self.repeat.display_name(lang);
        let lbl = if self.label.trim().is_empty() {
            match lang {
                Language::Chinese => "闹钟",
                Language::English => "Alarm",
            }
        } else {
            self.label.as_str()
        };
        format!("{} · {}", lbl, repeat_str)
    }

    /// Checks if this alarm should ring right now at `now`.
    pub fn should_ring(&self, now: DateTime<Local>) -> bool {
        if !self.enabled {
            return false;
        }
        if self.hour != now.hour() as u8 || self.minute != now.minute() as u8 {
            return false;
        }
        self.repeat.matches_weekday(now.weekday())
    }
}

/// Persistent configuration for all alarms.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlarmConfig {
    pub alarms: Vec<AlarmItem>,
    pub next_id: u32,
    pub snooze_minutes: u8,
}

impl Default for AlarmConfig {
    fn default() -> Self {
        Self {
            alarms: vec![
                AlarmItem::new(1, 7, 0, "起床闹钟", AlarmRepeat::Everyday),
                AlarmItem {
                    id: 2,
                    hour: 8,
                    minute: 30,
                    label: "工作日闹钟".to_string(),
                    enabled: false,
                    repeat: AlarmRepeat::Weekdays,
                    sound_id: 1,
                },
            ],
            next_id: 3,
            snooze_minutes: 5,
        }
    }
}

impl PersistentConfig for AlarmConfig {
    const APP_ID: &'static str = "ALARM_CLOCK";
    const FILE_NAME: &'static str = "alarms.toml";
}

impl AlarmConfig {
    /// Loads the configuration from disk, falling back to default.
    pub fn load_config() -> Self {
        <Self as PersistentConfig>::load()
    }

    /// Adds or updates an alarm. If `item.id` is 0, assigns a new unique ID.
    pub fn upsert(&mut self, mut item: AlarmItem) -> u32 {
        if item.id == 0 {
            item.id = self.next_id;
            self.next_id = self.next_id.wrapping_add(1).max(1);
            self.alarms.push(item.clone());
        } else if let Some(existing) = self.alarms.iter_mut().find(|a| a.id == item.id) {
            *existing = item.clone();
        } else {
            self.alarms.push(item.clone());
        }
        // Keep alarms sorted by time (hour, minute)
        self.alarms.sort_by_key(|a| (a.hour, a.minute));
        let _ = self.save();
        item.id
    }

    /// Toggles the enabled state of an alarm by ID.
    pub fn toggle(&mut self, id: u32) {
        if let Some(item) = self.alarms.iter_mut().find(|a| a.id == id) {
            item.enabled = !item.enabled;
            let _ = self.save();
        }
    }

    /// Removes an alarm by ID.
    pub fn remove(&mut self, id: u32) {
        self.alarms.retain(|a| a.id != id);
        let _ = self.save();
    }

    /// Retrieves an alarm item by ID.
    pub fn get(&self, id: u32) -> Option<&AlarmItem> {
        self.alarms.iter().find(|a| a.id == id)
    }

    /// Checks if any alarm is currently enabled.
    pub fn has_active_alarms(&self) -> bool {
        self.alarms.iter().any(|a| a.enabled)
    }
}
