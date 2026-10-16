use std::sync::{Arc, Mutex};

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
