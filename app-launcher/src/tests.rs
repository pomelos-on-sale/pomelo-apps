// The tests drive `update` by hand, and a test has no loop to return the `Task` it produces to.
// What that task does — the settings app scrolling its own body — belongs to the loop.
#![allow(unused_must_use)]
use super::*;

#[test]
fn default_preferences_are_chinese_dark_standard() {
    let board = Arc::new(Board::simulated());
    let launcher = Launcher::new(board);

    assert_eq!(launcher.preferences().language, Language::Chinese);
    assert_eq!(launcher.preferences().theme, ThemeMode::Dark);
    assert_eq!(launcher.preferences().font_tier, FontSizeTier::Standard);
}

#[test]
fn catalogue_entries_are_localized_in_both_languages() {
    for entry in CATALOGUE {
        if entry.name != "demo-counter" {
            assert_ne!(entry.localized_name(Language::Chinese), entry.localized_name(Language::English));
        }
        assert!(!entry.localized_name(Language::Chinese).is_empty());
        assert!(!entry.localized_name(Language::English).is_empty());
    }

    assert_eq!(CATALOGUE[TERMINAL].localized_name(Language::Chinese), "终端");
    assert_eq!(CATALOGUE[CALCULATOR].localized_name(Language::Chinese), "计算器");
    assert_eq!(CATALOGUE[COUNTER].localized_name(Language::Chinese), "demo-counter");
    assert_eq!(CATALOGUE[HELLO].localized_name(Language::Chinese), "你好");
    assert_eq!(CATALOGUE[SETTINGS].localized_name(Language::Chinese), "设置");
    assert_eq!(CATALOGUE[MUSIC].localized_name(Language::Chinese), "音乐");
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

    // Cycle font tier via Settings message
    launcher.update(Message::Settings(settings::Message::CycleFontTier));
    assert_eq!(launcher.preferences().font_tier, FontSizeTier::Large);
    assert_eq!(launcher.preferences().font_tier.base_size(), 30.0);

    launcher.update(Message::Settings(settings::Message::CycleFontTier));
    assert_eq!(launcher.preferences().font_tier, FontSizeTier::ExtraSmall);
    assert_eq!(launcher.preferences().font_tier.base_size(), 18.0);

    launcher.update(Message::Settings(settings::Message::CycleFontTier));
    assert_eq!(launcher.preferences().font_tier, FontSizeTier::Small);
    assert_eq!(launcher.preferences().font_tier.base_size(), 20.0);

    launcher.update(Message::Settings(settings::Message::CycleFontTier));
    assert_eq!(launcher.preferences().font_tier, FontSizeTier::Standard);
    assert_eq!(launcher.preferences().font_tier.base_size(), 24.0);

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
    assert_eq!(launcher.calculator().theme_mode(), ThemeMode::Dark);
    assert_eq!(launcher.counter().theme_mode(), ThemeMode::Dark);
    assert_eq!(launcher.hello().theme_mode(), ThemeMode::Dark);
    assert_eq!(launcher.music().theme_mode(), ThemeMode::Dark);
    assert_eq!(launcher.terminal().theme_mode(), ThemeMode::Dark);
}

#[test]
fn back_navigates_subpages_before_returning_to_grid() {
    let board = Arc::new(Board::simulated());
    let mut launcher = Launcher::new(Arc::clone(&board));

    // 1. Back while on Grid stays on Grid
    launcher.update(Message::Back);
    assert_eq!(launcher.screen, Screen::Grid);

    // 2. Single-page app (e.g. Calculator) exits directly to Grid
    launcher.update(Message::Open(CALCULATOR));
    assert_eq!(launcher.screen, Screen::App(CALCULATOR));
    launcher.update(Message::Back);
    assert_eq!(launcher.screen, Screen::Grid);

    // 3. Multi-page app (Settings) navigates internal subpages first
    launcher.update(Message::Open(SETTINGS));
    assert_eq!(launcher.screen, Screen::App(SETTINGS));
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
    assert_eq!(launcher.screen, Screen::App(SETTINGS));

    // Hardware Back (Message::Back) returns to Settings Main, remains in Settings
    launcher.update(Message::Back);
    assert_eq!(launcher.settings().section(), settings::SettingsSection::Main);
    assert_eq!(launcher.screen, Screen::App(SETTINGS));

    // Hardware Back from Main exits Settings to Grid
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
    launcher.update(Message::Open(MUSIC));
    assert_eq!(launcher.screen, Screen::App(MUSIC));
    assert!(launcher.is_app_running(MUSIC));
    assert_eq!(launcher.running_apps(), &[MUSIC]);

    // 2. Press Back: returns to Grid, but app continues running in background
    launcher.update(Message::Back);
    assert_eq!(launcher.screen, Screen::Grid);
    assert!(launcher.is_app_running(MUSIC));

    // 3. Open Calculator as well: both apps running in memory
    launcher.update(Message::Open(CALCULATOR));
    assert_eq!(launcher.screen, Screen::App(CALCULATOR));
    assert!(launcher.is_app_running(CALCULATOR));
    assert_eq!(launcher.running_apps(), &[MUSIC, CALCULATOR]);

    // 4. Press Exit in Calculator: kills Calculator and returns to Grid
    launcher.update(Message::Exit);
    assert_eq!(launcher.screen, Screen::Grid);
    assert!(!launcher.is_app_running(CALCULATOR));
    assert_eq!(launcher.running_apps(), &[MUSIC]);

    // 5. Press Exit on Grid: kills the last background app (Music)
    launcher.update(Message::Exit);
    assert!(!launcher.is_app_running(MUSIC));
    assert!(launcher.running_apps().is_empty());

    // 6. Test Settings reset on kill: navigate to Wifi subpage, kill app, next open is fresh
    launcher.update(Message::Open(SETTINGS));
    launcher.update(Message::Settings(settings::Message::Open(settings::SettingsSection::Wifi)));
    assert_eq!(launcher.settings().section(), settings::SettingsSection::Wifi);

    launcher.update(Message::Exit);
    assert_eq!(launcher.screen, Screen::Grid);
    assert!(!launcher.is_app_running(SETTINGS));

    launcher.update(Message::Open(SETTINGS));
    assert_eq!(launcher.settings().section(), settings::SettingsSection::Main);
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

    // Opening Calculator instantiates only Calculator
    launcher.update(Message::Open(CALCULATOR));
    assert!(launcher.calculator.is_some());
    assert!(launcher.counter.is_none());
    assert!(launcher.hello.is_none());
    assert!(launcher.settings.is_none());
    assert!(launcher.music.is_none());
    assert!(launcher.terminal.is_none());

    // Backgrounding Calculator (Back) preserves instance
    launcher.update(Message::Back);
    assert_eq!(launcher.screen, Screen::Grid);
    assert!(launcher.calculator.is_some());

    // Exiting / killing Calculator frees its heap memory (reverts to None)
    launcher.update(Message::Exit);
    assert!(launcher.calculator.is_none());
}
