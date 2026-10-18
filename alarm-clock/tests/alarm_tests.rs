use std::sync::Arc;
use chrono::{Datelike, Local, TimeZone, Weekday};
use pomelo_hal::Board;
use pomelo_widgets::Language;

use alarm_clock::model::{AlarmConfig, AlarmItem, AlarmRepeat};
use alarm_clock::{AlarmClock, Message, ViewMode};

#[test]
fn default_config_has_sensible_alarms() {
    let cfg = AlarmConfig::default();
    assert_eq!(cfg.alarms.len(), 2);

    let first = &cfg.alarms[0];
    assert_eq!(first.hour, 7);
    assert_eq!(first.minute, 0);
    assert!(first.enabled);
    assert_eq!(first.repeat, AlarmRepeat::Everyday);
    assert_eq!(first.time_string(), "07:00");

    let second = &cfg.alarms[1];
    assert_eq!(second.hour, 8);
    assert_eq!(second.minute, 30);
    assert!(!second.enabled);
    assert_eq!(second.repeat, AlarmRepeat::Weekdays);
    assert_eq!(second.time_string(), "08:30");
}

#[test]
fn repeat_weekday_matching() {
    // Weekday: Monday
    assert!(AlarmRepeat::Once.matches_weekday(Weekday::Mon));
    assert!(AlarmRepeat::Everyday.matches_weekday(Weekday::Mon));
    assert!(AlarmRepeat::Weekdays.matches_weekday(Weekday::Mon));
    assert!(!AlarmRepeat::Weekends.matches_weekday(Weekday::Mon));

    // Weekend: Saturday
    assert!(AlarmRepeat::Once.matches_weekday(Weekday::Sat));
    assert!(AlarmRepeat::Everyday.matches_weekday(Weekday::Sat));
    assert!(!AlarmRepeat::Weekdays.matches_weekday(Weekday::Sat));
    assert!(AlarmRepeat::Weekends.matches_weekday(Weekday::Sat));

    // Cycling
    assert_eq!(AlarmRepeat::Once.next(), AlarmRepeat::Everyday);
    assert_eq!(AlarmRepeat::Everyday.next(), AlarmRepeat::Weekdays);
    assert_eq!(AlarmRepeat::Weekdays.next(), AlarmRepeat::Weekends);
    assert_eq!(AlarmRepeat::Weekends.next(), AlarmRepeat::Once);
}

#[test]
fn alarm_should_ring_matching() {
    let alarm = AlarmItem::new(1, 7, 30, "上班", AlarmRepeat::Weekdays);

    // Monday 07:30 -> match
    let mon_730 = Local.with_ymd_and_hms(2026, 10, 19, 7, 30, 0).unwrap(); // 2026-10-19 is Monday
    assert_eq!(mon_730.weekday(), Weekday::Mon);
    assert!(alarm.should_ring(mon_730));

    // Monday 07:31 -> no match (minute mismatch)
    let mon_731 = Local.with_ymd_and_hms(2026, 10, 19, 7, 31, 0).unwrap();
    assert!(!alarm.should_ring(mon_731));

    // Sunday 07:30 -> no match (weekend)
    let sun_730 = Local.with_ymd_and_hms(2026, 10, 18, 7, 30, 0).unwrap(); // 2026-10-18 is Sunday
    assert_eq!(sun_730.weekday(), Weekday::Sun);
    assert!(!alarm.should_ring(sun_730));

    // Disabled alarm -> no match
    let mut disabled_alarm = alarm.clone();
    disabled_alarm.enabled = false;
    assert!(!disabled_alarm.should_ring(mon_730));
}

#[test]
fn config_upsert_and_toggle_and_remove() {
    let mut cfg = AlarmConfig::default();
    let initial_count = cfg.alarms.len();

    // Add a new alarm
    let new_item = AlarmItem::new(0, 6, 15, "晨跑", AlarmRepeat::Weekends);
    let assigned_id = cfg.upsert(new_item);
    assert_eq!(cfg.alarms.len(), initial_count + 1);

    // Should be sorted by time (06:15 is before 07:00)
    assert_eq!(cfg.alarms[0].id, assigned_id);
    assert_eq!(cfg.alarms[0].time_string(), "06:15");

    // Toggle
    assert!(cfg.alarms[0].enabled);
    cfg.toggle(assigned_id);
    assert!(!cfg.get(assigned_id).unwrap().enabled);
    cfg.toggle(assigned_id);
    assert!(cfg.get(assigned_id).unwrap().enabled);

    // Remove
    cfg.remove(assigned_id);
    assert_eq!(cfg.alarms.len(), initial_count);
    assert!(cfg.get(assigned_id).is_none());
}

#[test]
fn localized_subtitles() {
    let alarm = AlarmItem::new(1, 8, 0, "起床闹钟", AlarmRepeat::Weekdays);
    assert_eq!(
        alarm.subtitle_string(Language::Chinese),
        "起床闹钟 · 工作日"
    );
    assert_eq!(
        alarm.subtitle_string(Language::English),
        "起床闹钟 · Weekdays"
    );
}

#[test]
fn app_update_flow_add_edit_delete() {
    let board = Arc::new(Board::simulated());
    let mut app = AlarmClock::with_board(board);

    // Open new alarm dialog
    let _ = app.update(Message::OpenNew);
    assert!(matches!(app.mode, ViewMode::Edit { is_new: true, .. }));

    // Adjust hour and minute
    let _ = app.update(Message::PickerHourChange(2));
    let _ = app.update(Message::PickerMinuteChange(15));
    let _ = app.update(Message::SelectLabel("晨练".to_string()));
    let _ = app.update(Message::CycleRepeat);

    // Save
    let count_before = app.config.alarms.len();
    let _ = app.update(Message::SaveEdit);
    assert!(matches!(app.mode, ViewMode::List { .. }));
    assert_eq!(app.config.alarms.len(), count_before + 1);

    let saved = app.config.alarms.iter().find(|a| a.label == "晨练").expect("晨练 alarm saved");
    let saved_id = saved.id;

    // Open edit for this alarm
    let _ = app.update(Message::OpenEdit(saved_id));
    assert!(matches!(app.mode, ViewMode::Edit { is_new: false, .. }));

    // Delete
    let _ = app.update(Message::DeleteAlarm(saved_id));
    assert!(matches!(app.mode, ViewMode::List { .. }));
    assert_eq!(app.config.alarms.len(), count_before);
}

#[test]
fn ringing_dismiss_and_snooze() {
    let board = Arc::new(Board::simulated());
    let mut app = AlarmClock::with_board(board);

    // Simulate ringing mode
    app.mode = ViewMode::Ringing {
        alarm_id: 1,
        time_str: "07:00".to_string(),
        label: "起床闹钟".to_string(),
        sound_id: 0,
    };

    // Snooze
    let _ = app.update(Message::SnoozeRinging);
    assert!(matches!(app.mode, ViewMode::List { .. }));

    // Dismiss
    app.mode = ViewMode::Ringing {
        alarm_id: 1,
        time_str: "07:00".to_string(),
        label: "起床闹钟".to_string(),
        sound_id: 0,
    };
    let _ = app.update(Message::DismissRinging);
    assert!(matches!(app.mode, ViewMode::List { .. }));
}
