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
    std::env::set_var("POMELO_APP_DATA", &test_dir);
    f();
    let _ = std::fs::remove_dir_all(&test_dir);
    std::env::remove_var("POMELO_APP_DATA");
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
        let _ = model::save_today_cache(yesterday, &[sample_event.clone()]);
        assert!(
            model::load_today_cache(today).is_none(),
            "Yesterday's cache must not be accepted as today's cache"
        );

        // Save cache with today's date
        let _ = model::save_today_cache(today, &[sample_event.clone()]);
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
fn daily_schedule_caches_today_and_tomorrow_and_seamless_midnight_transition() {
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

        let tomorrow_event = ScheduleEvent {
            id: "tomorrow-evt".to_string(),
            title: "明日发布".to_string(),
            start: Local.with_ymd_and_hms(tomorrow.year(), tomorrow.month(), tomorrow.day(), 14, 0, 0).unwrap(),
            end: Local.with_ymd_and_hms(tomorrow.year(), tomorrow.month(), tomorrow.day(), 15, 0, 0).unwrap(),
            is_all_day: false,
            color_index: 1,
        };

        // 1. Save schedule config and cache with both today and tomorrow
        let config = model::ScheduleConfig {
            subscription_url: Some("http://example.com/calendar.ics".to_string()),
            ..Default::default()
        };
        let _ = model::save_config(&config);
        let _ = model::save_schedule_cache(today, &[today_event.clone()], &[tomorrow_event.clone()]);

        // 2. Load cache on today: both today and tomorrow are present
        let (loaded_today, loaded_tomorrow) = model::load_schedule_cache(today).expect("cache for today");
        assert_eq!(loaded_today.len(), 1);
        assert_eq!(loaded_today[0].title, "今日例会");
        assert_eq!(loaded_tomorrow.len(), 1);
        assert_eq!(loaded_tomorrow[0].title, "明日发布");

        // 3. Initialize app on today
        let board = Arc::new(Board::simulated());
        let mut app = DailySchedule::with_board(board);
        assert_eq!(app.events().len(), 1);
        assert_eq!(app.events()[0].title, "今日例会");
        assert_eq!(app.tomorrow_events().len(), 1);
        assert_eq!(app.tomorrow_events()[0].title, "明日发布");

        // 4. Midnight tick arrives (crossing to tomorrow)
        let midnight_now = Local.with_ymd_and_hms(tomorrow.year(), tomorrow.month(), tomorrow.day(), 0, 0, 1).unwrap();
        let _ = app.update(Message::Tick(midnight_now));

        // Immediately promoted tomorrow_events to today's events with zero network wait!
        assert_eq!(app.events().len(), 1);
        assert_eq!(app.events()[0].title, "明日发布");
    });
}

#[test]
fn daily_schedule_ten_minute_silent_refresh_flow() {
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

        // 1. If last sync was 5 minutes ago (< 10 mins), no refresh is triggered
        app.set_last_sync_at(Some(Instant::now() - std::time::Duration::from_secs(300)));
        let _ = app.update(Message::Tick(Local::now()));
        assert_eq!(app.sync_status(), SyncStatus::Idle);
        assert!(!app.has_active_dialog());

        // 2. If last sync was 11 minutes ago (>= 10 mins), triggers silent fetch
        app.set_last_sync_at(Some(Instant::now() - std::time::Duration::from_secs(660)));
        let _ = app.update(Message::Tick(Local::now()));
        assert_eq!(app.sync_status(), SyncStatus::Fetching);
        assert!(app.is_silent_fetch());
        // Silent fetch MUST NOT show any blocking modal dialog
        assert!(!app.has_active_dialog());

        // 3. Fetch completes: updates events and timestamp silently
        board.tick();
        board.tick();
        let _ = app.update(Message::Tick(Local::now()));
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

        // Simulate returning to schedule after being away for 30 minutes
        app.set_last_sync_at(Some(Instant::now() - std::time::Duration::from_secs(1800)));
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
        let _ = model::save_schedule_cache(today, &[sample_event], &[]);

        // 2. Open app with board
        let board = Arc::new(Board::simulated());
        let mut app = DailySchedule::with_board(Arc::clone(&board));

        // 3. Cached schedule is immediately visible (0ms delay)
        assert_eq!(app.events().len(), 1);
        assert_eq!(app.events()[0].title, "已缓存会议");

        // 4. Background silent fetch is triggered immediately without blocking dialog
        assert_eq!(app.sync_status(), SyncStatus::Fetching);
        assert!(app.is_silent_fetch());
        assert!(!app.has_active_dialog(), "Silent fetch on startup must not show modal dialog");

        // 5. Complete HTTP fetch in background
        board.tick();
        board.tick();
        let _ = app.update(Message::Tick(Local::now()));

        assert_eq!(app.sync_status(), SyncStatus::Idle);
        assert!(!app.has_active_dialog());
        assert!(app.last_sync_at().is_some());
    });
}




