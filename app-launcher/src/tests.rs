// The tests drive `update` by hand, and a test has no loop to return the `Task` it produces to.
// What that task does — the settings app scrolling its own body — belongs to the loop.
#![allow(unused_must_use)]
use std::time::Duration;
use super::*;

#[test]
fn default_preferences_are_chinese_dark_standard() {
    let board = Arc::new(Board::simulated());
    let launcher = Launcher::new(board);

    assert_eq!(launcher.preferences().language, Language::Chinese);
    assert_eq!(launcher.preferences().theme, ThemeMode::Dark);
    assert_eq!(launcher.preferences().font_tier, FontSizeTier::Standard);
    assert_eq!(launcher.preferences().screen_off_timeout, ScreenOffTimeout::Never);
    assert_eq!(launcher.board.screen_off_timeout(), None);
}

#[test]
fn catalogue_entries_are_localized_in_both_languages() {
    for entry in CATALOGUE {
        assert_ne!(entry.localized_name(Language::Chinese), entry.localized_name(Language::English));
        assert!(!entry.localized_name(Language::Chinese).is_empty());
        assert!(!entry.localized_name(Language::English).is_empty());
    }

    // Page 1 Row 1: Settings, Alarm
    assert_eq!(CATALOGUE[0].name, "Settings");
    assert_eq!(CATALOGUE[1].name, "Alarm");
    // Page 1 Row 2: Daily Schedule, Calculator
    assert_eq!(CATALOGUE[2].name, "Daily Schedule");
    assert_eq!(CATALOGUE[3].name, "Calculator");
    // Page 2 Row 1: Clipboard, Music
    assert_eq!(CATALOGUE[4].name, "Clipboard");
    assert_eq!(CATALOGUE[5].name, "Music");
    // Page 2 Row 2: Terminal, Demo Counter
    assert_eq!(CATALOGUE[6].name, "Terminal");
    assert_eq!(CATALOGUE[7].name, "Demo Counter");
    // Page 3 Row 1: Demo Hello
    assert_eq!(CATALOGUE[8].name, "Demo Hello");

    assert_eq!(entry_of(AppId::Alarm).localized_name(Language::Chinese), "闹钟");
    assert_eq!(entry_of(AppId::Alarm).localized_name(Language::English), "Alarm");
    assert_eq!(entry_of(AppId::Terminal).localized_name(Language::Chinese), "终端");
    assert_eq!(entry_of(AppId::Calculator).localized_name(Language::Chinese), "计算器");
    assert_eq!(entry_of(AppId::DemoCounter).localized_name(Language::Chinese), "演示 计数器");
    assert_eq!(entry_of(AppId::DemoCounter).localized_name(Language::English), "Demo Counter");
    assert_eq!(entry_of(AppId::DemoHello).localized_name(Language::Chinese), "演示 你好");
    assert_eq!(entry_of(AppId::DemoHello).localized_name(Language::English), "Demo Hello");
    assert_eq!(entry_of(AppId::Settings).localized_name(Language::Chinese), "设置");
    assert_eq!(entry_of(AppId::Music).localized_name(Language::Chinese), "音乐");
    assert_eq!(entry_of(AppId::DailySchedule).localized_name(Language::Chinese), "日程");
    assert_eq!(entry_of(AppId::WirelessClipboard).localized_name(Language::Chinese), "无线剪切板");
}

#[test]
fn tile_label_truncation_and_single_line() {
    // "demo-counter" fits completely in the wider tile line width
    assert_eq!(
        style::truncate_label("demo-counter", style::LABEL_MAX_WIDTH, style::LABEL),
        "demo-counter"
    );
    assert_eq!(
        style::truncate_label("Terminal", style::LABEL_MAX_WIDTH, style::LABEL),
        "Terminal"
    );
    assert_eq!(
        style::truncate_label("终端", style::LABEL_MAX_WIDTH, style::LABEL),
        "终端"
    );

    // Strips any potential newline to guarantee single line
    assert_eq!(
        style::truncate_label("demo-counter\nsecond-line", style::LABEL_MAX_WIDTH, style::LABEL),
        "demo-counter"
    );

    // Very long name truncates and appends "..."
    let long_name = "SuperUltraLongApplicationNameThatExceedsWidth";
    let truncated = style::truncate_label(long_name, style::LABEL_MAX_WIDTH, style::LABEL);
    assert!(truncated.ends_with("..."));
    assert!(truncated.len() < long_name.len());
    assert!(!truncated.contains('\n'));
    assert!(style::text_width(&truncated, style::LABEL) <= style::LABEL_MAX_WIDTH);

    // Very long Chinese name also truncates and appends "..."
    let long_chinese = "这是一个超长应用程序名称用于测试截断效果";
    let truncated_zh = style::truncate_label(long_chinese, style::LABEL_MAX_WIDTH, style::LABEL);
    assert!(truncated_zh.ends_with("..."));
    assert!(truncated_zh.chars().count() < long_chinese.chars().count());
    assert!(style::text_width(&truncated_zh, style::LABEL) <= style::LABEL_MAX_WIDTH);
}

#[test]
fn settings_updates_sync_preferences_to_launcher() {
    let board = Arc::new(Board::simulated());
    let mut launcher = Launcher::new(board);

    // Switch language via Settings message
    launcher.update(Message::Settings(settings::Message::SetLanguage(Language::English)));
    assert_eq!(launcher.preferences().language, Language::English);
    assert_eq!(launcher.language(), Language::English);

    // Switch theme via Settings message
    launcher.update(Message::Settings(settings::Message::SetTheme(ThemeMode::Light)));
    assert_eq!(launcher.preferences().theme, ThemeMode::Light);
    assert_eq!(launcher.calculator().theme_mode(), ThemeMode::Light);
    assert_eq!(launcher.counter().theme_mode(), ThemeMode::Light);
    assert_eq!(launcher.hello().theme_mode(), ThemeMode::Light);
    assert_eq!(launcher.music().theme_mode(), ThemeMode::Light);
    assert_eq!(launcher.terminal().theme_mode(), ThemeMode::Light);
    assert_eq!(launcher.daily_schedule().theme_mode(), ThemeMode::Light);

    // Cycle font tier via Settings message
    launcher.update(Message::Settings(settings::Message::CycleFontTier));
    assert_eq!(launcher.preferences().font_tier, FontSizeTier::Large);
    assert_eq!(launcher.preferences().font_tier.base_size(), 30.0);

    launcher.update(Message::Settings(settings::Message::CycleFontTier));
    assert_eq!(launcher.preferences().font_tier, FontSizeTier::Small);
    assert_eq!(launcher.preferences().font_tier.base_size(), 20.0);

    launcher.update(Message::Settings(settings::Message::CycleFontTier));
    assert_eq!(launcher.preferences().font_tier, FontSizeTier::Standard);
    assert_eq!(launcher.preferences().font_tier.base_size(), 24.0);

    // Cycle screen off timeout via Settings message
    launcher.update(Message::Settings(settings::Message::CycleScreenOffTimeout));
    assert_eq!(launcher.preferences().screen_off_timeout, ScreenOffTimeout::FifteenSeconds);
    assert_eq!(launcher.board.screen_off_timeout(), Some(Duration::from_secs(15)));

    launcher.update(Message::Settings(settings::Message::CycleScreenOffTimeout));
    assert_eq!(launcher.preferences().screen_off_timeout, ScreenOffTimeout::ThirtySeconds);
    assert_eq!(launcher.board.screen_off_timeout(), Some(Duration::from_secs(30)));

    launcher.update(Message::Settings(settings::Message::CycleScreenOffTimeout));
    assert_eq!(launcher.preferences().screen_off_timeout, ScreenOffTimeout::OneMinute);
    assert_eq!(launcher.board.screen_off_timeout(), Some(Duration::from_secs(60)));

    launcher.update(Message::Settings(settings::Message::CycleScreenOffTimeout));
    assert_eq!(launcher.preferences().screen_off_timeout, ScreenOffTimeout::FiveMinutes);
    assert_eq!(launcher.board.screen_off_timeout(), Some(Duration::from_secs(300)));

    launcher.update(Message::Settings(settings::Message::CycleScreenOffTimeout));
    assert_eq!(launcher.preferences().screen_off_timeout, ScreenOffTimeout::Never);
    assert_eq!(launcher.board.screen_off_timeout(), None);

    // Set preferences directly on launcher
    let custom_prefs = SystemPreferences::new(Language::Chinese, ThemeMode::Dark, FontSizeTier::Large);
    launcher.set_preferences(custom_prefs);
    assert_eq!(launcher.preferences(), custom_prefs);
    assert_eq!(launcher.settings().preferences(), custom_prefs);
    assert_eq!(launcher.calculator().preferences(), custom_prefs);
    assert_eq!(launcher.counter().preferences(), custom_prefs);
    assert_eq!(launcher.hello().preferences(), custom_prefs);
    assert_eq!(launcher.music().preferences(), custom_prefs);
    assert_eq!(launcher.terminal().preferences(), custom_prefs);
    assert_eq!(launcher.daily_schedule().preferences(), custom_prefs);
    assert_eq!(launcher.calculator().theme_mode(), ThemeMode::Dark);
    assert_eq!(launcher.counter().theme_mode(), ThemeMode::Dark);
    assert_eq!(launcher.hello().theme_mode(), ThemeMode::Dark);
    assert_eq!(launcher.music().theme_mode(), ThemeMode::Dark);
    assert_eq!(launcher.terminal().theme_mode(), ThemeMode::Dark);
    assert_eq!(launcher.daily_schedule().theme_mode(), ThemeMode::Dark);
}

#[test]
fn back_navigates_subpages_before_returning_to_grid() {
    let board = Arc::new(Board::simulated());
    let mut launcher = Launcher::new(Arc::clone(&board));

    // 1. Back while on Grid stays on Grid
    launcher.update(Message::Back);
    assert_eq!(launcher.screen, Screen::Grid);

    // 2. Single-page app (e.g. Calculator) exits directly to Grid
    launcher.update(Message::Open(AppId::Calculator));
    assert_eq!(launcher.screen, Screen::App(AppId::Calculator));
    launcher.update(Message::Back);
    assert_eq!(launcher.screen, Screen::Grid);

    // 3. Multi-page app (Settings) navigates internal subpages first
    launcher.update(Message::Open(AppId::Settings));
    assert_eq!(launcher.screen, Screen::App(AppId::Settings));
    assert_eq!(launcher.settings().section(), settings::SettingsSection::Main);

    // Open Wifi subpage
    launcher.update(Message::Settings(settings::Message::Open(settings::SettingsSection::Wifi)));
    assert_eq!(launcher.settings().section(), settings::SettingsSection::Wifi);

    for _ in 0..3 {
        board.tick();
    }
    launcher.update(Message::Settings(settings::Message::WifiFrame(
        std::time::Instant::now() + std::time::Duration::from_secs(1),
    )));

    // Open password prompt dialog
    launcher.update(Message::Settings(settings::Message::WifiSelect(0)));
    assert!(launcher.settings().wifi().prompt().is_some());

    // Hardware Back (Message::Back) closes prompt, remains on Wifi subpage
    launcher.update(Message::Back);
    assert!(launcher.settings().wifi().prompt().is_none());
    assert_eq!(launcher.settings().section(), settings::SettingsSection::Wifi);
    assert_eq!(launcher.screen, Screen::App(AppId::Settings));

    // Hardware Back (Message::Back) returns to Settings Main, remains in Settings
    launcher.update(Message::Back);
    assert_eq!(launcher.settings().section(), settings::SettingsSection::Main);
    assert_eq!(launcher.screen, Screen::App(AppId::Settings));

    // Hardware Back from Main exits Settings to Grid
    launcher.update(Message::Back);
    assert_eq!(launcher.screen, Screen::Grid);
}

#[test]
fn alarm_back_navigates_levels_before_exiting_to_grid() {
    let board = Arc::new(Board::simulated());
    let mut launcher = Launcher::new(Arc::clone(&board));

    // 1. Open Alarm app from grid
    launcher.update(Message::Open(AppId::Alarm));
    assert_eq!(launcher.screen, Screen::App(AppId::Alarm));
    assert_eq!(
        launcher.alarm().mode,
        alarm_clock::ViewMode::List { is_editing_list: false }
    );

    // 2. Open Add Alarm (Level 2: Edit ActionSheet)
    launcher.update(Message::Alarm(alarm_clock::Message::OpenNew));
    assert!(matches!(
        launcher.alarm().mode,
        alarm_clock::ViewMode::Edit { sub_sheet: None, .. }
    ));

    // 3. Open Repeat selection (Level 3: Selection SubPage)
    launcher.update(Message::Alarm(alarm_clock::Message::OpenRoute(
        alarm_clock::AlarmRoute::Repeat,
    )));
    assert!(matches!(
        launcher.alarm().mode,
        alarm_clock::ViewMode::Edit {
            sub_sheet: Some(alarm_clock::AlarmRoute::Repeat),
            ..
        }
    ));

    // 4. Hardware Back (Message::Back) pops Level 3 -> Level 2, remains in Alarm
    launcher.update(Message::Back);
    assert_eq!(launcher.screen, Screen::App(AppId::Alarm));
    assert!(matches!(
        launcher.alarm().mode,
        alarm_clock::ViewMode::Edit { sub_sheet: None, .. }
    ));

    // 5. Hardware Back (Message::Back) pops Level 2 -> Level 1 (List), remains in Alarm
    launcher.update(Message::Back);
    assert_eq!(launcher.screen, Screen::App(AppId::Alarm));
    assert_eq!(
        launcher.alarm().mode,
        alarm_clock::ViewMode::List { is_editing_list: false }
    );

    // 6. Hardware Back (Message::Back) from Root List exits Alarm to Grid
    launcher.update(Message::Back);
    assert_eq!(launcher.screen, Screen::Grid);
}

#[test]
fn back_keeps_app_running_in_background_and_exit_kills_app() {
    let board = Arc::new(Board::simulated());
    let mut launcher = Launcher::new(Arc::clone(&board));

    // Initially no apps running
    assert!(launcher.running_apps().is_empty());

    // 1. Open Music: it is added to running_apps
    launcher.update(Message::Open(AppId::Music));
    assert_eq!(launcher.screen, Screen::App(AppId::Music));
    assert!(launcher.is_app_running(AppId::Music));
    assert_eq!(launcher.running_apps(), &[AppId::Music]);

    // 2. Press Back: returns to Grid, but app continues running in background
    launcher.update(Message::Back);
    assert_eq!(launcher.screen, Screen::Grid);
    assert!(launcher.is_app_running(AppId::Music));

    // 3. Open Calculator as well: both apps running in memory
    launcher.update(Message::Open(AppId::Calculator));
    assert_eq!(launcher.screen, Screen::App(AppId::Calculator));
    assert!(launcher.is_app_running(AppId::Calculator));
    assert_eq!(launcher.running_apps(), &[AppId::Music, AppId::Calculator]);

    // 4. Press Exit in Calculator: kills Calculator and returns to Grid
    launcher.update(Message::Exit);
    assert_eq!(launcher.screen, Screen::Grid);
    assert!(!launcher.is_app_running(AppId::Calculator));
    assert_eq!(launcher.running_apps(), &[AppId::Music]);

    // 5. Press Exit on Grid: kills the last background app (Music)
    launcher.update(Message::Exit);
    assert!(!launcher.is_app_running(AppId::Music));
    assert!(launcher.running_apps().is_empty());

    // 6. Test Settings reset on kill: navigate to Wifi subpage, kill app, next open is fresh
    launcher.update(Message::Open(AppId::Settings));
    launcher.update(Message::Settings(settings::Message::Open(settings::SettingsSection::Wifi)));
    assert_eq!(launcher.settings().section(), settings::SettingsSection::Wifi);

    launcher.update(Message::Exit);
    assert_eq!(launcher.screen, Screen::Grid);
    assert!(!launcher.is_app_running(AppId::Settings));

    launcher.update(Message::Open(AppId::Settings));
    assert_eq!(launcher.settings().section(), settings::SettingsSection::Main);
}

#[test]
fn home_returns_to_grid_and_preserves_app_state() {
    let board = Arc::new(Board::simulated());
    let mut launcher = Launcher::new(Arc::clone(&board));

    // 1. Open Settings and enter Wifi subpage
    launcher.update(Message::Open(AppId::Settings));
    launcher.update(Message::Settings(settings::Message::Open(settings::SettingsSection::Wifi)));
    assert_eq!(launcher.screen, Screen::App(AppId::Settings));
    assert_eq!(launcher.settings().section(), settings::SettingsSection::Wifi);

    // 2. Press Home: immediately returns to Grid, but Settings stays running in background
    launcher.update(Message::Home);
    assert_eq!(launcher.screen, Screen::Grid);
    assert!(launcher.is_app_running(AppId::Settings));
    assert_eq!(launcher.running_apps(), &[AppId::Settings]);

    // 3. Reopen Settings from Grid: state is fully preserved! Section is still Wifi!
    launcher.update(Message::Open(AppId::Settings));
    assert_eq!(launcher.screen, Screen::App(AppId::Settings));
    assert_eq!(launcher.settings().section(), settings::SettingsSection::Wifi);

    // 4. Open DemoCounter alongside Settings
    launcher.update(Message::Open(AppId::DemoCounter));
    assert_eq!(launcher.screen, Screen::App(AppId::DemoCounter));
    launcher.update(Message::Counter(demo_counter::Message::Tap));
    assert_eq!(launcher.counter().count(), 1);

    // 5. Press Home from DemoCounter: returns to Grid, both apps still running in background
    launcher.update(Message::Home);
    assert_eq!(launcher.screen, Screen::Grid);
    assert!(launcher.is_app_running(AppId::DemoCounter));
    assert!(launcher.is_app_running(AppId::Settings));

    // 6. Reopen DemoCounter: count is still 1!
    launcher.update(Message::Open(AppId::DemoCounter));
    assert_eq!(launcher.counter().count(), 1);

    // 7. Press Exit (KEY3): DemoCounter is killed
    launcher.update(Message::Exit);
    assert_eq!(launcher.screen, Screen::Grid);
    assert!(!launcher.is_app_running(AppId::DemoCounter));
    assert!(launcher.is_app_running(AppId::Settings));
}

#[test]
fn status_updates_clock_battery_and_settings() {
    let board = Arc::new(Board::simulated());
    let mut launcher = Launcher::new(Arc::clone(&board));

    launcher.update(Message::Status("14:30".to_string(), 85, true, 3));
    assert_eq!(launcher.clock, "14:30");
    assert_eq!(launcher.battery, 85);
    assert!(launcher.charging);
    assert_eq!(launcher.wifi, 3);
    assert_eq!(launcher.settings().battery().percent, 85);
    assert!(launcher.settings().battery().charging);
}

#[test]
fn page_changed_updates_launcher_page() {
    let board = Arc::new(Board::simulated());
    let mut launcher = Launcher::new(board);
    assert_eq!(launcher.page, 0);

    launcher.update(Message::PageChanged(1));
    assert_eq!(launcher.page, 1);
}

#[test]
fn grid_view_builds_and_all_pages_render_without_panics() {
    let board = Arc::new(Board::simulated());
    let launcher = Launcher::new(board);
    let _view = launcher.view();

    for p in 0..launcher.pages() {
        let _page = launcher.page(p);
    }
}

#[test]
fn apps_are_lazily_loaded_on_demand_and_freed_on_kill() {
    let board = Arc::new(Board::simulated());
    let mut launcher = Launcher::new(board);

    // At boot, all sub-apps are None (0 boot CPU time / 0 heap allocations for apps)
    assert!(launcher.calculator.is_none());
    assert!(launcher.counter.is_none());
    assert!(launcher.hello.is_none());
    assert!(launcher.settings.is_none());
    assert!(launcher.music.is_none());
    assert!(launcher.terminal.is_none());
    assert!(launcher.daily_schedule.is_none());
    assert!(launcher.wireless_clipboard.is_none());

    // Opening Calculator instantiates only Calculator
    launcher.update(Message::Open(AppId::Calculator));
    assert!(launcher.calculator.is_some());
    assert!(launcher.counter.is_none());
    assert!(launcher.hello.is_none());
    assert!(launcher.settings.is_none());
    assert!(launcher.music.is_none());
    assert!(launcher.terminal.is_none());
    assert!(launcher.daily_schedule.is_none());
    assert!(launcher.wireless_clipboard.is_none());

    // Backgrounding Calculator (Back) preserves instance
    launcher.update(Message::Back);
    assert_eq!(launcher.screen, Screen::Grid);
    assert!(launcher.calculator.is_some());

    // Exiting / killing Calculator frees its heap memory (reverts to None)
    launcher.update(Message::Exit);
    assert!(launcher.calculator.is_none());

    // Opening Daily Schedule instantiates only Daily Schedule
    launcher.update(Message::Open(AppId::DailySchedule));
    assert!(launcher.daily_schedule.is_some());

    // Backgrounding Daily Schedule (Back) preserves instance
    launcher.update(Message::Back);
    assert_eq!(launcher.screen, Screen::Grid);
    assert!(launcher.daily_schedule.is_some());

    // Exiting / killing Daily Schedule frees its heap memory (reverts to None)
    launcher.update(Message::Exit);
    assert!(launcher.daily_schedule.is_none());

    // Opening Wireless Clipboard instantiates only Wireless Clipboard
    launcher.update(Message::Open(AppId::WirelessClipboard));
    assert!(launcher.wireless_clipboard.is_some());

    // Backgrounding Wireless Clipboard (Back) preserves instance
    launcher.update(Message::Back);
    assert_eq!(launcher.screen, Screen::Grid);
    assert!(launcher.wireless_clipboard.is_some());

    // Exiting / killing Wireless Clipboard frees its heap memory (reverts to None)
    launcher.update(Message::Exit);
    assert!(launcher.wireless_clipboard.is_none());
}

#[test]
fn boot_initializes_wifi_autoconnect_task() {
    let board = Arc::new(Board::simulated());
    let (launcher, _task) = Launcher::boot(board);
    assert_eq!(
        launcher.task_manager().status(SystemTaskId::WifiAutoConnect),
        TaskStatus::Running
    );
}

#[test]
fn system_task_wifi_connected_triggers_time_sync() {
    let board = Arc::new(Board::simulated());
    // Simulate Wi-Fi connected with saved network
    board.wifi().set_enabled(true).unwrap();
    board.wifi().connect("Pomelo-OS", "password").unwrap();
    assert_eq!(board.wifi().status().state, pomelo_hal::WifiState::Connected);
    assert!(!board.time().is_synced());

    let (mut launcher, _) = Launcher::boot(Arc::clone(&board));

    // When WifiAutoConnect finishes successfully
    launcher.update(Message::SystemTask(SystemTaskMessage::Finished {
        id: SystemTaskId::WifiAutoConnect,
        result: Ok(()),
    }));

    // TimeSync should be marked running and dispatched
    assert_eq!(
        launcher.task_manager().status(SystemTaskId::TimeSync),
        TaskStatus::Running
    );

    // When TimeSync finishes successfully
    launcher.update(Message::SystemTask(SystemTaskMessage::Finished {
        id: SystemTaskId::TimeSync,
        result: Ok(()),
    }));

    assert_eq!(
        launcher.task_manager().status(SystemTaskId::TimeSync),
        TaskStatus::Success
    );
}

#[test]
fn status_message_with_connected_wifi_triggers_time_sync_if_not_synced() {
    let board = Arc::new(Board::simulated());
    let mut launcher = Launcher::new(Arc::clone(&board));
    assert!(!board.time().is_synced());
    assert_eq!(
        launcher.task_manager().status(SystemTaskId::TimeSync),
        TaskStatus::Idle
    );

    // Status message with wifi signal bars > 0 (meaning connected)
    launcher.update(Message::Status("10:00".into(), 80, false, 3));

    // Automatically launches TimeSync
    assert_eq!(
        launcher.task_manager().status(SystemTaskId::TimeSync),
        TaskStatus::Running
    );
}

#[test]
fn app_preferences_trait_and_propagation_across_apps() {
    let board = Arc::new(Board::simulated());
    let mut launcher = Launcher::new(board);

    // Instantiate all hosted apps
    let _ = launcher.get_or_create_calculator();
    let _ = launcher.get_or_create_counter();
    let _ = launcher.get_or_create_hello();
    let _ = launcher.get_or_create_settings();
    let _ = launcher.get_or_create_music();
    let _ = launcher.get_or_create_terminal();
    let _ = launcher.get_or_create_daily_schedule();
    let _ = launcher.get_or_create_wireless_clipboard();

    let mut count = 0;
    launcher.for_each_active_app_mut(|app| {
        count += 1;
        assert_eq!(app.theme_mode(), ThemeMode::Dark);
        assert_eq!(app.language(), Language::Chinese);
    });
    assert_eq!(count, 8);

    // Update preferences through AppPreferences trait on launcher
    let new_prefs = SystemPreferences {
        language: Language::English,
        theme: ThemeMode::Light,
        font_tier: FontSizeTier::Large,
        screen_off_timeout: ScreenOffTimeout::FifteenSeconds,
        stay_awake_on_usb: true,
    };
    launcher.set_preferences(new_prefs);

    assert_eq!(launcher.theme_mode(), ThemeMode::Light);
    assert_eq!(launcher.language(), Language::English);

    let mut updated_count = 0;
    launcher.for_each_active_app_mut(|app| {
        updated_count += 1;
        assert_eq!(app.theme_mode(), ThemeMode::Light);
        assert_eq!(app.language(), Language::English);
        assert_eq!(app.preferences().font_tier, FontSizeTier::Large);
        assert_eq!(app.preferences().screen_off_timeout, ScreenOffTimeout::FifteenSeconds);
    });
    assert_eq!(updated_count, 8);
}

#[test]
fn alarm_ringing_keeps_screen_on_and_clears_on_dismiss_or_navigation() {
    let board = Arc::new(Board::simulated());
    let mut launcher = Launcher::new(Arc::clone(&board));

    // Initially on Grid: keep_screen_on is false
    assert!(!launcher.keep_screen_on());
    assert!(!board.is_keep_screen_on());

    // 1. Trigger alarm ringing by sending CheckSchedule when an alarm is due or manually triggering ringing
    launcher.update(Message::Open(AppId::Alarm));
    assert!(!launcher.keep_screen_on());
    assert!(!board.is_keep_screen_on());

    // Switch alarm to Ringing mode
    let alarm = launcher.get_or_create_alarm();
    alarm.mode = alarm_clock::ViewMode::Ringing {
        alarm_id: 1,
        time_str: "08:00".to_string(),
        label: "Wake Up".to_string(),
        sound_id: 1,
        session_id: 42,
    };
    launcher.sync_keep_screen_on();

    assert!(launcher.keep_screen_on(), "Launcher should keep screen on while alarm is ringing");
    assert!(board.is_keep_screen_on(), "Board should have keep_screen_on set to true");

    // 2. Dismissing alarm (e.g. RingTimeout or Dismiss) resets keep_screen_on
    launcher.update(Message::Alarm(alarm_clock::Message::RingTimeout { session_id: 42 }));
    assert!(!launcher.keep_screen_on(), "Launcher clears keep_screen_on after ring timeout");
    assert!(!board.is_keep_screen_on(), "Board clears keep_screen_on after ring timeout");

    // 3. Ring again, and user presses Home to go to Grid
    let alarm = launcher.get_or_create_alarm();
    alarm.mode = alarm_clock::ViewMode::Ringing {
        alarm_id: 2,
        time_str: "09:00".to_string(),
        label: "Meeting".to_string(),
        sound_id: 1,
        session_id: 43,
    };
    launcher.sync_keep_screen_on();
    assert!(board.is_keep_screen_on());

    launcher.update(Message::Home);
    assert_eq!(launcher.screen, Screen::Grid);
    assert!(!launcher.keep_screen_on(), "Leaving alarm screen clears keep_screen_on");
    assert!(!board.is_keep_screen_on(), "Board keep_screen_on is false when on Grid");
}

