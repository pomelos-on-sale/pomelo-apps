use std::sync::{Arc, Mutex};
use std::time::Instant;

use chrono::{Datelike, Local, TimeZone};
use daily_schedule::model::{self, ScheduleEvent};
use daily_schedule::{DailySchedule, Message, SyncStatus};
use pomelo_hal::Board;

static TEST_MUTEX: Mutex<()> = Mutex::new(());

fn with_test_storage<F: FnOnce()>(f: F) {
    let _guard = TEST_MUTEX.lock().unwrap();
    let test_dir = std::env::temp_dir().join(format!("pomelo_test_sched_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&test_dir);
    std::env::set_var("POMELO_STORAGE_DIR", &test_dir);
    f();
    let _ = std::fs::remove_dir_all(&test_dir);
    std::env::remove_var("POMELO_STORAGE_DIR");
}

#[test]
fn daily_schedule_workflow_no_subscription_then_import_from_clipboard() {
    with_test_storage(|| {
        let board = Arc::new(Board::simulated());
        let mut app = DailySchedule::with_board(board);

        // 1. Initial state: no subscription
        assert_eq!(app.subscription_url(), None);
        assert!(app.events().is_empty());

        // 2. Clipboard contains invalid text
        let _ = app.update(Message::ClipboardImported(Some("just some text".to_string())));
        assert_eq!(app.subscription_url(), None, "Should reject non-url text");

        // 3. Clipboard contains webcal subscription URL
        let _ = app.update(Message::ClipboardImported(Some(
            "webcal://p20-caldav.icloud.com/published/2/test_calendar".to_string(),
        )));

        assert_eq!(
            app.subscription_url(),
            Some("https://p20-caldav.icloud.com/published/2/test_calendar")
        );
    });
}

#[test]
fn daily_schedule_cache_validation_today_vs_stale() {
    with_test_storage(|| {
        let today = Local::now().date_naive();
        let yesterday = today - chrono::Duration::days(1);

        let sample_event = ScheduleEvent {
            id: "sample-1".to_string(),
            title: "测试日程".to_string(),
            start: Local.with_ymd_and_hms(today.year(), today.month(), today.day(), 10, 0, 0).unwrap(),
            end: Local.with_ymd_and_hms(today.year(), today.month(), today.day(), 11, 0, 0).unwrap(),
            is_all_day: false,
            color_index: 0,
        };

        // Save cache with yesterday's date
        let _ = model::save_today_cache(yesterday, std::slice::from_ref(&sample_event));
        assert!(
            model::load_today_cache(today).is_none(),
            "Yesterday's cache must not be accepted as today's cache"
        );

        // Save cache with today's date
        let _ = model::save_today_cache(today, std::slice::from_ref(&sample_event));
        let loaded = model::load_today_cache(today);
        assert!(loaded.is_some(), "Today's cache must be accepted");
        assert_eq!(loaded.unwrap().len(), 1);
    });
}

#[test]
fn daily_schedule_sync_completion_flow() {
    with_test_storage(|| {
        let board = Arc::new(Board::simulated());
        let mut app = DailySchedule::with_board(Arc::clone(&board));

        let today = Local::now().date_naive();

        // Import URL (using http scheme in simulator without NTP clock sync)
        let _ = app.update(Message::ClipboardImported(Some(
            "http://example.com/calendar.ics".to_string(),
        )));
        assert_eq!(app.sync_status(), SyncStatus::Fetching);

        // Advance simulated board ticks
        board.tick();
        board.tick();

        // App ticks to process HTTP completion
        let _ = app.update(Message::Tick(Local::now()));

        assert_eq!(app.sync_status(), SyncStatus::Idle);

        // Today's cache file is created
        assert!(model::load_today_cache(today).is_some());
    });
}

#[test]
fn daily_schedule_dialog_lifecycle_and_timeline_stack() {
    with_test_storage(|| {
        let board = Arc::new(Board::simulated());
        let mut app = DailySchedule::with_board(Arc::clone(&board));

        // 1. Initial boot without subscription: NoSubscription dialog is active over timeline
        assert!(app.has_active_dialog());
        let _ = app.view();

        // 2. Dismiss dialog: timeline is still shown, dialog is closed
        let _ = app.update(Message::DismissDialog);
        assert!(!app.has_active_dialog());
        let _ = app.view();

        // 3. Import URL: starts fetching, Syncing dialog is active
        let _ = app.update(Message::ClipboardImported(Some(
            "http://example.com/calendar.ics".to_string(),
        )));
        assert_eq!(app.sync_status(), SyncStatus::Fetching);
        assert!(app.has_active_dialog());
        let _ = app.view();

        // 4. Dismiss while fetching (background mode): dialog is closed while fetch continues
        let _ = app.update(Message::DismissDialog);
        assert!(!app.has_active_dialog());
        assert_eq!(app.sync_status(), SyncStatus::Fetching);

        // 5. Complete fetch
        board.tick();
        board.tick();
        let _ = app.update(Message::Tick(Local::now()));
        assert_eq!(app.sync_status(), SyncStatus::Idle);
        assert!(!app.has_active_dialog());
        let _ = app.view();
    });
}

#[test]
fn daily_schedule_smooth_scroll_return_after_10_seconds() {
    use iced::widget::scrollable::AbsoluteOffset;
    use std::time::{Duration, Instant};

    with_test_storage(|| {
        let board = Arc::new(Board::simulated());
        let mut app = DailySchedule::with_board(board);

        let today = Local::now().date_naive();
        let now = Local
            .with_ymd_and_hms(today.year(), today.month(), today.day(), 10, 0, 0)
            .unwrap();
        // 1. Initial tick focuses on timeline
        let _ = app.update(Message::Tick(now));
        assert!(!app.is_animating_scroll());

        // 2. User manually scrolls away from current time (e.g. to offset Y = 800)
        let _ = app.update(Message::TimelineScrolled(AbsoluteOffset {
            x: 0.0,
            y: 800.0,
        }));
        assert_eq!(app.current_scroll_y(), 800.0);
        assert!(!app.is_animating_scroll());

        // 3. User scrolled 5 seconds ago (< 10s): still in user browsing period, no auto-reset
        app.set_user_scrolled_at(Some(Instant::now() - Duration::from_secs(5)));
        let _ = app.update(Message::Tick(now));
        assert!(!app.is_animating_scroll());
        assert_eq!(app.current_scroll_y(), 800.0);

        // 4. User scrolled 11 seconds ago (>= 10s): 10s inactivity expires, starts smooth animation!
        app.set_user_scrolled_at(Some(Instant::now() - Duration::from_secs(11)));
        let _ = app.update(Message::Tick(now));
        assert!(app.is_animating_scroll());

        // 5. Subscription includes window::frames when animating
        let _ = app.subscription();

        // 6. Advance animation frames
        let anim_start = Instant::now();
        // Midpoint frame: value moves between 800 and target_y
        let _ = app.update(Message::ScrollFrame(anim_start + Duration::from_millis(250)));
        assert!(app.is_animating_scroll());
        assert!(app.current_scroll_y() < 800.0);

        // Final frame at 500ms+: animation finishes and reaches target
        let _ = app.update(Message::ScrollFrame(anim_start + Duration::from_millis(600)));
        assert!(!app.is_animating_scroll());
    });
}

#[test]
fn daily_schedule_caches_today_and_midnight_transition_triggers_refresh() {
    with_test_storage(|| {
        let today = Local::now().date_naive();
        let tomorrow = today + chrono::Duration::days(1);

        let today_event = ScheduleEvent {
            id: "today-evt".to_string(),
            title: "今日例会".to_string(),
            start: Local.with_ymd_and_hms(today.year(), today.month(), today.day(), 10, 0, 0).unwrap(),
            end: Local.with_ymd_and_hms(today.year(), today.month(), today.day(), 11, 0, 0).unwrap(),
            is_all_day: false,
            color_index: 0,
        };

        // 1. Save schedule config and cache for today
        let config = model::ScheduleConfig {
            subscription_url: Some("http://example.com/calendar.ics".to_string()),
            ..Default::default()
        };
        let _ = model::save_config(&config);
        let _ = model::save_today_cache(today, std::slice::from_ref(&today_event));

        // 2. Load cache on today: today is present
        let loaded_today = model::load_today_cache(today).expect("cache for today");
        assert_eq!(loaded_today.len(), 1);
        assert_eq!(loaded_today[0].title, "今日例会");

        // 3. Initialize app on today
        let board = Arc::new(Board::simulated());
        let mut app = DailySchedule::with_board(board);
        assert_eq!(app.events().len(), 1);
        assert_eq!(app.events()[0].title, "今日例会");

        // 4. Midnight tick arrives (crossing to tomorrow 00:00:00)
        let midnight_now = Local.with_ymd_and_hms(tomorrow.year(), tomorrow.month(), tomorrow.day(), 0, 0, 0).unwrap();
        let _ = app.update(Message::Tick(midnight_now));

        // Date transitions to new day, and minute 0 triggers on-the-hour silent fetch for new day
        assert_eq!(app.sync_status(), SyncStatus::Fetching);
        assert!(app.is_silent_fetch());
    });
}

#[test]
fn daily_schedule_hourly_on_the_hour_silent_refresh_flow() {
    with_test_storage(|| {
        let board = Arc::new(Board::simulated());
        let mut app = DailySchedule::with_board(Arc::clone(&board));
        let today = Local::now().date_naive();

        // Setup subscription
        let _ = app.update(Message::ClipboardImported(Some(
            "http://example.com/calendar.ics".to_string(),
        )));
        board.tick();
        board.tick();
        let _ = app.update(Message::Tick(Local::now()));
        assert_eq!(app.sync_status(), SyncStatus::Idle);

        // 1. Tick arrives at minute 35 (not on the hour) -> no refresh triggered
        let t_35 = Local.with_ymd_and_hms(today.year(), today.month(), today.day(), 10, 35, 0).unwrap();
        let _ = app.update(Message::Tick(t_35));
        assert_eq!(app.sync_status(), SyncStatus::Idle);
        assert!(!app.has_active_dialog());

        // 2. Tick arrives at minute 00 (top of the hour 11:00) -> triggers silent fetch
        let t_00 = Local.with_ymd_and_hms(today.year(), today.month(), today.day(), 11, 0, 0).unwrap();
        let _ = app.update(Message::Tick(t_00));
        assert_eq!(app.sync_status(), SyncStatus::Fetching);
        assert!(app.is_silent_fetch());
        assert!(!app.has_active_dialog());

        // 3. Subsequent tick within the same minute (11:00:15) -> does not trigger duplicate fetch
        let t_00_15 = Local.with_ymd_and_hms(today.year(), today.month(), today.day(), 11, 0, 15).unwrap();
        let _ = app.update(Message::Tick(t_00_15));
        assert_eq!(app.sync_status(), SyncStatus::Fetching);

        // 4. Fetch completes: updates events and returns to Idle
        board.tick();
        board.tick();
        let _ = app.update(Message::Tick(t_00_15));
        assert_eq!(app.sync_status(), SyncStatus::Idle);
        assert!(!app.has_active_dialog());
        assert!(app.last_sync_at().unwrap().elapsed() < std::time::Duration::from_secs(3));
    });
}

#[test]
fn daily_schedule_focus_reentry_staleness_check() {
    with_test_storage(|| {
        let board = Arc::new(Board::simulated());
        let mut app = DailySchedule::with_board(Arc::clone(&board));

        // Setup subscription
        let _ = app.update(Message::ClipboardImported(Some(
            "http://example.com/calendar.ics".to_string(),
        )));
        board.tick();
        board.tick();
        let _ = app.update(Message::Tick(Local::now()));
        assert_eq!(app.sync_status(), SyncStatus::Idle);

        // Simulate returning to schedule after being away for 30 minutes (< 1 hour -> no fetch)
        app.set_last_sync_at(Some(Instant::now() - std::time::Duration::from_secs(1800)));
        let _ = app.focus();
        assert_eq!(app.sync_status(), SyncStatus::Idle);

        // Simulate returning to schedule after being away for 70 minutes (>= 1 hour -> triggers silent fetch)
        app.set_last_sync_at(Some(Instant::now() - std::time::Duration::from_secs(4200)));
        let _ = app.focus();

        // Automatically initiated silent refresh upon re-entering foreground
        assert_eq!(app.sync_status(), SyncStatus::Fetching);
        assert!(app.is_silent_fetch());
        assert!(!app.has_active_dialog());
    });
}

#[test]
fn daily_schedule_startup_with_cache_immediately_triggers_silent_fetch() {
    with_test_storage(|| {
        let today = Local::now().date_naive();
        let sample_event = ScheduleEvent {
            id: "sample-startup".to_string(),
            title: "已缓存会议".to_string(),
            start: Local.with_ymd_and_hms(today.year(), today.month(), today.day(), 9, 30, 0).unwrap(),
            end: Local.with_ymd_and_hms(today.year(), today.month(), today.day(), 10, 30, 0).unwrap(),
            is_all_day: false,
            color_index: 0,
        };

        // 1. Prepare existing config and cache
        let config = model::ScheduleConfig {
            subscription_url: Some("http://example.com/calendar.ics".to_string()),
            ..Default::default()
        };
        let _ = model::save_config(&config);
        let _ = model::save_today_cache(today, &[sample_event]);

        // 2. Instantiate app with board
        let board = Arc::new(Board::simulated());
        let mut app = DailySchedule::with_board(Arc::clone(&board));

        // 3. Cached schedule is immediately visible (0ms delay), no network fetch on instantiation
        assert_eq!(app.events().len(), 1);
        assert_eq!(app.events()[0].title, "已缓存会议");
        assert_eq!(app.sync_status(), SyncStatus::Idle);

        // 4. Opening/focusing the app initiates background silent fetch
        let _ = app.focus();
        assert_eq!(app.sync_status(), SyncStatus::Fetching);
        assert!(app.is_silent_fetch());
        assert!(!app.has_active_dialog(), "Silent fetch on focus must not show modal dialog");

        // 5. Complete HTTP fetch in background
        board.tick();
        board.tick();
        let _ = app.update(Message::Tick(Local::now()));

        assert_eq!(app.sync_status(), SyncStatus::Idle);
        assert!(!app.has_active_dialog());
        assert!(app.last_sync_at().is_some());
    });
}

#[test]
fn daily_schedule_sound_reminder_flow() {
    with_test_storage(|| {
        use daily_schedule::sound::ScheduleSounds;
        use chrono::TimeZone;

        // 1. Verify sound initialization finds valid physical sound files
        let sounds = ScheduleSounds::init_and_load();
        assert!(sounds.start_path.exists(), "Start sound file must exist on disk");
        assert!(sounds.end_path.exists(), "End sound file must exist on disk");

        // 2. Prepare test schedule event starting at 10:00 and ending at 11:00
        let today = Local::now().date_naive();
        let start_time = Local.with_ymd_and_hms(today.year(), today.month(), today.day(), 10, 0, 0).unwrap();
        let end_time = Local.with_ymd_and_hms(today.year(), today.month(), today.day(), 11, 0, 0).unwrap();
        let event = ScheduleEvent {
            id: "meeting-alert".to_string(),
            title: "重要会议".to_string(),
            start: start_time,
            end: end_time,
            is_all_day: false,
            color_index: 1,
        };

        // Config with reminder_sound enabled
        let config = model::ScheduleConfig {
            subscription_url: Some("http://example.com/calendar.ics".to_string()),
            reminder_sound: true,
            ..Default::default()
        };
        let _ = model::save_config(&config);
        let _ = model::save_today_cache(today, &[event]);

        let board = Arc::new(Board::simulated());
        let mut app = DailySchedule::with_board(Arc::clone(&board));
        assert_eq!(app.events().len(), 1);

        // Advance to 09:59 (initial tick)
        let before_start = Local.with_ymd_and_hms(today.year(), today.month(), today.day(), 9, 59, 0).unwrap();
        let _ = app.update(Message::Tick(before_start));
        assert!(!board.audio().is_playing(), "No chime before event start");

        // Advance to 10:00 (event start time), display should be powered on
        let _ = board.set_display_power(false);
        assert!(!board.is_display_on());
        let _ = app.update(Message::Tick(start_time));
        assert!(board.is_display_on(), "Display should be turned on when event begins");
        assert!(board.audio().is_playing(), "Start chime plays when event begins");
        board.audio().stop();

        // Repeated tick in the same minute should not repeat chime
        let same_minute = Local.with_ymd_and_hms(today.year(), today.month(), today.day(), 10, 0, 30).unwrap();
        let _ = app.update(Message::Tick(same_minute));
        assert!(!board.audio().is_playing(), "Chime does not repeat in same minute");

        // Advance to 10:59
        let before_end = Local.with_ymd_and_hms(today.year(), today.month(), today.day(), 10, 59, 0).unwrap();
        let _ = app.update(Message::Tick(before_end));
        assert!(!board.audio().is_playing());

        // Advance to 11:00 (event end time)
        let _ = app.update(Message::Tick(end_time));
        assert!(board.audio().is_playing(), "End chime plays when event ends");
    });
}

#[test]
fn daily_schedule_first_frame_focus_without_subscription() {
    with_test_storage(|| {
        let board = Arc::new(Board::simulated());
        let mut app = DailySchedule::with_board(board);

        let today = Local::now().date_naive();
        let now = Local.with_ymd_and_hms(today.year(), today.month(), today.day(), 14, 30, 0).unwrap();
        let _ = app.update(Message::Tick(now));

        // 1. Initially without subscription, current_scroll_y matches target_y
        let expected_y = daily_schedule::timeline::Timeline::target_y(now);
        assert!((app.current_scroll_y() - expected_y).abs() <= 1.0);

        // 2. Initial ScrollFrame immediately executes initial focus on frame 1
        let _ = app.update(Message::ScrollFrame(Instant::now()));
        assert!((app.current_scroll_y() - expected_y).abs() <= 1.0);
        assert!(!app.is_animating_scroll());

        // 3. TimelineScrolled(0.0) arriving during initial mount does NOT lock out for 10 seconds
        let _ = app.update(Message::TimelineScrolled(iced::widget::scrollable::AbsoluteOffset {
            x: 0.0,
            y: expected_y,
        }));
        assert!(!app.is_animating_scroll());

        // 4. Dismissing dialog immediately refocuses to target_y without 10-second wait
        let _ = app.update(Message::DismissDialog);
        assert!((app.current_scroll_y() - expected_y).abs() <= 1.0);
        assert!(!app.is_animating_scroll());
    });
}

#[test]
fn daily_schedule_startup_check_and_import_dialog_flow() {
    with_test_storage(|| {
        let board = Arc::new(Board::simulated());
        let mut app = DailySchedule::with_board(Arc::clone(&board));

        // 1. App starts without subscription -> Dialog is displayed
        assert!(app.has_active_dialog());
        let _ = app.view();

        // 2. Click Cancel ("取消") -> Dialog is closed, timeline is empty
        let _ = app.update(Message::DismissDialog);
        assert!(!app.has_active_dialog());
        assert!(app.events().is_empty());
        let _ = app.view();

        // 3. Each time the app starts / focuses without subscription -> Dialog pops up again
        let _ = app.focus();
        assert!(app.has_active_dialog(), "Dialog must reappear on focus when no subscription");
        let _ = app.view();

        // 4. Try import from clipboard with invalid text -> Dialog stays open and shows error
        let _ = app.update(Message::ClipboardImported(Some("invalid-url-text".to_string())));
        assert!(app.has_active_dialog(), "Dialog remains open after invalid clipboard");
        assert_eq!(app.subscription_url(), None);
        let _ = app.view();

        // 5. Try import from clipboard with valid URL -> Starts fetching, Dialog stays open showing progress
        let _ = app.update(Message::ClipboardImported(Some(
            "http://example.com/calendar.ics".to_string(),
        )));
        assert_eq!(app.sync_status(), SyncStatus::Fetching);
        assert!(app.has_active_dialog(), "Dialog shows syncing status");
        let _ = app.view();

        // 6. Complete fetch -> Dialog automatically closes, cache saved
        board.tick();
        board.tick();
        let _ = app.update(Message::Tick(Local::now()));
        assert_eq!(app.sync_status(), SyncStatus::Idle);
        assert!(!app.has_active_dialog(), "Dialog closes after successful sync");
        assert!(model::load_today_cache(Local::now().date_naive()).is_some());
        let _ = app.view();

        // 7. Focus when subscription is already configured -> No dialog shown
        let _ = app.focus();
        assert!(!app.has_active_dialog(), "No dialog shown on focus when subscription is configured");
    });
}

#[test]
fn daily_schedule_event_start_alarm_trigger_and_dismiss_flow() {
    with_test_storage(|| {
        let board = Arc::new(Board::simulated());
        let today = Local::now().date_naive();

        // 1. Configure event_start_alarm = true
        let mut cfg = model::load_config();
        cfg.subscription_url = Some("http://example.com/cal.ics".to_string());
        cfg.event_start_alarm = true;
        cfg.reminder_sound = false;
        let _ = model::save_config(&cfg);

        let event = ScheduleEvent {
            id: "alarm-evt-1".to_string(),
            title: "核心战略会议".to_string(),
            start: Local.with_ymd_and_hms(today.year(), today.month(), today.day(), 14, 0, 0).unwrap(),
            end: Local.with_ymd_and_hms(today.year(), today.month(), today.day(), 15, 0, 0).unwrap(),
            is_all_day: false,
            color_index: 0,
        };
        let _ = model::save_today_cache(today, std::slice::from_ref(&event));

        let mut app = DailySchedule::with_board(Arc::clone(&board));
        assert!(!app.is_ringing());

        // 2. Before start time (13:59), not ringing
        let t1 = Local.with_ymd_and_hms(today.year(), today.month(), today.day(), 13, 59, 0).unwrap();
        let _ = app.update(Message::Tick(t1));
        assert!(!app.is_ringing());

        // 3. At start time (14:00), event start alarm triggers
        let t2 = Local.with_ymd_and_hms(today.year(), today.month(), today.day(), 14, 0, 0).unwrap();
        let _ = app.update(Message::Tick(t2));
        assert!(app.is_ringing(), "Should be ringing at event start time");
        assert_eq!(app.ringing_state.as_ref().unwrap().title, "核心战略会议");
        assert_eq!(app.ringing_state.as_ref().unwrap().time_range_str, "14:00 - 15:00");

        // 4. View renders ringing screen without crashing
        let _ = app.view();

        // 5. Dismiss ringing
        let _ = app.update(Message::DismissRinging);
        assert!(!app.is_ringing(), "Should stop ringing after DismissRinging");
        assert!(app.ringing_state.is_none());

        // 6. View returns to normal timeline
        let _ = app.view();
    });
}

#[test]
fn daily_schedule_event_start_alarm_timeout_flow() {
    with_test_storage(|| {
        let board = Arc::new(Board::simulated());
        let today = Local::now().date_naive();

        let mut cfg = model::load_config();
        cfg.subscription_url = Some("http://example.com/cal.ics".to_string());
        cfg.event_start_alarm = true;
        let _ = model::save_config(&cfg);

        let event = ScheduleEvent {
            id: "alarm-evt-2".to_string(),
            title: "超时测试会议".to_string(),
            start: Local.with_ymd_and_hms(today.year(), today.month(), today.day(), 16, 0, 0).unwrap(),
            end: Local.with_ymd_and_hms(today.year(), today.month(), today.day(), 17, 0, 0).unwrap(),
            is_all_day: false,
            color_index: 0,
        };
        let _ = model::save_today_cache(today, std::slice::from_ref(&event));

        let mut app = DailySchedule::with_board(Arc::clone(&board));

        let t = Local.with_ymd_and_hms(today.year(), today.month(), today.day(), 16, 0, 0).unwrap();
        let _ = app.update(Message::Tick(t));
        assert!(app.is_ringing());

        let session_id = app.ringing_state.as_ref().unwrap().session_id;

        // Mismatched session id does not stop ringing
        let _ = app.update(Message::RingTimeout { session_id: session_id + 999 });
        assert!(app.is_ringing());

        // Matching session id stops ringing (60s timeout)
        let _ = app.update(Message::RingTimeout { session_id });
        assert!(!app.is_ringing());
    });
}

#[test]
fn daily_schedule_event_start_alarm_go_back_flow() {
    with_test_storage(|| {
        let board = Arc::new(Board::simulated());
        let today = Local::now().date_naive();

        let mut cfg = model::load_config();
        cfg.subscription_url = Some("http://example.com/cal.ics".to_string());
        cfg.event_start_alarm = true;
        let _ = model::save_config(&cfg);

        let event = ScheduleEvent {
            id: "alarm-evt-3".to_string(),
            title: "返回按键测试".to_string(),
            start: Local.with_ymd_and_hms(today.year(), today.month(), today.day(), 9, 30, 0).unwrap(),
            end: Local.with_ymd_and_hms(today.year(), today.month(), today.day(), 10, 0, 0).unwrap(),
            is_all_day: false,
            color_index: 0,
        };
        let _ = model::save_today_cache(today, std::slice::from_ref(&event));

        let mut app = DailySchedule::with_board(Arc::clone(&board));

        // When not ringing, go_back returns false
        assert!(!app.go_back());

        // Trigger alarm
        let t = Local.with_ymd_and_hms(today.year(), today.month(), today.day(), 9, 30, 0).unwrap();
        let _ = app.update(Message::Tick(t));
        assert!(app.is_ringing());

        // When ringing, go_back consumes and stops ringing
        assert!(app.go_back());
        assert!(!app.is_ringing());

        // Subsequent go_back is unconsumed
        assert!(!app.go_back());
    });
}

#[test]
fn daily_schedule_reminder_sound_does_not_trigger_fullscreen_alarm() {
    with_test_storage(|| {
        let board = Arc::new(Board::simulated());
        let today = Local::now().date_naive();

        let mut cfg = model::load_config();
        cfg.subscription_url = Some("http://example.com/cal.ics".to_string());
        cfg.event_start_alarm = false;
        cfg.reminder_sound = true;
        let _ = model::save_config(&cfg);

        let event = ScheduleEvent {
            id: "sound-only-evt".to_string(),
            title: "仅提示音日程".to_string(),
            start: Local.with_ymd_and_hms(today.year(), today.month(), today.day(), 11, 0, 0).unwrap(),
            end: Local.with_ymd_and_hms(today.year(), today.month(), today.day(), 12, 0, 0).unwrap(),
            is_all_day: false,
            color_index: 0,
        };
        let _ = model::save_today_cache(today, std::slice::from_ref(&event));

        let mut app = DailySchedule::with_board(Arc::clone(&board));

        let t = Local.with_ymd_and_hms(today.year(), today.month(), today.day(), 11, 0, 0).unwrap();
        let _ = app.update(Message::Tick(t));

        // Must not be ringing full screen
        assert!(!app.is_ringing());
    });
}

#[test]
fn daily_schedule_side_by_side_view_renders_cleanly() {
    with_test_storage(|| {
        let board = Arc::new(Board::simulated());
        let app = DailySchedule::with_board(Arc::clone(&board));

        // Side-by-side view (digital clock on the left, timeline on the right) builds cleanly
        let _ = app.view();

        // Directly verify digital clock renders without panic
        let now = Local::now();
        let _ = daily_schedule::time_display::view::<Message>(now, pomelo_widgets::Language::Chinese);
    });
}

#[test]
fn daily_schedule_event_alarm_preserves_scroll_offset_and_widget_tree_structure() {
    use iced::advanced::widget::Tree;
    use iced::widget::scrollable::AbsoluteOffset;

    with_test_storage(|| {
        let board = Arc::new(Board::simulated());
        let today = Local::now().date_naive();

        let mut cfg = model::load_config();
        cfg.subscription_url = Some("http://example.com/cal.ics".to_string());
        cfg.event_start_alarm = true;
        let _ = model::save_config(&cfg);

        let event = ScheduleEvent {
            id: "alarm-scroll-preserve".to_string(),
            title: "滚动位置保持测试".to_string(),
            start: Local.with_ymd_and_hms(today.year(), today.month(), today.day(), 15, 30, 0).unwrap(),
            end: Local.with_ymd_and_hms(today.year(), today.month(), today.day(), 16, 30, 0).unwrap(),
            is_all_day: false,
            color_index: 0,
        };
        let _ = model::save_today_cache(today, std::slice::from_ref(&event));

        let mut app = DailySchedule::with_board(Arc::clone(&board));

        // 1. Initial tick to establish initial focus
        let t_init = Local.with_ymd_and_hms(today.year(), today.month(), today.day(), 15, 0, 0).unwrap();
        let _ = app.update(Message::Tick(t_init));

        // 2. Initial view tag before ringing
        let initial_root_tag = {
            let view = app.view();
            Tree::new(&view).tag
        };

        // 3. User has scrolled timeline to y = 520.0 (and is within browsing period)
        let user_scroll_y = 520.0;
        let _ = app.update(Message::TimelineScrolled(AbsoluteOffset {
            x: 0.0,
            y: user_scroll_y,
        }));
        app.set_user_scrolled_at(Some(std::time::Instant::now()));
        assert_eq!(app.current_scroll_y(), user_scroll_y);

        // 4. Alarm triggers at 15:30
        let t_alarm = Local.with_ymd_and_hms(today.year(), today.month(), today.day(), 15, 30, 0).unwrap();
        let _ = app.update(Message::Tick(t_alarm));
        assert!(app.is_ringing());

        // 5. While ringing, widget tree retains same root container tag (Stack preserved)
        // and internal current_scroll_y remains intact
        let ringing_root_tag = {
            let view = app.view();
            Tree::new(&view).tag
        };
        assert_eq!(
            ringing_root_tag, initial_root_tag,
            "Root widget tag must remain identical during ringing to preserve implicit widget state"
        );
        assert_eq!(
            app.current_scroll_y(),
            user_scroll_y,
            "Scroll offset must not be reset to 0 while ringing screen is active"
        );

        // 6. Dismiss ringing
        let _ = app.update(Message::DismissRinging);
        assert!(!app.is_ringing());

        // 7. After dismiss, widget tree tag is still identical and scroll offset is preserved
        let after_dismiss_tag = {
            let view = app.view();
            Tree::new(&view).tag
        };
        assert_eq!(
            after_dismiss_tag, initial_root_tag,
            "Root widget tag must remain identical after dismiss"
        );
        assert_eq!(
            app.current_scroll_y(),
            user_scroll_y,
            "Scroll offset must remain preserved after dismissing ringing"
        );
    });
}

#[test]
fn daily_schedule_failed_sync_preserves_existing_events_and_cache() {
    struct MockHttp {
        status_code: u16,
        body: Vec<u8>,
        done: bool,
    }
    impl pomelo_hal::traits::HttpBackend for MockHttp {
        fn start(&mut self, _url: &str) -> Result<(), pomelo_hal::HalError> {
            self.done = true;
            Ok(())
        }
        fn state(&self) -> pomelo_hal::HttpState {
            if self.done {
                pomelo_hal::HttpState::Done
            } else {
                pomelo_hal::HttpState::Idle
            }
        }
        fn status_code(&self) -> Option<u16> {
            Some(self.status_code)
        }
        fn take_result(&mut self) -> Result<Vec<u8>, pomelo_hal::HalError> {
            self.done = false;
            Ok(self.body.clone())
        }
        fn cancel(&mut self) -> Result<(), pomelo_hal::HalError> {
            self.done = false;
            Ok(())
        }
    }

    with_test_storage(|| {
        let today = Local::now().date_naive();
        let existing_event = ScheduleEvent {
            id: "evt-keep".to_string(),
            title: "重要日程不可被冲掉".to_string(),
            start: Local.with_ymd_and_hms(today.year(), today.month(), today.day(), 14, 0, 0).unwrap(),
            end: Local.with_ymd_and_hms(today.year(), today.month(), today.day(), 15, 0, 0).unwrap(),
            is_all_day: false,
            color_index: 0,
        };

        // 1. Pre-save valid today's event in cache and config
        let config = model::ScheduleConfig {
            subscription_url: Some("http://example.com/calendar.ics".to_string()),
            ..Default::default()
        };
        let _ = model::save_config(&config);
        let _ = model::save_today_cache(today, std::slice::from_ref(&existing_event));

        // 2. Mock HTTP backend returning 429 Too Many Requests with HTML body
        let mock_http = MockHttp {
            status_code: 429,
            body: b"<html><body>429 Too Many Requests</body></html>".to_vec(),
            done: false,
        };

        let board = Arc::new(Board::from_backends(
            Box::new(pomelo_hal::sim::SimPower::new()),
            Box::new(pomelo_hal::sim::SimWifi::new()),
            Box::new(pomelo_hal::sim::SimAudio::new()),
            Box::new(pomelo_hal::sim::SimMic::new()),
            Box::new(pomelo_hal::sim::SimImu::new()),
            Box::new(pomelo_hal::sim::SimInput::new()),
            Box::new(pomelo_hal::sim::SimTime::new()),
            Box::new(pomelo_hal::sim::SimStorage::new()),
            Box::new(mock_http),
        ));

        let mut app = DailySchedule::with_board(board);
        assert_eq!(app.events().len(), 1);
        assert_eq!(app.events()[0].title, "重要日程不可被冲掉");

        // 3. Trigger silent refresh
        app.trigger_silent_fetch();
        assert_eq!(app.sync_status(), SyncStatus::Fetching);

        // 4. Poll async tasks to process HTTP Done (429)
        let _ = app.update(Message::AsyncPoll);
        assert_eq!(app.sync_status(), SyncStatus::Idle);

        // 5. Existing event MUST remain intact and cache MUST not be cleared
        assert_eq!(
            app.events().len(),
            1,
            "Existing events must not be wiped out by HTTP 429 failure"
        );
        assert_eq!(app.events()[0].title, "重要日程不可被冲掉");

        let cached = model::load_today_cache(today).expect("cache should still exist");
        assert_eq!(
            cached.len(),
            1,
            "Cached events must not be overwritten by HTTP 429 failure"
        );
        assert_eq!(cached[0].title, "重要日程不可被冲掉");
    });
}





