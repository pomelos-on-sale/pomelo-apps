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
    assert!(!first.enabled);
    assert_eq!(first.label, "起床");
    assert_eq!(first.repeat, AlarmRepeat::everyday());
    assert_eq!(first.time_string(), "07:00");

    let second = &cfg.alarms[1];
    assert_eq!(second.hour, 8);
    assert_eq!(second.minute, 30);
    assert!(!second.enabled);
    assert_eq!(second.label, "工作");
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
        "起床闹钟  工作日"
    );
    assert_eq!(
        alarm.subtitle_string(Language::English),
        "起床闹钟  Weekdays"
    );

    let never_alarm = AlarmItem::new(2, 9, 0, "单次闹钟", AlarmRepeat::never());
    assert_eq!(
        never_alarm.subtitle_string(Language::Chinese),
        "单次闹钟"
    );
    assert_eq!(
        never_alarm.subtitle_string(Language::English),
        "单次闹钟"
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

    // Close repeat sheet and return to Edit sheet via Back
    let _ = app.update(Message::Back);
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

    // Verify editing a disabled alarm and clicking Save (SaveEdit / checkmark) sets it to enabled: true
    let disabled_id = app.config.alarms[0].id;
    app.config.alarms[0].enabled = false;
    let _ = app.update(Message::OpenEdit(disabled_id));
    let _ = app.update(Message::SaveEdit);
    let updated = app.config.get(disabled_id).expect("found updated alarm");
    assert!(updated.enabled, "Saving an edited alarm must set enabled to true");
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
        session_id: 1,
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
        session_id: 1,
    };
    let _ = app.update(Message::DismissRinging);
    assert!(matches!(app.mode, ViewMode::List { .. }));
}

#[test]
fn ringing_auto_dismisses_after_session_timeout() {
    let board = Arc::new(Board::simulated());
    let mut app = AlarmClock::with_board(board);

    app.mode = ViewMode::Ringing {
        alarm_id: 1,
        time_str: "07:00".to_string(),
        label: "起床闹钟".to_string(),
        sound_id: 0,
        session_id: 1,
    };

    // Stale timeout with wrong session_id is safely ignored
    let _ = app.update(Message::RingTimeout { session_id: 999 });
    assert!(matches!(app.mode, ViewMode::Ringing { .. }));

    // Matching session_id auto-dismisses ringing
    let _ = app.update(Message::RingTimeout { session_id: 1 });
    assert!(matches!(app.mode, ViewMode::List { .. }));
}

#[test]
fn alarm_triggers_on_check_schedule() {
    let board = Arc::new(Board::simulated());
    let mut app = AlarmClock::with_board(board);
    let now = chrono::Local::now();
    use chrono::Timelike;
    app.config.alarms.push(AlarmItem::new(
        1,
        now.hour() as u8,
        now.minute() as u8,
        "测试闹钟",
        AlarmRepeat::everyday(),
    ));

    let _ = app.update(Message::CheckSchedule);
    assert!(matches!(app.mode, ViewMode::Ringing { .. }));
}

#[test]
fn snooze_triggers_on_check_schedule() {
    let board = Arc::new(Board::simulated());
    let mut app = AlarmClock::with_board(board);
    let past = chrono::Local::now() - chrono::Duration::seconds(5);
    app.snooze_until = Some(past);

    let _ = app.update(Message::CheckSchedule);
    assert!(matches!(app.mode, ViewMode::Ringing { .. }));
}

#[test]
fn time_synced_reschedules_and_clears_expired_snooze() {
    let board = Arc::new(Board::simulated());
    let mut app = AlarmClock::with_board(board);

    // Set an expired snooze
    let past = chrono::Local::now() - chrono::Duration::seconds(10);
    app.snooze_until = Some(past);

    let _ = app.update(Message::TimeSynced);
    assert_eq!(app.snooze_until, None);
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
    let _ = app.update(Message::Back);
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
    let _ = app.update(Message::Back);
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
    let _ = app.update(Message::Back);
    if let ViewMode::Edit { draft, sub_sheet, .. } = &app.mode {
        assert_eq!(sub_sheet, &None);
        assert_eq!(draft.snooze_minutes, 15);
    }
}

#[test]
fn alarm_card_split_interaction_option_a() {
    let board = Arc::new(Board::simulated());
    let app = AlarmClock::with_board(board);
    let view = app.view();
    let mut ui = iced_test::Simulator::new(view);

    // Verify alarm items are rendered in the split card list
    assert!(ui.find("07:00").is_ok(), "Find 07:00 alarm time");
    assert!(ui.find("08:30").is_ok(), "Find 08:30 alarm time");
    assert!(ui.find("起床  每天").is_ok(), "Find 起床  每天 subtitle");
    assert!(ui.find("工作  工作日").is_ok(), "Find 工作  工作日 subtitle");
}

#[test]
fn go_back_navigates_levels_correctly() {
    let board = Arc::new(Board::simulated());
    let mut app = AlarmClock::with_board(board);

    // 1. Initial Root List: go_back returns false (nothing to pop)
    assert_eq!(app.go_back(), false);
    assert_eq!(app.mode, ViewMode::List { is_editing_list: false });

    // 2. Enter list editing mode (delete buttons active)
    let _ = app.update(Message::ToggleListEdit);
    assert_eq!(app.mode, ViewMode::List { is_editing_list: true });
    // go_back exits list editing mode, stays in list
    assert_eq!(app.go_back(), true);
    assert_eq!(app.mode, ViewMode::List { is_editing_list: false });

    // 3. Open New Alarm (Level 2: Edit ActionSheet)
    let _ = app.update(Message::OpenNew);
    assert!(matches!(app.mode, ViewMode::Edit { sub_sheet: None, .. }));

    // 4. Open SubPage (Level 3: Repeat selection)
    let _ = app.update(Message::OpenRoute(AlarmRoute::Repeat));
    assert!(matches!(
        app.mode,
        ViewMode::Edit {
            sub_sheet: Some(AlarmRoute::Repeat),
            ..
        }
    ));

    // 5. Back from Level 3 returns to Level 2 (Edit ActionSheet)
    assert_eq!(app.go_back(), true);
    assert!(matches!(app.mode, ViewMode::Edit { sub_sheet: None, .. }));

    // 6. Back from Level 2 returns to Level 1 (List)
    assert_eq!(app.go_back(), true);
    assert_eq!(app.mode, ViewMode::List { is_editing_list: false });

    // 7. Back from Level 1 returns false (ready to exit to Launcher Grid)
    assert_eq!(app.go_back(), false);

    // 8. Test Message::Back in update
    let _ = app.update(Message::OpenNew);
    assert!(matches!(app.mode, ViewMode::Edit { sub_sheet: None, .. }));
    let _ = app.update(Message::Back);
    assert_eq!(app.mode, ViewMode::List { is_editing_list: false });
}

