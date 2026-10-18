use std::sync::Arc;
use chrono::{Datelike, Local, TimeZone, Weekday};
use pomelo_hal::Board;
use pomelo_widgets::Language;

use alarm_clock::model::{AlarmConfig, AlarmItem, AlarmRepeat};
use alarm_clock::{AlarmClock, AlarmRoute, Message, ViewMode};

#[test]
fn default_config_has_sensible_alarms() {
    let cfg = AlarmConfig::default();
    assert_eq!(cfg.alarms.len(), 2);

    let first = &cfg.alarms[0];
    assert_eq!(first.hour, 7);
    assert_eq!(first.minute, 0);
    assert!(first.enabled);
    assert_eq!(first.repeat, AlarmRepeat::everyday());
    assert_eq!(first.time_string(), "07:00");

    let second = &cfg.alarms[1];
    assert_eq!(second.hour, 8);
    assert_eq!(second.minute, 30);
    assert!(!second.enabled);
    assert_eq!(second.repeat, AlarmRepeat::weekdays());
    assert_eq!(second.time_string(), "08:30");
}

#[test]
fn repeat_schedule_and_weekday_matching() {
    // Default / Never
    let never = AlarmRepeat::never();
    assert!(never.is_never());
    assert_eq!(never.display_summary(Language::Chinese), "永不");
    assert_eq!(never.display_summary(Language::English), "Never");
    assert!(never.matches_weekday(Weekday::Mon));
    assert!(never.matches_weekday(Weekday::Sun));

    // Everyday
    let everyday = AlarmRepeat::everyday();
    assert!(everyday.is_everyday());
    assert_eq!(everyday.display_summary(Language::Chinese), "每天");
    assert_eq!(everyday.display_summary(Language::English), "Every day");
    assert!(everyday.matches_weekday(Weekday::Mon));
    assert!(everyday.matches_weekday(Weekday::Sun));

    // Weekdays
    let weekdays = AlarmRepeat::weekdays();
    assert!(weekdays.is_weekdays());
    assert_eq!(weekdays.display_summary(Language::Chinese), "工作日");
    assert_eq!(weekdays.display_summary(Language::English), "Weekdays");
    assert!(weekdays.matches_weekday(Weekday::Mon));
    assert!(weekdays.matches_weekday(Weekday::Fri));
    assert!(!weekdays.matches_weekday(Weekday::Sat));
    assert!(!weekdays.matches_weekday(Weekday::Sun));

    // Weekends
    let weekends = AlarmRepeat::weekends();
    assert!(weekends.is_weekends());
    assert_eq!(weekends.display_summary(Language::Chinese), "周末");
    assert_eq!(weekends.display_summary(Language::English), "Weekends");
    assert!(!weekends.matches_weekday(Weekday::Mon));
    assert!(weekends.matches_weekday(Weekday::Sat));
    assert!(weekends.matches_weekday(Weekday::Sun));

    // Toggling specific days
    let mut custom = AlarmRepeat::never();
    // 0: Sunday, 1: Monday, 2: Tuesday, 3: Wednesday, 4: Thursday, 5: Friday, 6: Saturday
    custom.toggle_day(1); // Monday
    custom.toggle_day(3); // Wednesday
    assert!(!custom.is_never());
    assert!(custom.is_day_selected(1));
    assert!(custom.is_day_selected(3));
    assert!(!custom.is_day_selected(2));
    assert_eq!(custom.display_summary(Language::Chinese), "周一 周三");
    assert_eq!(custom.display_summary(Language::English), "Mon Wed");
}

#[test]
fn alarm_should_ring_matching() {
    let alarm = AlarmItem::new(1, 7, 30, "上班", AlarmRepeat::weekdays());

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
    let new_item = AlarmItem::new(0, 6, 15, "晨跑", AlarmRepeat::weekends());
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
    let alarm = AlarmItem::new(1, 8, 0, "起床闹钟", AlarmRepeat::weekdays());
    assert_eq!(
        alarm.subtitle_string(Language::Chinese),
        "起床闹钟 · 工作日"
    );
    assert_eq!(
        alarm.subtitle_string(Language::English),
        "起床闹钟 · Weekdays"
    );

    let never_alarm = AlarmItem::new(2, 9, 0, "单次闹钟", AlarmRepeat::never());
    assert_eq!(
        never_alarm.subtitle_string(Language::Chinese),
        "单次闹钟 · 永不"
    );
    assert_eq!(
        never_alarm.subtitle_string(Language::English),
        "单次闹钟 · Never"
    );
}

#[test]
fn app_update_flow_add_edit_repeat_and_delete() {
    let board = Arc::new(Board::simulated());
    let mut app = AlarmClock::with_board(board);

    // Open new alarm dialog -> ActionSheet Edit mode
    let _ = app.update(Message::OpenNew);
    assert!(matches!(app.mode, ViewMode::Edit { is_new: true, .. }));

    // By default, new alarm repeat is "永不" (Never)
    if let ViewMode::Edit { draft, .. } = &app.mode {
        assert!(draft.repeat.is_never());
        assert_eq!(draft.repeat.display_summary(Language::Chinese), "永不");
    }

    // Adjust hour and minute via roller stepper
    let _ = app.update(Message::HourChange(2));
    let _ = app.update(Message::MinuteChange(15));
    let _ = app.update(Message::SelectLabel("晨练".to_string()));

    // Open repeat schedule sub-sheet
    let _ = app.update(Message::OpenRoute(AlarmRoute::Repeat));
    assert!(matches!(
        app.mode,
        ViewMode::Edit {
            sub_sheet: Some(AlarmRoute::Repeat),
            ..
        }
    ));

    // Select Monday (idx 1) and Wednesday (idx 3)
    let _ = app.update(Message::ToggleRepeatDay(1));
    let _ = app.update(Message::ToggleRepeatDay(3));

    // Close repeat sheet and return to Edit sheet
    let _ = app.update(Message::CloseSubSheet);
    assert!(matches!(
        app.mode,
        ViewMode::Edit {
            sub_sheet: None,
            ..
        }
    ));

    if let ViewMode::Edit { draft, .. } = &app.mode {
        assert_eq!(draft.repeat.display_summary(Language::Chinese), "周一 周三");
        assert!(draft.repeat.is_day_selected(1));
        assert!(draft.repeat.is_day_selected(3));
    }

    // Toggle snooze switch
    let _ = app.update(Message::ToggleSnooze);
    if let ViewMode::Edit { draft, .. } = &app.mode {
        assert!(!draft.snooze_enabled);
    }

    // Save
    let count_before = app.config.alarms.len();
    let _ = app.update(Message::SaveEdit);
    assert!(matches!(app.mode, ViewMode::List { .. }));
    assert_eq!(app.config.alarms.len(), count_before + 1);

    let saved = app.config.alarms.iter().find(|a| a.label == "晨练").expect("晨练 alarm saved");
    let saved_id = saved.id;
    assert_eq!(saved.repeat.display_summary(Language::Chinese), "周一 周三");

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

#[test]
fn sub_sheets_open_and_select() {
    let board = Arc::new(Board::simulated());
    let mut app = AlarmClock::with_board(board);

    let _ = app.update(Message::OpenNew);

    // Label sub-sheet
    let _ = app.update(Message::OpenRoute(AlarmRoute::Label));
    assert!(matches!(
        app.mode,
        ViewMode::Edit {
            sub_sheet: Some(AlarmRoute::Label),
            ..
        }
    ));
    let _ = app.update(Message::SelectLabel("工作".to_string()));
    let _ = app.update(Message::CloseSubSheet);
    if let ViewMode::Edit { draft, sub_sheet, .. } = &app.mode {
        assert_eq!(sub_sheet, &None);
        assert_eq!(draft.label, "工作");
    }

    // Sound sub-sheet
    let _ = app.update(Message::OpenRoute(AlarmRoute::Sound));
    assert!(matches!(
        app.mode,
        ViewMode::Edit {
            sub_sheet: Some(AlarmRoute::Sound),
            ..
        }
    ));
    let _ = app.update(Message::SelectSound(2));
    let _ = app.update(Message::CloseSubSheet);
    if let ViewMode::Edit { draft, sub_sheet, .. } = &app.mode {
        assert_eq!(sub_sheet, &None);
        assert_eq!(draft.sound_id, 2);
    }

    // SnoozeDuration sub-sheet
    let _ = app.update(Message::OpenRoute(AlarmRoute::SnoozeDuration));
    assert!(matches!(
        app.mode,
        ViewMode::Edit {
            sub_sheet: Some(AlarmRoute::SnoozeDuration),
            ..
        }
    ));
    let _ = app.update(Message::SelectSnoozeMinutes(15));
    let _ = app.update(Message::CloseSubSheet);
    if let ViewMode::Edit { draft, sub_sheet, .. } = &app.mode {
        assert_eq!(sub_sheet, &None);
        assert_eq!(draft.snooze_minutes, 15);
    }
}
