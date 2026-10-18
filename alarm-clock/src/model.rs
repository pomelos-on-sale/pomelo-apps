//! Alarm Clock data models, repeat rules, and persistent storage.

use chrono::{DateTime, Datelike, Local, Timelike, Weekday};
use pomelo_hal::storage::PersistentConfig;
use pomelo_widgets::Language;
use serde::{Deserialize, Serialize};

/// Days of the week in Sunday-to-Saturday order matching iOS Clock.
/// 0: Sunday, 1: Monday, 2: Tuesday, 3: Wednesday, 4: Thursday, 5: Friday, 6: Saturday.
pub const DAY_NAMES: [(&str, &str, &str, &str); 7] = [
    ("每周日", "Every Sunday", "周日", "Sun"),
    ("每周一", "Every Monday", "周一", "Mon"),
    ("每周二", "Every Tuesday", "周二", "Tue"),
    ("每周三", "Every Wednesday", "周三", "Wed"),
    ("每周四", "Every Thursday", "周四", "Thu"),
    ("每周五", "Every Friday", "周五", "Fri"),
    ("每周六", "Every Saturday", "周六", "Sat"),
];

/// Alarm repeat schedule.
///
/// Stores boolean flags for each of the 7 days of the week starting from Sunday:
/// `[Sun, Mon, Tue, Wed, Thu, Fri, Sat]`.
/// When all days are false, it defaults to "永不" (Never).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlarmRepeat {
    pub days: [bool; 7],
}

impl Default for AlarmRepeat {
    fn default() -> Self {
        Self::never()
    }
}

impl AlarmRepeat {
    /// Creates a schedule that repeats never (default: all days unselected).
    pub const fn never() -> Self {
        Self { days: [false; 7] }
    }

    /// Creates a schedule repeating everyday.
    pub const fn everyday() -> Self {
        Self { days: [true; 7] }
    }

    /// Creates a schedule repeating Monday through Friday.
    pub const fn weekdays() -> Self {
        Self {
            days: [false, true, true, true, true, true, false],
        }
    }

    /// Creates a schedule repeating Saturday and Sunday.
    pub const fn weekends() -> Self {
        Self {
            days: [true, false, false, false, false, false, true],
        }
    }

    /// Checks if no days are selected ("永不" / Never).
    pub fn is_never(&self) -> bool {
        !self.days.iter().any(|&d| d)
    }

    /// Checks if all 7 days are selected ("每天" / Everyday).
    pub fn is_everyday(&self) -> bool {
        self.days.iter().all(|&d| d)
    }

    /// Checks if only Monday through Friday are selected ("工作日" / Weekdays).
    pub fn is_weekdays(&self) -> bool {
        self.days == [false, true, true, true, true, true, false]
    }

    /// Checks if only Saturday and Sunday are selected ("周末" / Weekends).
    pub fn is_weekends(&self) -> bool {
        self.days == [true, false, false, false, false, false, true]
    }

    /// Toggles the repeat selection for the given day index (0..6).
    pub fn toggle_day(&mut self, day_idx: usize) {
        if day_idx < 7 {
            self.days[day_idx] = !self.days[day_idx];
        }
    }

    /// Checks if the day at `day_idx` (0..6) is selected.
    pub fn is_day_selected(&self, day_idx: usize) -> bool {
        day_idx < 7 && self.days[day_idx]
    }

    /// Converts a [`Weekday`] to index in 0..6 (Sun = 0, Mon = 1, ..., Sat = 6).
    pub fn weekday_to_idx(weekday: Weekday) -> usize {
        match weekday {
            Weekday::Sun => 0,
            Weekday::Mon => 1,
            Weekday::Tue => 2,
            Weekday::Wed => 3,
            Weekday::Thu => 4,
            Weekday::Fri => 5,
            Weekday::Sat => 6,
        }
    }

    /// Checks if this repeat schedule includes `weekday`.
    pub fn matches_weekday(&self, weekday: Weekday) -> bool {
        if self.is_never() {
            // When "Never" is selected, the alarm triggers once on the matching time
            true
        } else {
            let idx = Self::weekday_to_idx(weekday);
            self.days[idx]
        }
    }

    /// Formats the summary string displayed on the right side of the "重复" row.
    pub fn display_summary(&self, lang: Language) -> String {
        if self.is_never() {
            match lang {
                Language::Chinese => "永不".to_string(),
                Language::English => "Never".to_string(),
            }
        } else if self.is_everyday() {
            match lang {
                Language::Chinese => "每天".to_string(),
                Language::English => "Every day".to_string(),
            }
        } else if self.is_weekdays() {
            match lang {
                Language::Chinese => "工作日".to_string(),
                Language::English => "Weekdays".to_string(),
            }
        } else if self.is_weekends() {
            match lang {
                Language::Chinese => "周末".to_string(),
                Language::English => "Weekends".to_string(),
            }
        } else {
            let mut parts = Vec::new();
            for (idx, &selected) in self.days.iter().enumerate() {
                if selected {
                    let short_name = match lang {
                        Language::Chinese => DAY_NAMES[idx].2,
                        Language::English => DAY_NAMES[idx].3,
                    };
                    parts.push(short_name);
                }
            }
            parts.join(" ")
        }
    }
}

/// Available alarm ringtone melodies.
pub const ALARM_SOUND_NAMES: [(&str, &str); 3] = [
    ("雷达", "Radar"),
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

/// Preset labels for quick selection.
pub const PRESET_LABELS: [&str; 8] = [
    "闹钟", "起床", "工作", "晨练", "纪念日", "吃药", "喝水", "休息",
];

/// Preset snooze durations in minutes.
pub const SNOOZE_PRESETS: [u8; 5] = [5, 10, 15, 20, 30];

/// An individual alarm record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlarmItem {
    pub id: u32,
    pub hour: u8,       // 0..23
    pub minute: u8,     // 0..59
    pub label: String,  // e.g. "闹钟"
    pub enabled: bool,
    pub repeat: AlarmRepeat,
    pub sound_id: u8,
    pub snooze_enabled: bool,
    pub snooze_minutes: u8,
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
            snooze_enabled: true,
            snooze_minutes: 5,
        }
    }

    /// Formats the time as "HH:MM" (e.g. "07:30").
    pub fn time_string(&self) -> String {
        format!("{:02}:{:02}", self.hour, self.minute)
    }

    /// Subtitle string displayed below the time in the alarm list.
    pub fn subtitle_string(&self, lang: Language) -> String {
        let lbl = if self.label.trim().is_empty() {
            match lang {
                Language::Chinese => "闹钟",
                Language::English => "Alarm",
            }
        } else {
            self.label.as_str()
        };
        if self.repeat.is_never() {
            lbl.to_string()
        } else {
            let repeat_str = self.repeat.display_summary(lang);
            format!("{}  {}", lbl, repeat_str)
        }
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
}

impl Default for AlarmConfig {
    fn default() -> Self {
        Self {
            alarms: vec![
                AlarmItem {
                    id: 1,
                    hour: 7,
                    minute: 0,
                    label: "起床".to_string(),
                    enabled: false,
                    repeat: AlarmRepeat::everyday(),
                    sound_id: 0,
                    snooze_enabled: true,
                    snooze_minutes: 5,
                },
                AlarmItem {
                    id: 2,
                    hour: 8,
                    minute: 30,
                    label: "工作".to_string(),
                    enabled: false,
                    repeat: AlarmRepeat::weekdays(),
                    sound_id: 1,
                    snooze_enabled: true,
                    snooze_minutes: 5,
                },
            ],
            next_id: 3,
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
