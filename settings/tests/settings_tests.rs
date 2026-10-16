//! The settings app's own tests: what the pages say, through the widget tree.
//!
//! Nothing here names a platform layer, reads a panel or draws a pixel. `iced_test`'s simulator
//! builds the *real* widget tree, finds a control by the text it is drawn with, presses it, and
//! hands back the message that produced. What the app costs the board — frames, damage, a finger
//! on the panel, a scroll — is asserted in `firmware/panel-tests`, where the panel is.

// These tests apply messages by hand, and a test has no loop to return the `Task` an update
// produces to. What that task does — send the page body back to the top — is the loop's business,
// not the state's. See `Settings::update`.
#![allow(unused_must_use)]

use std::sync::Arc;
use std::time::Duration;

use iced::time::Instant;
use iced::Size;
use iced_test::Simulator;
use pomelo_hal::{ApInfo, Board, ScanState, WifiState, WifiStatus};
use pomelo_material_symbols::Icon;
use pomelo_widgets::touch_keyboard::{KeyAction, KeyboardMode};

use settings::{FontSizeTier, Language, Message, Settings, SettingsSection, SleepTimeout, ThemeMode};

/// The reads these tests make, in the names they were written with.
///
/// The app exposes one door — [`Settings::wifi`] — and the reads that used to sit on `Settings` are
/// the Wi-Fi page's own. Rather than rewrite two dozen call sites, the tests say what they need
/// here; that list is also the honest answer to "which reads do these tests use".
trait WifiReads {
    fn wifi_enabled(&self) -> bool;
    fn access_points(&self) -> &[ApInfo];
    fn wifi_status(&self) -> &WifiStatus;
    fn password_prompt(&self) -> Option<&ApInfo>;
    fn password(&self) -> &str;
    fn password_revealed(&self) -> bool;
    fn language(&self) -> Language;
    fn theme_mode(&self) -> ThemeMode;
    fn font_tier(&self) -> FontSizeTier;
}

impl WifiReads for Settings {
    fn wifi_enabled(&self) -> bool {
        self.wifi().enabled()
    }

    fn access_points(&self) -> &[ApInfo] {
        self.wifi().access_points()
    }

    fn wifi_status(&self) -> &WifiStatus {
        self.wifi().status()
    }

    fn password_prompt(&self) -> Option<&ApInfo> {
        self.wifi().prompt()
    }

    fn password(&self) -> &str {
        self.wifi().password()
    }

    fn password_revealed(&self) -> bool {
        self.wifi().revealed()
    }

    fn language(&self) -> Language {
        self.preferences().language
    }

    fn theme_mode(&self) -> ThemeMode {
        self.preferences().theme
    }

    fn font_tier(&self) -> FontSizeTier {
        self.preferences().font_tier
    }
}

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
const PAGES: [(&str, SettingsSection, &str); 9] = [
    ("Wi-Fi", SettingsSection::Wifi, "Wi-Fi"),
    ("Memory (RAM)", SettingsSection::Memory, "Memory"),
    ("Storage", SettingsSection::Storage, "Storage"),
    ("Battery", SettingsSection::Battery, "Battery & Power"),
    ("Applications", SettingsSection::Apps, "Applications"),
    (
        "Data Backup & Restore",
        SettingsSection::Backup,
        "Data Backup & Restore",
    ),
    ("System", SettingsSection::SystemInfo, "About"),
    ("Theme & Display", SettingsSection::Theme, "Display & Theme"),
    ("Date & Time", SettingsSection::Time, "Date & Time"),
];

/// A screen tall enough for the whole main list, for a test that presses any row in it.
///
/// Deliberately generous, and more than twice the design panel's 480: the list is longer than the
/// panel and scrolls on the device, but the simulator has no scroll, so a press needs a screen that
/// shows all of it. It only has to be *taller than the list*, with room for the next few rounds of
/// tuning — a tight fit turns every metric change into a test failure, which is how it went from 800
/// to here when the list's top gap and its gaps between cards both grew.
const TALL: f32 = 1200.0;

/// `settings`' interface on the design panel.
fn interface(settings: &Settings) -> Simulator<'_, Message> {
    screen(settings, PANEL)
}

/// `settings`' interface on a screen `height` logical pixels tall.
///
/// The layout is flex, so it does not notice the difference; only what fits in the viewport does.
fn screen(settings: &Settings, height: f32) -> Simulator<'_, Message> {
    Simulator::with_size(
        iced::Settings {
            // By *generic* family: the tester's default is "Fira Sans", which only exists when
            // iced's `fira-sans` feature is on — 441 KiB of glyphs this firmware has no room for.
            default_font: iced::Font::MONOSPACE,
            ..iced::Settings::default()
        },
        Size::new(PANEL, height),
        settings.view(),
    )
}

/// Presses `label` in a freshly built tree of `settings`' current state, and applies what came back.
///
/// On [`TALL`] and not the design panel: the main list is longer than the panel and scrolls, and the
/// simulator has no scroll — `find` only sees what is visible — so a press asks for a screen that
/// shows the whole list. What the panel's own height is for is the *layout*, which is what
/// `interface` checks.
fn press(settings: &mut Settings, label: &str) -> Result<(), iced_test::Error> {
    let mut ui = screen(settings, TALL);
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
        settings.update(Message::Back);
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
fn back_returns_to_the_main_list() {
    let mut settings = english();

    settings.update(Message::Open(SettingsSection::Wifi));
    assert_eq!(settings.section(), SettingsSection::Wifi);

    settings.update(Message::Back);

    assert_eq!(settings.section(), SettingsSection::Main);
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

/// Every page says where it is, and can go back via back message.
#[test]
fn every_page_is_titled_and_can_go_back() {
    let mut settings = english();

    for (row, section, title) in PAGES {
        settings.update(Message::Open(section));

        {
            let mut ui = interface(&settings);

            assert!(ui.find(title).is_ok(), "the {row} page is not titled");
            assert!(
                ui.find("Back").is_err(),
                "the {row} page has no on-screen Back button"
            );
        }
        assert!(
            settings.go_back().is_some(),
            "the {row} page can go back to the list"
        );
        assert_eq!(settings.section(), SettingsSection::Main);
    }

    settings.update(Message::Open(SettingsSection::Main));

    {
        let mut ui = interface(&settings);
        assert!(ui.find("Settings").is_ok(), "the list is titled");
        assert!(ui.find("Back").is_err());
    }
    assert!(
        settings.go_back().is_none(),
        "the list cannot go back further"
    );
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

/// The password is written down when the connection comes up — and not before.
#[test]
fn the_password_is_written_down_once_it_works() {
    let (board, mut settings) = scanned();

    assert!(
        board.wifi().saved().is_none(),
        "a board that has never been on a network remembers nothing"
    );

    // The prompt opens empty, and an empty password is refused before the radio is asked.
    settings.update(Message::WifiSelect(0));
    settings.update(Message::WifiConnect);
    assert!(
        board.wifi().saved().is_none(),
        "an attempt the radio never saw is not written down"
    );

    for character in "hunter2".chars() {
        settings.update(Message::WifiKey(KeyAction::Char(character)));
    }
    settings.update(Message::WifiConnect);

    assert_eq!(settings.wifi_status().state, WifiState::Connected);

    let saved = board.wifi().saved().expect("the network that worked");
    assert_eq!(saved.ssid, "ESP-Rust-5G");
    assert_eq!(saved.password, "hunter2");
    assert!(saved.enabled, "the radio was on when it connected");
    assert!(saved.autoconnect, "and it is wanted again at boot");
}

/// The switch is a setting too, and it is written beside the network it belongs to.
#[test]
fn the_switch_is_written_beside_a_remembered_network() {
    let (board, mut settings) = scanned();

    settings.update(Message::WifiSelect(0));
    for character in "hunter2".chars() {
        settings.update(Message::WifiKey(KeyAction::Char(character)));
    }
    settings.update(Message::WifiConnect);

    let initial = board.wifi().saved().expect("saved after connect");
    assert!(initial.enabled, "on, having connected");
    assert_eq!(initial.ssid, "ESP-Rust-5G");
    assert_eq!(initial.password, "hunter2");

    settings.update(Message::ToggleWifi);
    let after_toggle_off = board.wifi().saved().expect("persisted after toggle off");
    assert!(!after_toggle_off.enabled, "off, and written down");
    assert_eq!(after_toggle_off.ssid, "ESP-Rust-5G", "SSID must be preserved when turning off");
    assert_eq!(after_toggle_off.password, "hunter2", "Password must be preserved when turning off");

    settings.update(Message::ToggleWifi);
    let after_toggle_on = board.wifi().saved().expect("persisted after toggle on");
    assert!(after_toggle_on.enabled, "and on again");
    assert_eq!(after_toggle_on.ssid, "ESP-Rust-5G", "SSID must be preserved when turning back on");
    assert_eq!(after_toggle_on.password, "hunter2", "Password must be preserved when turning back on");
}

/// The switch is persisted even if no network has ever been connected.
#[test]
fn the_switch_is_written_down_even_without_network() {
    let board = Arc::new(Board::simulated());
    let mut settings = Settings::new(Arc::clone(&board));
    settings.update(Message::Open(SettingsSection::Wifi));

    assert!(board.wifi().saved().is_none(), "initially no config file");

    settings.update(Message::ToggleWifi);
    let saved = board.wifi().saved().expect("persisted switch choice");
    assert_eq!(saved.enabled, settings.wifi_enabled());
    assert!(!saved.has_network());
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
    //
    // On [`TALL`], like [`press`]: the Wi-Fi page's own head puts that row below the design panel's
    // edge, and the simulator has no scroll — what is being asserted is the `stack`, not the fit.
    let mut ui = screen(&settings, TALL);
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

/// Every sub-page's head carries a way back, and it is the press the board's back key makes.
///
/// A page that can only be left by a hardware key is a page with no way out on a board that has
/// none, so the button is not decoration — it is the exit. It is found the way a `find` finds
/// anything, by the text it is drawn with, and the text of an icon button is its glyph.
#[test]
fn a_sub_page_head_carries_the_way_back() -> Result<(), iced_test::Error> {
    let mut settings = english();

    press(&mut settings, "Memory (RAM)")?;
    assert_eq!(settings.section(), SettingsSection::Memory);

    press(&mut settings, Icon::ARROW_BACK.glyph())?;
    assert_eq!(
        settings.section(),
        SettingsSection::Main,
        "the head's button goes back"
    );

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

    assert!(
        settings.go_back().is_some(),
        "the press is consumed by the prompt"
    );
    assert!(settings.password_prompt().is_none());
    assert_eq!(
        settings.section(),
        SettingsSection::Wifi,
        "and the page is still up"
    );

    assert!(
        settings.go_back().is_some(),
        "the next press leaves the page"
    );
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
    assert_eq!(settings.font_tier().base_size(), 24.0);

    settings.update(Message::CycleFontTier);
    assert_eq!(settings.font_tier(), FontSizeTier::Large);
    assert_eq!(settings.font_tier().base_size(), 30.0);

    settings.update(Message::CycleFontTier);
    assert_eq!(settings.font_tier(), FontSizeTier::ExtraSmall);
    assert_eq!(settings.font_tier().base_size(), 18.0);

    settings.update(Message::CycleFontTier);
    assert_eq!(settings.font_tier(), FontSizeTier::Small);
    assert_eq!(settings.font_tier().base_size(), 20.0);

    settings.update(Message::CycleFontTier);
    assert_eq!(settings.font_tier(), FontSizeTier::Standard);
    assert_eq!(settings.font_tier().base_size(), 24.0);
}

#[test]
fn the_sleep_timeout_can_be_cycled() {
    let mut settings = english();
    assert_eq!(settings.preferences().sleep_timeout, SleepTimeout::Never);

    settings.update(Message::CycleSleepTimeout);
    assert_eq!(settings.preferences().sleep_timeout, SleepTimeout::FifteenSeconds);

    settings.update(Message::CycleSleepTimeout);
    assert_eq!(settings.preferences().sleep_timeout, SleepTimeout::ThirtySeconds);

    settings.update(Message::CycleSleepTimeout);
    assert_eq!(settings.preferences().sleep_timeout, SleepTimeout::OneMinute);

    settings.update(Message::CycleSleepTimeout);
    assert_eq!(settings.preferences().sleep_timeout, SleepTimeout::FiveMinutes);

    settings.update(Message::CycleSleepTimeout);
    assert_eq!(settings.preferences().sleep_timeout, SleepTimeout::Never);
}

#[test]
fn theme_page_shows_auto_sleep_option_and_dialog_selection() -> Result<(), iced_test::Error> {
    let mut settings = Settings::new(simulated());
    settings.update(Message::Open(SettingsSection::Theme));

    // Initially displays "自动息屏" and default "从不"
    {
        let mut ui = iced_test::Simulator::new(settings.view());
        assert!(ui.find("自动息屏").is_ok());
        assert!(ui.find("从不").is_ok());
    }

    // Tapping row opens sleep timeout options modal dialog
    settings.update(Message::PromptSleepTimeout);
    {
        let mut ui = iced_test::Simulator::new(settings.view());
        assert!(ui.find("从不").is_ok());
        assert!(ui.find("15 秒").is_ok());
        assert!(ui.find("30 秒").is_ok());
        assert!(ui.find("1 分钟").is_ok());
        assert!(ui.find("5 分钟").is_ok());
        assert!(ui.find("取消").is_ok());
    }

    // Dismiss with Back key
    settings.update(Message::Back);
    {
        let mut ui = iced_test::Simulator::new(settings.view());
        assert!(ui.find("取消").is_err(), "dialog is closed");
    }

    // Open dialog again and select 1 minute
    settings.update(Message::PromptSleepTimeout);
    settings.update(Message::SetSleepTimeout(SleepTimeout::OneMinute));
    {
        let mut ui = iced_test::Simulator::new(settings.view());
        assert_eq!(settings.preferences().sleep_timeout, SleepTimeout::OneMinute);
        assert!(ui.find("1 分钟").is_ok(), "theme page tile shows selected option");
    }

    Ok(())
}

#[test]
fn storage_page_shows_internal_storage_and_format_dialog() -> Result<(), iced_test::Error> {
    let mut settings = Settings::new(simulated());
    settings.update(Message::Open(SettingsSection::Storage));

    // By default in Chinese:
    {
        let mut ui = screen(&settings, TALL);
        assert!(ui.find("内部存储").is_ok(), "shows 内部存储 title");
        assert!(ui.find("/internal").is_ok(), "shows /internal mount point");
        assert!(ui.find("LittleFS").is_ok(), "shows LittleFS filesystem");
        assert!(ui.find("磨损均衡").is_err(), "磨损均衡 is removed");
    }

    // When format is prompted:
    settings.update(Message::PromptFormatSd);
    {
        let mut ui = screen(&settings, TALL);
        assert!(ui.find("警告：格式化将清空此卡所有数据！是否继续？").is_ok(), "shows format warning prompt");
        assert!(ui.find("格式化").is_ok(), "shows 格式化 confirm button");
        assert!(ui.find("取消").is_ok(), "shows 取消 cancel button");
    }

    // Back cancels format prompt:
    settings.update(Message::Back);
    {
        let mut ui = screen(&settings, TALL);
        assert!(ui.find("警告：格式化将清空此卡所有数据！是否继续？").is_err(), "dialog is closed");
        assert_eq!(settings.section(), SettingsSection::Storage, "still on storage page");
    }

    Ok(())
}

#[test]
fn backup_page_dialog_flow() -> Result<(), iced_test::Error> {
    let mut settings = Settings::new(simulated());
    settings.update(Message::Open(SettingsSection::Backup));

    // Shows backup and restore options
    {
        let mut ui = screen(&settings, TALL);
        assert!(ui.find("数据备份与恢复").is_ok(), "shows 数据备份与恢复 card");
        assert!(ui.find("备份应用数据").is_ok(), "shows 备份应用数据 option");
        assert!(ui.find("恢复应用数据").is_ok(), "shows 恢复应用数据 option");
    }

    // Prompt backup modal
    settings.update(Message::PromptBackupAppData);
    {
        let mut ui = screen(&settings, TALL);
        assert!(
            ui.find("将把 /internal/AppData 备份至 SD 卡。如果 SD 卡已有旧备份，将会被覆盖。是否继续？").is_ok(),
            "shows backup warning"
        );
        assert!(ui.find("备份").is_ok(), "shows 备份 confirm button");
        assert!(ui.find("取消").is_ok(), "shows 取消 button");
    }

    // Cancel backup modal via Back
    settings.update(Message::Back);
    {
        let mut ui = screen(&settings, TALL);
        assert!(
            ui.find("将把 /internal/AppData 备份至 SD 卡。如果 SD 卡已有旧备份，将会被覆盖。是否继续？").is_err(),
            "backup modal closed"
        );
    }

    // Prompt restore modal
    settings.update(Message::PromptRestoreAppData);
    {
        let mut ui = screen(&settings, TALL);
        assert!(
            ui.find("警告：恢复将覆盖本机所有应用数据与配置，且不可撤销！是否继续？").is_ok(),
            "shows restore warning"
        );
        assert!(ui.find("恢复").is_ok(), "shows 恢复 confirm button");
        assert!(ui.find("取消").is_ok(), "shows 取消 button");
    }

    // Cancel restore modal via Cancel button
    settings.update(Message::CancelRestoreAppData);
    {
        let mut ui = screen(&settings, TALL);
        assert!(
            ui.find("警告：恢复将覆盖本机所有应用数据与配置，且不可撤销！是否继续？").is_err(),
            "restore modal closed"
        );
    }

    Ok(())
}

#[test]
fn applications_and_daily_schedule_settings_flow() {
    let board = Arc::new(Board::simulated());
    let mut settings = Settings::new(board.clone());

    // 1. Initial page is Main
    assert_eq!(settings.section(), SettingsSection::Main);

    // 2. Open Applications section
    settings.update(Message::Open(SettingsSection::Apps));
    assert_eq!(settings.section(), SettingsSection::Apps);

    // 3. Open Daily Schedule settings from Apps
    settings.update(Message::Open(SettingsSection::AppDailySchedule));
    assert_eq!(settings.section(), SettingsSection::AppDailySchedule);

    // 4. Test import from clipboard
    settings.update(Message::DailyScheduleClipboardImported(Some(
        "webcal://example.com/calendar/test/reach.ics".to_string(),
    )));
    assert_eq!(
        settings.daily_schedule_config().subscription_url.as_deref(),
        Some("https://example.com/calendar/test/reach.ics")
    );
    assert!(settings.schedule_sync_notice().is_some());
    settings.update(Message::DailyScheduleDismissNotice);
    assert!(settings.schedule_sync_notice().is_none());

    // 5. Test real async sync flow: start HTTP fetch and poll frames
    settings.update(Message::DailyScheduleTestSync);
    assert!(settings.is_testing_schedule_sync());

    // Simulated HTTP takes 2 ticks to complete
    board.tick();
    settings.update(Message::DailyScheduleSyncFrame);
    assert!(settings.is_testing_schedule_sync());

    board.tick();
    settings.update(Message::DailyScheduleSyncFrame);
    assert!(!settings.is_testing_schedule_sync());
    assert!(settings.schedule_sync_notice().is_some());
    let notice = settings.schedule_sync_notice().unwrap();
    assert!(notice.1.contains("HTTP 200"));
    settings.update(Message::DailyScheduleDismissNotice);
    assert!(settings.schedule_sync_notice().is_none());

    // 6. Test back navigation: Daily Schedule -> Apps -> Main
    settings.update(Message::Back);
    assert_eq!(settings.section(), SettingsSection::Apps);

    settings.update(Message::Back);
    assert_eq!(settings.section(), SettingsSection::Main);

    // 7. Re-open Daily Schedule and test Clear flow
    settings.update(Message::Open(SettingsSection::AppDailySchedule));
    settings.update(Message::DailySchedulePromptClear);
    settings.update(Message::DailyScheduleConfirmClear);
    assert_eq!(settings.daily_schedule_config().subscription_url, None);
}

#[test]
fn scroll_position_is_remembered_and_restored_on_back() {
    use iced::widget::scrollable::AbsoluteOffset;

    let mut settings = english();

    // 1. Initial offset at Main is 0.0
    assert_eq!(settings.section(), SettingsSection::Main);
    assert_eq!(settings.scroll_offset(SettingsSection::Main), 0.0);

    // 2. User scrolls Main down to 150.0 px
    settings.update(Message::Scrolled(AbsoluteOffset { x: 0.0, y: 150.0 }));
    assert_eq!(settings.scroll_offset(SettingsSection::Main), 150.0);

    // 3. User taps Storage: Storage opens, its offset is 0.0, Main's offset remains 150.0
    settings.update(Message::Open(SettingsSection::Storage));
    assert_eq!(settings.section(), SettingsSection::Storage);
    assert_eq!(settings.scroll_offset(SettingsSection::Storage), 0.0);
    assert_eq!(settings.scroll_offset(SettingsSection::Main), 150.0);

    // 4. User scrolls Storage to 60.0 px
    settings.update(Message::Scrolled(AbsoluteOffset { x: 0.0, y: 60.0 }));
    assert_eq!(settings.scroll_offset(SettingsSection::Storage), 60.0);

    // 5. User clicks Back: returns to Main, Main's offset is restored to 150.0 px!
    assert!(settings.go_back().is_some());
    assert_eq!(settings.section(), SettingsSection::Main);
    assert_eq!(settings.scroll_offset(SettingsSection::Main), 150.0);

    // 6. Multi-level hierarchy test: Main (150px) -> Apps (scroll 75px) -> DailySchedule (scroll 30px)
    settings.update(Message::Open(SettingsSection::Apps));
    assert_eq!(settings.section(), SettingsSection::Apps);
    assert_eq!(settings.scroll_offset(SettingsSection::Apps), 0.0);

    settings.update(Message::Scrolled(AbsoluteOffset { x: 0.0, y: 75.0 }));
    assert_eq!(settings.scroll_offset(SettingsSection::Apps), 75.0);

    settings.update(Message::Open(SettingsSection::AppDailySchedule));
    assert_eq!(settings.section(), SettingsSection::AppDailySchedule);
    assert_eq!(settings.scroll_offset(SettingsSection::AppDailySchedule), 0.0);

    settings.update(Message::Scrolled(AbsoluteOffset { x: 0.0, y: 30.0 }));
    assert_eq!(settings.scroll_offset(SettingsSection::AppDailySchedule), 30.0);

    // Back from DailySchedule -> Apps: Apps offset is restored to 75.0 px!
    assert!(settings.go_back().is_some());
    assert_eq!(settings.section(), SettingsSection::Apps);
    assert_eq!(settings.scroll_offset(SettingsSection::Apps), 75.0);

    // Back from Apps -> Main: Main offset is restored to 150.0 px!
    assert!(settings.go_back().is_some());
    assert_eq!(settings.section(), SettingsSection::Main);
    assert_eq!(settings.scroll_offset(SettingsSection::Main), 150.0);
}
