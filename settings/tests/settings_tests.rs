//! The settings app's own tests: what the pages say, through the widget tree.
//!
//! Nothing here names a platform layer, reads a panel or draws a pixel. `iced_test`'s simulator
//! builds the *real* widget tree, finds a control by the text it is drawn with, presses it, and
//! hands back the message that produced. What the app costs the board — frames, damage, a finger
//! on the panel, a scroll — is asserted in `firmware/panel-tests`, where the panel is.

use std::sync::Arc;
use std::time::Duration;

use iced::time::Instant;
use iced::Size;
use iced_test::Simulator;
use pomelo_hal::{Board, ScanState, WifiState};
use pomelo_widgets::touch_keyboard::{KeyAction, KeyboardMode};

use settings::{FontSizeTier, Language, Message, Settings, SettingsSection, ThemeMode};

/// The panel the layout is designed for. The app fills whatever it is given; a test has to pick a
/// size, and this is the one the design was drawn at.
const PANEL: f32 = 480.0;

/// The app in English, which is the language the labels below are written in.
///
/// The default is Chinese, so a test that means to assert a label has to say which language it is
/// asserting — otherwise it would pass or fail depending on a default it does not control.
fn english() -> Settings {
    let mut settings = Settings::new(simulated());
    settings.update(Message::SetLanguage(Language::English));
    settings
}

/// A board with no hardware behind it: the HAL's simulator, which is what every test here runs
/// against. Five access points, a scan that needs three `tick`s, and a connection that comes up as
/// soon as the network accepts the password.
fn simulated() -> Arc<Board> {
    Arc::new(Board::simulated())
}

/// Every page: the label of its row in the main list, the section it opens, and the title over it.
///
/// The two labels differ on purpose in three places — the row says `Battery` and the page says
/// `Battery & Power`, the row says `System` and the page says `About`, the row says
/// `Theme & Display` and the page says `Display & Theme` — which is the original's wording and
/// worth keeping: the row is what the section is *about*, the title is where you are.
const PAGES: [(&str, SettingsSection, &str); 7] = [
    ("Wi-Fi", SettingsSection::Wifi, "Wi-Fi"),
    ("Memory (RAM)", SettingsSection::Memory, "Memory"),
    ("Storage", SettingsSection::Storage, "Storage"),
    ("Battery", SettingsSection::Battery, "Battery & Power"),
    ("System", SettingsSection::SystemInfo, "About"),
    ("Theme & Display", SettingsSection::Theme, "Display & Theme"),
    ("Date & Time", SettingsSection::Time, "Date & Time"),
];

fn interface(settings: &Settings) -> Simulator<'_, Message> {
    Simulator::with_size(
        iced::Settings {
            // By *generic* family: the tester's default is "Fira Sans", which only exists when
            // iced's `fira-sans` feature is on — 441 KiB of glyphs this firmware has no room for.
            default_font: iced::Font::MONOSPACE,
            ..iced::Settings::default()
        },
        Size::new(PANEL, PANEL),
        settings.view(),
    )
}

/// Presses `label` in a freshly built tree of `settings`' current state, and applies what came back.
fn press(settings: &mut Settings, label: &str) -> Result<(), iced_test::Error> {
    let mut ui = interface(settings);
    let _ = ui.click(label)?;

    for message in ui.into_messages() {
        settings.update(message);
    }

    Ok(())
}

#[test]
fn a_row_opens_its_section() -> Result<(), iced_test::Error> {
    let mut settings = english();

    assert_eq!(settings.section(), SettingsSection::Main);

    press(&mut settings, "Memory (RAM)")?;

    assert_eq!(settings.section(), SettingsSection::Memory);
    Ok(())
}

#[test]
fn every_row_opens_its_own_section() -> Result<(), iced_test::Error> {
    let mut settings = english();

    for (row, section, _) in PAGES {
        press(&mut settings, row)?;
        assert_eq!(settings.section(), section, "the row labelled {row}");

        // And back, which is also the only way the next row is on screen again.
        press(&mut settings, "Back")?;
        assert_eq!(settings.section(), SettingsSection::Main, "after {row}");
    }

    Ok(())
}

/// The default is the original's language, and the language row *is* the switch.
///
/// Two things at once, and both are the point: a fresh app speaks Chinese, and pressing the row
/// hands it the other language — iced rebuilds the view from the new state, so the label changes
/// with no redraw machinery of our own.
#[test]
fn the_language_row_switches_between_chinese_and_english() -> Result<(), iced_test::Error> {
    let mut settings = Settings::new(simulated());

    assert_eq!(settings.language(), Language::Chinese);

    {
        let mut ui = interface(&settings);

        assert!(
            ui.find("内存 (RAM)").is_ok(),
            "the list is Chinese by default"
        );
        assert!(ui.find("Memory (RAM)").is_err(), "and not English");
    }

    press(&mut settings, "语言")?;

    assert_eq!(settings.language(), Language::English);

    let mut ui = interface(&settings);

    assert!(
        ui.find("Memory (RAM)").is_ok(),
        "the row switched it to English"
    );
    assert!(ui.find("内存 (RAM)").is_err(), "and Chinese is gone");

    Ok(())
}

/// Switching the language is app-wide state, so a page that is open stays open.
#[test]
fn switching_the_language_keeps_the_page() {
    let mut settings = Settings::new(simulated());
    settings.update(Message::Open(SettingsSection::Memory));

    {
        let mut ui = interface(&settings);

        assert!(
            ui.find("内部 SRAM").is_ok(),
            "the page's labels are Chinese"
        );
    }

    settings.update(Message::SetLanguage(Language::English));

    let mut ui = interface(&settings);

    assert!(ui.find("Internal SRAM").is_ok(), "and English afterwards");
    assert_eq!(
        settings.section(),
        SettingsSection::Memory,
        "the page is still the one that was open"
    );
}

#[test]
fn back_returns_to_the_main_list() -> Result<(), iced_test::Error> {
    let mut settings = english();

    settings.update(Message::Open(SettingsSection::Wifi));
    assert_eq!(settings.section(), SettingsSection::Wifi);

    press(&mut settings, "Back")?;

    assert_eq!(settings.section(), SettingsSection::Main);
    Ok(())
}

/// The main list is the one page with nowhere to go back to, so it must not offer to.
#[test]
fn the_main_list_has_no_back_button() {
    let settings = english();
    let mut ui = interface(&settings);

    assert!(
        ui.find("Back").is_err(),
        "there is nowhere to go back to from the list"
    );
}

/// Every page says where it is, and only the list has no way back.
#[test]
fn every_page_is_titled_and_can_go_back() {
    let mut settings = english();

    for (row, section, title) in PAGES {
        settings.update(Message::Open(section));

        let mut ui = interface(&settings);

        assert!(ui.find(title).is_ok(), "the {row} page is not titled");
        assert!(
            ui.find("Back").is_ok(),
            "the {row} page cannot go back to the list"
        );
    }

    settings.update(Message::Open(SettingsSection::Main));

    let mut ui = interface(&settings);
    assert!(ui.find("Settings").is_ok(), "the list is titled");
    assert!(ui.find("Back").is_err());
}

/// The Wi-Fi page reads out what the switch is doing.
///
/// The switch itself is a `toggler` with no label — the text beside it is the label — so the panel
/// tests are where a finger on the switch is exercised. Here the model is asked directly, which is
/// what the switch's own message does, and the page is checked for saying so.
#[test]
fn the_wi_fi_page_says_what_the_switch_is_doing() -> Result<(), iced_test::Error> {
    let mut settings = english();
    settings.update(Message::Open(SettingsSection::Wifi));

    assert!(settings.wifi_enabled());

    {
        // Scoped, because the tree borrows the state it was built from: a simulator alive across
        // an `update` would be a tree of a state that no longer exists.
        let mut ui = interface(&settings);
        assert!(
            ui.find("Scanning…").is_ok(),
            "opening the page asks the radio for a scan, and the page says so"
        );
    }

    settings.update(Message::ToggleWifi);
    assert!(!settings.wifi_enabled());

    let mut ui = interface(&settings);
    assert!(
        ui.find("Wi-Fi is off").is_ok(),
        "and says so once it is off"
    );
    assert!(
        ui.find("192.168.1.108").is_err(),
        "the address is not still on the page"
    );
    assert!(
        ui.find("Scanning…").is_err(),
        "and there is nothing scanning behind a radio that is off"
    );

    Ok(())
}

/// The 24-hour switch is the other one, and its page is the one that shows it.
#[test]
fn the_time_page_follows_the_24_hour_switch() {
    let mut settings = Settings::new(simulated());
    settings.update(Message::Open(SettingsSection::Time));

    assert!(settings.is_24h_format(), "the original starts at 24-hour");

    {
        let mut ui = interface(&settings);
        assert!(ui.find("10:24").is_ok(), "24-hour has no suffix");
    }

    settings.update(Message::Toggle24Hour);
    assert!(!settings.is_24h_format());

    let mut ui = interface(&settings);
    assert!(ui.find("10:24 AM").is_ok(), "12-hour adds the suffix");
    assert!(ui.find("10:24").is_err());
}

/// The battery page shows the reading the platform pushed in, not one of its own.
#[test]
fn the_battery_page_shows_the_reading_it_was_given() {
    let mut settings = Settings::new(simulated());
    settings.update(Message::Open(SettingsSection::Battery));

    settings.set_battery(settings::Battery {
        percent: 42,
        charging: false,
        voltage_mv: 3_700,
    });

    let mut ui = interface(&settings);

    assert!(ui.find("42%").is_ok(), "the level it was given");
    assert!(ui.find("3700 mV").is_ok(), "and the voltage");
}

// =============================================================================
// The Wi-Fi page
// =============================================================================

/// The board a Wi-Fi test drives, and the app reading it.
///
/// The `Arc` is shared on purpose: the page reads the radio through its own handle, and the test
/// drives it through this one. `SimWifi` only advances on `tick`, which is what makes these tests
/// deterministic instead of time-dependent.
fn wifi_app() -> (Arc<Board>, Settings) {
    let board = simulated();
    let mut settings = Settings::new(Arc::clone(&board));
    settings.update(Message::SetLanguage(Language::English));

    (board, settings)
}

/// The Wi-Fi page, open, with a finished scan behind it.
fn scanned() -> (Arc<Board>, Settings) {
    let (board, mut settings) = wifi_app();

    settings.update(Message::Open(SettingsSection::Wifi));
    for _ in 0..3 {
        board.tick();
    }
    frame(&mut settings, Duration::from_secs(1));

    (board, settings)
}

/// One frame of the Wi-Fi page's subscription.
///
/// The instant is the caller's, because the page reads the radio at most four times a second: a
/// test that read the clock itself would be testing the throttle half the time, and would pass or
/// fail depending on how busy the machine was.
fn frame(settings: &mut Settings, after: Duration) {
    settings.update(Message::WifiFrame(Instant::now() + after));
}

/// Opening the page is what starts the scan — and the page shows the radio, not a guess.
#[test]
fn opening_the_wifi_page_starts_a_scan() {
    let (board, mut settings) = wifi_app();

    settings.update(Message::Open(SettingsSection::Wifi));

    assert_eq!(board.wifi().scan_state(), ScanState::Scanning);
    assert!(settings.access_points().is_empty(), "nothing yet");

    frame(&mut settings, Duration::ZERO);
    assert!(
        settings.access_points().is_empty(),
        "the simulator needs three ticks before it has anything"
    );

    for _ in 0..3 {
        board.tick();
    }
    frame(&mut settings, Duration::from_secs(1));

    assert_eq!(settings.access_points().len(), 5);

    let found = settings.access_points();
    assert_eq!(found[0].ssid, "ESP-Rust-5G", "strongest first");
    assert!(
        found[0].rssi >= found[1].rssi && found[1].rssi >= found[2].rssi,
        "and the rest in order: {:?}",
        found.iter().map(|ap| ap.rssi).collect::<Vec<_>>()
    );
}

/// The list is drawn from what the scan found: names, the lock, and the way to scan again.
#[test]
fn the_list_shows_what_the_scan_found() -> Result<(), iced_test::Error> {
    let (_, settings) = scanned();

    let mut ui = interface(&settings);

    assert!(ui.find("ESP-Rust-5G").is_ok(), "a network by name");
    assert!(ui.find("OpenGuest").is_ok(), "the open one too");
    assert!(ui.find("secured").is_ok(), "and the lock, as a word");
    assert!(ui.find("Scan again").is_ok(), "the scan is not running");

    Ok(())
}

/// A locked network asks; the password is what connects it.
#[test]
fn a_locked_network_asks_for_its_password() {
    let (_, mut settings) = scanned();

    settings.update(Message::WifiSelect(0));
    assert_eq!(
        settings.password_prompt().map(|ap| ap.ssid.as_str()),
        Some("ESP-Rust-5G"),
        "a locked network opens the prompt"
    );

    // No password is refused by the radio, and the prompt is still there to say so.
    settings.update(Message::WifiConnect);
    assert_ne!(settings.wifi_status().state, WifiState::Connected);
    assert!(settings.password_prompt().is_some(), "still asking");

    {
        let mut ui = interface(&settings);
        assert!(
            ui.find("Connection failed").is_ok(),
            "and the page says why"
        );
    }

    for character in "hunter2".chars() {
        settings.update(Message::WifiKey(KeyAction::Char(character)));
    }
    assert_eq!(settings.password(), "hunter2");

    settings.update(Message::WifiConnect);

    assert!(settings.password_prompt().is_none(), "the prompt is done");
    assert_eq!(settings.wifi_status().state, WifiState::Connected);
    assert_eq!(settings.wifi_status().ssid, "ESP-Rust-5G");
}

/// The prompt's keyboard is the shared one: the mode switch is the keyboard's own state.
#[test]
fn the_prompt_has_the_whole_keyboard() -> Result<(), iced_test::Error> {
    let (_, mut settings) = scanned();

    settings.update(Message::WifiSelect(0));

    {
        let mut ui = interface(&settings);

        assert!(ui.find("q").is_ok(), "letters first");
        assert!(ui.find("123").is_ok(), "with a way to the numbers");
    }

    settings.update(Message::WifiKey(KeyAction::SwitchMode(KeyboardMode::Numbers)));

    {
        let mut ui = interface(&settings);

        assert!(ui.find("#+=").is_ok(), "the numeric page is up");
        assert!(ui.find("ABC").is_ok(), "and the way back");
        assert!(ui.find("q").is_err(), "the letters are gone");
    }

    Ok(())
}

/// An open network connects on the tap: there is nothing to ask for.
#[test]
fn an_open_network_connects_without_a_prompt() {
    let (_, mut settings) = scanned();

    // The simulator's open network is the fourth by signal strength.
    assert_eq!(settings.access_points()[3].ssid, "OpenGuest");

    settings.update(Message::WifiSelect(3));

    assert!(settings.password_prompt().is_none(), "no prompt");
    assert_eq!(settings.wifi_status().state, WifiState::Connected);
    assert_eq!(settings.wifi_status().ssid, "OpenGuest");

    let mut ui = interface(&settings);
    assert!(ui.find("Disconnect").is_ok(), "the page offers the way out");
}

/// The connection card is the radio's numbers, and disconnecting leaves the network.
#[test]
fn the_connection_card_is_the_boards_own_numbers() -> Result<(), iced_test::Error> {
    let (_, mut settings) = scanned();

    // Before anything is connected, the page must not be showing an address.
    {
        let mut ui = interface(&settings);
        assert!(ui.find("0.0.0.0").is_err(), "nothing is connected yet");
        assert!(ui.find("Disconnect").is_err(), "and there is no way out");
    }

    settings.update(Message::WifiSelect(3));

    let address = settings.wifi_status().ip.clone();

    {
        let mut ui = interface(&settings);
        assert!(ui.find(address.as_str()).is_ok(), "the address the radio reported: {address}");
        assert!(ui.find("Disconnect").is_ok(), "and the way out");
    }

    settings.update(Message::WifiDisconnect);

    assert_eq!(settings.wifi_status().state, WifiState::Disconnected);

    Ok(())
}

/// The switch is the radio's, not the page's: turning it off empties the list.
#[test]
fn the_switch_turns_the_radio_off() -> Result<(), iced_test::Error> {
    let (board, mut settings) = scanned();

    settings.update(Message::ToggleWifi);

    assert!(!settings.wifi_enabled());
    assert!(!board.wifi().is_enabled(), "the radio itself is off");
    assert!(settings.access_points().is_empty(), "and nothing to list");

    let mut ui = interface(&settings);
    assert!(ui.find("Wi-Fi is off").is_ok());

    Ok(())
}

/// The prompt is a modal: a press that lands where the list is never reaches the list.
///
/// This is the one thing about stacking a prompt over a page that has to be asserted rather than
/// assumed — a layer that does not capture is a layer the page answers through, and the page behind
/// this prompt is a screen of buttons. What the press *does* instead is the backdrop's business
/// (it cancels); what matters is that the list did not act.
#[test]
fn the_prompt_takes_the_finger_from_the_list() -> Result<(), iced_test::Error> {
    let (_, mut settings) = scanned();

    settings.update(Message::WifiSelect(0));
    assert!(settings.password_prompt().is_some(), "the prompt is up");

    // `OpenGuest` is a row of the list behind the prompt, and pressing it would connect.
    let mut ui = interface(&settings);
    let _ = ui.click("OpenGuest")?;

    for message in ui.into_messages() {
        settings.update(message);
    }

    assert_ne!(
        settings.wifi_status().ssid, "OpenGuest",
        "the press went to the list behind the prompt"
    );
    assert_ne!(settings.wifi_status().state, WifiState::Connected);

    Ok(())
}

/// The hardware back button closes the prompt before it leaves the page.
///
/// This is the launcher's path, not `update`'s: the launcher intercepts `Message::Back` and calls
/// `go_back` itself, so a prompt that only closed inside `update` would never close on the board.
#[test]
fn the_hardware_back_closes_the_prompt_first() {
    let (_, mut settings) = scanned();

    settings.update(Message::WifiSelect(0));
    assert!(settings.password_prompt().is_some());

    assert!(settings.go_back(), "the press is consumed by the prompt");
    assert!(settings.password_prompt().is_none());
    assert_eq!(
        settings.section(),
        SettingsSection::Wifi,
        "and the page is still up"
    );

    assert!(settings.go_back(), "the next press leaves the page");
    assert_eq!(settings.section(), SettingsSection::Main);
}

/// The prompt can show what was typed, and go back to dots.
#[test]
fn the_prompt_can_show_what_was_typed() -> Result<(), iced_test::Error> {
    let (_, mut settings) = scanned();

    settings.update(Message::WifiSelect(0));

    for character in "hunter2".chars() {
        settings.update(Message::WifiKey(KeyAction::Char(character)));
    }

    {
        let mut ui = interface(&settings);
        assert!(ui.find("hunter2").is_err(), "dots, not the password");
        assert!(ui.find("Show password").is_ok(), "and the way to reveal it");
    }

    settings.update(Message::WifiReveal);
    assert!(settings.password_revealed());

    {
        let mut ui = interface(&settings);
        assert!(ui.find("hunter2").is_ok(), "the password itself");
        assert!(ui.find("Hide").is_ok(), "and the way back");
    }

    settings.update(Message::WifiReveal);
    assert!(!settings.password_revealed(), "hidden again");

    // And a prompt opened later starts hidden: a password left in the clear is not a state to
    // inherit from the last one.
    settings.update(Message::WifiReveal);
    settings.update(Message::WifiCancel);
    settings.update(Message::WifiSelect(1));

    assert!(!settings.password_revealed());
    assert!(settings.password().is_empty(), "and empty");

    Ok(())
}

/// The Wi-Fi page asks for frames only while it is waiting on the radio.
///
/// This is the regression test for what a real board showed: the app subscribed to every frame to
/// read the radio four times a second, and the host runs the view and the damage diff for every
/// message a subscription produces — so an idle password prompt printed a `[phase]` line a
/// millisecond, with `glyphs 0/0` and `clear 0.00`, for as long as it was open.
#[test]
fn the_frames_stop_when_the_radio_is_no_longer_waited_on() {
    let (board, mut settings) = wifi_app();

    assert!(
        !settings.is_waiting_on_wifi(),
        "the page is not even open, so nothing is waiting"
    );

    settings.update(Message::Open(SettingsSection::Wifi));
    assert!(settings.is_waiting_on_wifi(), "a scan is in flight");

    for _ in 0..3 {
        board.tick();
    }

    assert!(
        settings.is_waiting_on_wifi(),
        "the radio is done but the page has not read the results yet"
    );

    frame(&mut settings, Duration::from_secs(1));

    assert!(
        !settings.is_waiting_on_wifi(),
        "the page has what it was waiting for, so the frames must stop"
    );

    // Merely being *open* does not re-arm it — that is the whole point.
    frame(&mut settings, Duration::from_secs(10));
    assert!(!settings.is_waiting_on_wifi());

    // A radio that is off has nothing to wait for either.
    settings.update(Message::ToggleWifi);
    assert!(!settings.is_waiting_on_wifi());
}

#[test]
fn the_theme_can_be_switched() {
    let mut settings = english();
    assert_eq!(settings.theme_mode(), ThemeMode::Dark);

    settings.update(Message::SetTheme(ThemeMode::Light));
    assert_eq!(settings.theme_mode(), ThemeMode::Light);

    settings.update(Message::SetTheme(ThemeMode::Dark));
    assert_eq!(settings.theme_mode(), ThemeMode::Dark);
}

#[test]
fn the_font_tier_can_be_cycled() {
    let mut settings = english();
    assert_eq!(settings.font_tier(), FontSizeTier::Standard);
    assert_eq!(settings.font_tier().base_size(), 18.0);

    settings.update(Message::CycleFontTier);
    assert_eq!(settings.font_tier(), FontSizeTier::Large);
    assert_eq!(settings.font_tier().base_size(), 21.0);

    settings.update(Message::CycleFontTier);
    assert_eq!(settings.font_tier(), FontSizeTier::Standard);
    assert_eq!(settings.font_tier().base_size(), 18.0);
}
