//! The settings app, built from iced widgets — **a standard iced program**.
//!
//! A port of the settings app this replaces: the same section list, the same grouped cards and
//! the same switches, rebuilt from `iced`'s own widgets.
//!
//! Two decisions in the port are deliberate and worth stating before the code.
//!
//! # The labels are bilingual
//!
//! The interface speaks Simplified Chinese and English, and the language is part of the app's
//! state ([`Language`]): a switch is a state change, and iced's own diff repaints only the rows
//! whose text actually changed. [`i18n`] says why this is a static table and an exhaustive `match`
//! rather than `rust-i18n` or `fluent`.
//!
//! Chinese is the default, because it is the original: this app is a port of a Chinese settings
//! app, and its labels were **only** ASCII because the old 16 KiB Latin subset could not draw a
//! CJK glyph at all. The font carries 4008 Chinese characters now (see
//! `vendor/iced-pomelo-winit/fonts/README.md`), so the originals are back, with the English port
//! kept as the second language.
//!
//! What is translated is the interface: sections, row labels, switches, buttons, footnotes, and
//! the words in values (`未连接`, `已用`). What is not translated is **data** — a network's name, an
//! IP, a MAC, byte counts, a model number, a time. Those are facts about this machine, and the
//! pages below still write them as literals.
//!
//! # The board is injected, and the Wi-Fi page is why
//!
//! The app is handed an `Arc<pomelo_hal::Board>` by whoever builds it — the launcher passes the one
//! it already shares with every app it hosts, and `main.rs` builds the desktop simulator — which is
//! the same shape `music-player` uses. The port deliberately took no board, because every page it
//! had was a *reading*: a battery percentage is served by [`Settings::set_battery`], which the
//! platform pushes into, and a pushed value is testable with nothing but a widget tree.
//!
//! Wi-Fi is not a reading. Scanning and connecting are a conversation — the page asks the radio
//! (`Board::wifi().scan_start()`), waits, and asks again — and a push-only channel would put that
//! conversation in the *launcher*, which is the one thing hosting an app as a widget exists to
//! avoid. So the board is here, and so is the polling ([`Settings::subscription`]).
//!
//! The battery page still takes its number from the platform: a reading the platform owns stays
//! pushed, and nothing here reads `board.power()`.
//!
//! # Scrolling
//!
//! Every page is a 44px navigation bar plus an `iced_widget::scrollable` body, so a page taller
//! than the panel scrolls. Switching pages opens the new page at the top; that reset is the
//! whole reason [`page`] exists, and it is explained there.

mod i18n;
mod page;
pub mod style;

use std::sync::Arc;
use std::time::Duration;

use iced::theme::Palette;
use iced::time::Instant;
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{button, container, mouse_area, opaque, scrollable, stack, text, Column, Row, Space};
use iced::{Alignment, Border, Color, Element, Length, Padding, Renderer, Shadow, Subscription, Theme};
use pomelo_hal::{ApInfo, Board, ScanState, WifiState, WifiStatus};

pub use pomelo_widgets::{FontSizeTier, Language, SystemPreferences, ThemeMode};
pub use i18n::{Key, LanguageExt};
use page::{
    section_page, BatteryTag, MainTag, MemoryTag, StorageTag, SystemTag, ThemeTag, TimeTag, WifiTag,
};
use pomelo_widgets::touch_keyboard::{self, KeyAction, KeyboardMode};

pub use style::SCREEN;

/// The element type every builder below speaks.
type UI<'a> = Element<'a, Message>;

/// Which page is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsSection {
    /// The grouped list of sections.
    Main,
    /// Wi-Fi: a switch and the current network.
    Wifi,
    /// Memory: a usage bar and the heap numbers.
    Memory,
    /// Storage: a usage bar and the LittleFS numbers.
    Storage,
    /// Battery: the reading the platform last pushed.
    Battery,
    /// System information.
    SystemInfo,
    /// Display and theme.
    Theme,
    /// Date and time.
    Time,
}

/// What the app reacts to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Message {
    /// A row of the main list was pressed.
    Open(SettingsSection),
    /// The back button (or the hardware back key) was pressed.
    Back,
    /// The Wi-Fi switch was flipped.
    ToggleWifi,
    /// The 24-hour-time switch was flipped.
    Toggle24Hour,
    /// The language row was pressed, and the interface is now in `Language`.
    SetLanguage(Language),
    /// Set the appearance theme mode.
    SetTheme(ThemeMode),
    /// Cycle to the next font size tier.
    CycleFontTier,

    /// The Wi-Fi page asked for a fresh scan: the page was opened, or the scan row was pressed.
    WifiScan,
    /// A frame arrived while the Wi-Fi page was open — the cue to read the radio.
    ///
    /// The instant is the frame's, so the page can read the radio a few times a second instead of
    /// once per frame: iced's `time::every` needs a tokio or smol runtime and this stack has
    /// neither, so a frame *is* the clock here (see [`Settings::subscription`]).
    WifiFrame(Instant),
    /// A network in the list was pressed.
    WifiSelect(usize),
    /// A key of the password prompt's keyboard.
    WifiKey(KeyAction),
    /// The prompt's connect button — or its return key.
    WifiConnect,
    /// The prompt's cancel button.
    WifiCancel,
    /// The prompt's show/hide button: draw the password in the clear, or as dots again.
    WifiReveal,
    /// The connection card's disconnect row.
    WifiDisconnect,
}

/// The battery reading the battery page shows.
///
/// The platform owns the real number and pushes it in with [`Settings::set_battery`]; the values
/// here are what the original read from a bench board. There is deliberately no
/// `pomelo_hal` dependency — see the crate documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Battery {
    /// Charge, 0 to 100.
    pub percent: u8,
    /// Whether a charger is connected.
    pub charging: bool,
    /// Pack voltage in millivolts.
    pub voltage_mv: u16,
}

impl Default for Battery {
    fn default() -> Self {
        Self {
            percent: 88,
            charging: true,
            voltage_mv: 4_100,
        }
    }
}

/// The settings app.
pub struct Settings {
    section: SettingsSection,
    wifi_enabled: bool,
    is_24h_format: bool,
    battery: Battery,
    /// The system preferences (language, theme, font size tier).
    preferences: SystemPreferences,
    /// The board, for the one page that has a conversation with hardware instead of a reading.
    /// See the crate documentation.
    board: Arc<Board>,
    /// The Wi-Fi page: the radio's own state, the list, and the password prompt.
    wifi: Wifi,
}

/// How often an open Wi-Fi page reads the radio.
///
/// The page is told about every frame while it is up (see [`Settings::subscription`]) and reads the
/// board this often. A read is a mutex and a few atomics; four times a second is quicker than a
/// finger and far slower than the board's own loop.
const WIFI_READ_EVERY: Duration = Duration::from_millis(250);

/// The Wi-Fi page's own state.
///
/// Everything here is what the radio last said, copied — not a handle on the radio itself. The page
/// draws from this, and [`Settings::read_wifi`] is what refreshes it.
struct Wifi {
    /// What the last completed scan found, best signal first.
    access_points: Vec<ApInfo>,
    /// Where the radio is: idle, scanning, or finished.
    scan: ScanState,
    /// The connection as of the last read.
    status: WifiStatus,
    /// The prompt is open for this entry of `access_points`.
    prompt: Option<usize>,
    /// What has been typed into the prompt.
    password: String,
    /// Which page the prompt's keyboard is showing.
    keyboard: KeyboardMode,
    /// Whether the prompt draws what has been typed in the clear.
    revealed: bool,
    /// A connect has been asked for and has not come back yet.
    pending: bool,
    /// The last attempt failed, so the prompt says so.
    failed: bool,
    /// When the radio was last read. `None` until the next frame after a scan was asked for.
    read_at: Option<Instant>,
    /// Whether the finished scan's results have been taken.
    ///
    /// `ScanState::Done` stays `Done` until the next scan starts, so without this the page would
    /// rebuild the list four times a second for as long as it is open.
    taken: bool,
}

impl Wifi {
    /// The page before the radio has said anything.
    fn new() -> Self {
        Self {
            access_points: Vec::new(),
            scan: ScanState::Idle,
            status: WifiStatus::default(),
            prompt: None,
            password: String::new(),
            keyboard: KeyboardMode::Lower,
            revealed: false,
            pending: false,
            failed: false,
            read_at: None,
            taken: false,
        }
    }
}

impl Settings {
    /// The app at the main list, the way the original starts, with `board` for the pages that need
    /// hardware.
    pub fn new(board: Arc<Board>) -> Self {
        let mut settings = Self {
            section: SettingsSection::Main,
            wifi_enabled: false,
            is_24h_format: true,
            battery: Battery::default(),
            preferences: SystemPreferences::default(),
            board,
            wifi: Wifi::new(),
        };

        // One read at boot, so the switch and the main list's Wi-Fi row are true before the page is
        // ever opened.
        settings.read_wifi();

        settings
    }

    /// The page that is open. Read by the host and by the tests.
    pub fn section(&self) -> SettingsSection {
        self.section
    }

    /// Whether the Wi-Fi switch is on.
    pub fn wifi_enabled(&self) -> bool {
        self.wifi_enabled
    }

    /// What the last scan found, best signal first. Read by the tests.
    pub fn access_points(&self) -> &[ApInfo] {
        &self.wifi.access_points
    }

    /// The connection as of the last read. Read by the tests.
    pub fn wifi_status(&self) -> &WifiStatus {
        &self.wifi.status
    }

    /// The network the password prompt is open for, if it is open. Read by the tests.
    pub fn password_prompt(&self) -> Option<&ApInfo> {
        self.wifi
            .prompt
            .and_then(|index| self.wifi.access_points.get(index))
    }

    /// What has been typed into the prompt. Read by the tests.
    pub fn password(&self) -> &str {
        &self.wifi.password
    }

    /// Whether the prompt is drawing the password in the clear. Read by the tests.
    pub fn password_revealed(&self) -> bool {
        self.wifi.revealed
    }

    /// Whether the clock is 24-hour.
    pub fn is_24h_format(&self) -> bool {
        self.is_24h_format
    }

    /// The battery reading the battery page shows.
    pub fn battery(&self) -> Battery {
        self.battery
    }

    /// The language the interface is drawn in.
    pub fn language(&self) -> Language {
        self.preferences.language
    }

    /// Switches to `language`.
    pub fn set_language(&mut self, language: Language) {
        self.preferences.language = language;
    }

    /// The aggregated system preferences.
    pub fn preferences(&self) -> SystemPreferences {
        self.preferences
    }

    /// Sets the aggregated system preferences.
    pub fn set_preferences(&mut self, preferences: SystemPreferences) {
        self.preferences = preferences;
    }

    /// The visual theme mode.
    pub fn theme_mode(&self) -> ThemeMode {
        self.preferences.theme
    }

    /// Sets the visual theme mode.
    pub fn set_theme_mode(&mut self, theme: ThemeMode) {
        self.preferences.theme = theme;
    }

    /// The font size tier.
    pub fn font_tier(&self) -> FontSizeTier {
        self.preferences.font_tier
    }

    /// Sets the font size tier.
    pub fn set_font_tier(&mut self, tier: FontSizeTier) {
        self.preferences.font_tier = tier;
    }

    /// Opens `section`.
    ///
    /// A change of section opens a fresh page, scrolled to the top. That is not done here: iced
    /// keeps the scroll offset in the widget tree, and the tree node is reset when the page's
    /// tag changes, which is what [`page::Page`] arranges. Setting the section it is already on
    /// changes nothing, so the offset survives — which is exactly what the original's
    /// `SettingsModel::set_section` did.
    pub fn set_section(&mut self, section: SettingsSection) {
        self.section = section;
    }

    /// Returns to the main list, reporting whether there was anywhere to return from.
    ///
    /// `false` means the back press was *not* consumed: the launcher backgrounds the app
    /// instead, which is the contract the original's `go_back` had.
    ///
    /// The password prompt is a page of its own while it is open, so back closes it before it
    /// leaves the Wi-Fi page — and it closes it *here* rather than in `update`, because the launcher
    /// intercepts `Message::Back` and calls this directly: a modal that only closed when the app was
    /// running standalone would be a modal that never closes on the board.
    pub fn go_back(&mut self) -> bool {
        if self.wifi.prompt.is_some() {
            self.cancel_prompt();
            return true;
        }

        if self.section == SettingsSection::Main {
            false
        } else {
            self.section = SettingsSection::Main;
            true
        }
    }

    /// Takes a battery reading the platform pushed in.
    pub fn set_battery(&mut self, reading: Battery) {
        self.battery = reading;
    }
}

// No `Default`: the app cannot be built without a board, and a `Default` that invented one would be
// a second composition root. `Settings::new(Arc<Board>)` is the only way in — which is what the
// firmware's root and `main.rs` both call, and what every test does with the simulator.

impl Settings {
    /// The theme: the palette and the background the platform paints behind the tree.
    pub fn theme(&self) -> Theme {
        match self.preferences.theme {
            ThemeMode::Dark => Theme::custom(
                "PomeloDark",
                Palette {
                    background: style::black(),
                    ..Palette::DARK
                },
            ),
            ThemeMode::Light => Theme::custom(
                "PomeloLight",
                Palette {
                    background: self.preferences.theme.background(),
                    ..Palette::LIGHT
                },
            ),
        }
    }

    /// Reacts to one message.
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Open(section) => self.open(section),
            Message::Back => {
                self.go_back();
            }
            Message::ToggleWifi => self.toggle_wifi(),
            Message::Toggle24Hour => self.is_24h_format = !self.is_24h_format,
            Message::SetLanguage(language) => self.set_language(language),
            Message::SetTheme(theme) => self.set_theme_mode(theme),
            Message::CycleFontTier => self.set_font_tier(self.preferences.font_tier.cycle()),
            Message::WifiScan => self.start_scan(),
            Message::WifiFrame(now) => self.poll(now),
            Message::WifiSelect(index) => self.select(index),
            Message::WifiKey(action) => self.key(action),
            Message::WifiConnect => self.connect(),
            Message::WifiCancel => self.cancel_prompt(),
            Message::WifiReveal => self.wifi.revealed = !self.wifi.revealed,
            Message::WifiDisconnect => self.disconnect(),
        }
    }

    /// Whether the app is waiting on the radio, which is what [`Settings::subscription`] is for.
    ///
    /// A scan is in flight, or a finished scan's results have not been taken yet, or a connection
    /// is being waited for. Nothing else needs the loop: the page redraws when a finger touches it.
    ///
    /// This is the fix for what a real board showed: `[phase]` lines with `glyphs 0/0` and
    /// `clear 0.00`, one per millisecond, while nothing on the page moved. A frame subscription is
    /// not a timer — every frame it delivers is a message, iced runs the view and the damage diff
    /// for every message, and on this board that is about 10 ms a thousand times a second. So the
    /// page asks for frames only while it is waiting for something it cannot get any other way.
    pub fn is_waiting_on_wifi(&self) -> bool {
        if self.section != SettingsSection::Wifi || !self.wifi_enabled {
            return false;
        }

        let waiting_for_results = self.wifi.scan == ScanState::Done && !self.wifi.taken;

        // `ScanState::Error` is deliberately not "waiting": nothing will arrive to stop it, and a
        // subscription that never ends is the bug this method exists to prevent.
        self.wifi.scan == ScanState::Scanning || waiting_for_results || self.wifi.pending
    }

    /// The app's subscriptions: the Wi-Fi page's frames, while that page is *waiting* on the radio.
    ///
    /// Nothing here is told the screen's size: the prompt's keyboard is flex, so it fills the card
    /// it is put in whatever the card is.
    ///
    /// `window::frames()` and not `iced::time::every`: that one is `cfg(any(tokio, smol, wasm32))`
    /// and this stack has no async runtime behind it — the board's loop polls its queue of futures
    /// once per frame, so a frame *is* this platform's clock.
    ///
    /// And only while [`Settings::is_waiting_on_wifi`] — a frame is not a free tick:
    /// the host runs the view and the damage diff for every message a subscription produces, which
    /// on the board is ~10 ms, and a page that asked for every frame while nothing was happening
    /// asked for that a thousand times a second.
    pub fn subscription(&self) -> Subscription<Message> {
        if self.is_waiting_on_wifi() {
            iced::window::frames().map(Message::WifiFrame)
        } else {
            Subscription::none()
        }
    }

    // =========================================================================
    // The Wi-Fi page's logic: one method per message, and every one of them asks
    // the board rather than guessing what it said.
    // =========================================================================

    /// Opens `section`, and does what opening it means.
    ///
    /// Only the Wi-Fi page has an opening move: a scan. Coming *back* to it does not re-scan — the
    /// results are still good, and the scan row is there to ask again.
    fn open(&mut self, section: SettingsSection) {
        let entering = section == SettingsSection::Wifi && self.section != section;

        self.set_section(section);

        if entering {
            self.start_scan();
        }
    }

    /// Flips the radio's switch, and asks for a scan when it goes on.
    fn toggle_wifi(&mut self) {
        let on = !self.wifi_enabled;

        if self.board.wifi().set_enabled(on).is_err() {
            return;
        }

        self.wifi_enabled = on;

        if on {
            self.start_scan();
        } else {
            // Nothing to list and nothing to connect to: the page says so.
            self.wifi.access_points.clear();
            self.wifi.scan = ScanState::Idle;
            self.wifi.taken = false;
            self.cancel_prompt();
            self.read_wifi();
        }
    }

    /// Asks the radio for a scan.
    fn start_scan(&mut self) {
        if !self.wifi_enabled {
            return;
        }

        if self.board.wifi().scan_start().is_err() {
            self.wifi.scan = ScanState::Error;
            return;
        }

        self.wifi.scan = ScanState::Scanning;
        self.wifi.taken = false;
        self.wifi.read_at = None;
    }

    /// Reads the radio once, copying what it says into the page's state.
    fn read_wifi(&mut self) {
        let wifi = self.board.wifi();

        self.wifi_enabled = wifi.is_enabled();
        self.wifi.scan = wifi.scan_state();

        // `Done` stays `Done` until the next scan starts, so the list is taken once rather than
        // rebuilt four times a second for as long as the page is open.
        if self.wifi.scan == ScanState::Done && !self.wifi.taken {
            if let Ok(found) = wifi.scan_results() {
                let mut found = found;
                // Best signal first: the network a finger is looking for is at the top.
                found.sort_by_key(|ap| std::cmp::Reverse(ap.rssi));
                self.wifi.access_points = found;
                self.wifi.taken = true;
            }
        }

        self.wifi.status = wifi.status();
    }

    /// A frame arrived while the page is open: read the radio, a few times a second.
    fn poll(&mut self, now: Instant) {
        let due = match self.wifi.read_at {
            Some(last) => now.duration_since(last) >= WIFI_READ_EVERY,
            None => true,
        };

        if !due {
            return;
        }

        self.wifi.read_at = Some(now);
        self.read_wifi();

        // A prompt that was waiting on the radio: the connection either came up, or it did not.
        if self.wifi.pending {
            match self.wifi.status.state {
                WifiState::Connected => self.connected(),
                WifiState::Disconnected => {
                    self.wifi.pending = false;
                    self.wifi.failed = true;
                }
                _ => {}
            }
        }
    }

    /// A network was pressed: connect to it, or ask for its password first.
    fn select(&mut self, index: usize) {
        let Some(ap) = self.wifi.access_points.get(index) else {
            return;
        };

        let (ssid, secure) = (ap.ssid.clone(), ap.secure);

        if !secure {
            self.request_connect(&ssid, "");
            return;
        }

        // A prompt for a network starts empty, whatever was typed for the last one — and hidden
        // again: a password left in the clear is not a state to inherit.
        self.wifi.password.clear();
        self.wifi.keyboard = KeyboardMode::Lower;
        self.wifi.revealed = false;
        self.wifi.failed = false;
        self.wifi.pending = false;
        self.wifi.prompt = Some(index);
    }

    /// A key of the prompt's keyboard.
    fn key(&mut self, action: KeyAction) {
        match action {
            KeyAction::Char(character) => self.wifi.password.push(character),
            KeyAction::Space => self.wifi.password.push(' '),
            KeyAction::Backspace => {
                self.wifi.password.pop();
            }
            KeyAction::SwitchMode(mode) => self.wifi.keyboard = mode,
            KeyAction::Enter => self.connect(),
        }
    }

    /// The prompt's connect button, and its return key.
    fn connect(&mut self) {
        let Some(index) = self.wifi.prompt else {
            return;
        };
        let Some(ssid) = self
            .wifi
            .access_points
            .get(index)
            .map(|ap| ap.ssid.clone())
        else {
            return;
        };

        let password = self.wifi.password.clone();

        self.request_connect(&ssid, &password);
    }

    /// Asks the radio to connect, and says what happened.
    fn request_connect(&mut self, ssid: &str, password: &str) {
        self.wifi.failed = false;

        if self.board.wifi().connect(ssid, password).is_err() {
            // Refused before it tried: no password on a locked network, or a network it does not
            // know. A prompt that is open is the thing that can say so; a tap on an open network
            // has nothing on screen to explain, so the list stands as it was.
            self.wifi.failed = self.wifi.prompt.is_some();
            return;
        }

        self.wifi.pending = self.wifi.prompt.is_some();
        self.read_wifi();

        if self.wifi.pending && self.wifi.status.state == WifiState::Connected {
            // A board that connects in one step (the simulator does) is done in this message.
            self.connected();
        }
    }

    /// The connection came up: the prompt has nothing left to ask.
    fn connected(&mut self) {
        self.wifi.pending = false;
        self.wifi.failed = false;
        self.wifi.prompt = None;
        self.wifi.password.clear();
        self.wifi.keyboard = KeyboardMode::Lower;
        self.wifi.revealed = false;

        self.read_wifi();
    }

    /// Closes the prompt without connecting.
    fn cancel_prompt(&mut self) {
        self.wifi.prompt = None;
        self.wifi.password.clear();
        self.wifi.keyboard = KeyboardMode::Lower;
        self.wifi.revealed = false;
        self.wifi.failed = false;
        self.wifi.pending = false;
    }

    /// Leaves the network we are on.
    fn disconnect(&mut self) {
        let _ = self.board.wifi().disconnect();

        self.read_wifi();
    }

    /// Describes the interface for the current state.
    pub fn view(&self) -> UI<'_> {
        // Each page is wrapped in its own tag, so a change of section is a new widget-tree node
        // and the scroll starts at the top. See `crate::page`.
        let page = match self.section {
            SettingsSection::Main => {
                section_page::<MainTag, _, _, _>(main_page(self.preferences, self.battery))
            }
            SettingsSection::Wifi => section_page::<WifiTag, _, _, _>(wifi_page(
                self.preferences,
                self.wifi_enabled,
                &self.wifi,
            )),
            SettingsSection::Memory => {
                section_page::<MemoryTag, _, _, _>(memory_page(self.preferences))
            }
            SettingsSection::Storage => {
                section_page::<StorageTag, _, _, _>(storage_page(self.preferences))
            }
            SettingsSection::Battery => {
                section_page::<BatteryTag, _, _, _>(battery_page(
                    self.preferences,
                    self.battery,
                ))
            }
            SettingsSection::SystemInfo => {
                section_page::<SystemTag, _, _, _>(system_page(self.preferences))
            }
            SettingsSection::Theme => {
                section_page::<ThemeTag, _, _, _>(theme_page(self.preferences))
            }
            SettingsSection::Time => section_page::<TimeTag, _, _, _>(time_page(
                self.preferences,
                self.is_24h_format,
            )),
        };

        container(page)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

// =============================================================================
// The main list
// =============================================================================

/// The main list, in `language`.
///
/// The labels are translated; the values beside them are not, because they are the machine's facts
/// (a network's name, a model number, a time) and not the interface's words. See [`i18n`].
fn main_page<'a>(preferences: SystemPreferences, battery: Battery) -> UI<'a> {
    let language = preferences.language;
    let theme = preferences.theme;
    let value = if battery.charging {
        format!("{}% {}", battery.percent, language.text(Key::Charging))
    } else {
        format!("{}%", battery.percent)
    };

    let entries = [
        (
            "W",
            style::badge(SettingsSection::Wifi),
            language.text(Key::Wifi),
            "ESP-Rust-5G".to_string(),
            Message::Open(SettingsSection::Wifi),
        ),
        (
            "M",
            style::badge(SettingsSection::Memory),
            language.text(Key::Memory),
            "8.0 MB PSRAM".to_string(),
            Message::Open(SettingsSection::Memory),
        ),
        (
            "S",
            style::badge(SettingsSection::Storage),
            language.text(Key::Storage),
            "11 MB LittleFS".to_string(),
            Message::Open(SettingsSection::Storage),
        ),
        (
            "B",
            style::badge(SettingsSection::Battery),
            language.text(Key::BatteryRow),
            value,
            Message::Open(SettingsSection::Battery),
        ),
        (
            "i",
            style::badge(SettingsSection::SystemInfo),
            language.text(Key::System),
            "ESP32-S3 AMOLED".to_string(),
            Message::Open(SettingsSection::SystemInfo),
        ),
        (
            "T",
            style::badge(SettingsSection::Theme),
            language.text(Key::Theme),
            preferences.theme.name(language).to_string(),
            Message::Open(SettingsSection::Theme),
        ),
        (
            "D",
            style::badge(SettingsSection::Time),
            language.text(Key::Time),
            "10:24 (UTC+8)".to_string(),
            Message::Open(SettingsSection::Time),
        ),
        (
            "L",
            style::badge(SettingsSection::Main),
            language.text(Key::Language),
            language.name().to_string(),
            // The row *is* the switch: pressing it hands the app the other language, and the
            // value shows which one is on. A page of two rows would be the conventional shape and
            // is what this becomes if a third language ever arrives.
            Message::SetLanguage(language.other()),
        ),
    ];

    let last = entries.len() - 1;
    let mut rows: Vec<UI<'a>> = Vec::new();

    for (index, (glyph, color, label, value, message)) in entries.into_iter().enumerate() {
        rows.push(row(glyph, color, label, value, message, theme));

        if index < last {
            rows.push(separator(theme));
        }
    }

    let footnote = text(language.text(Key::BackHint))
        .size(style::FOOTNOTE_FONT)
        .color(style::footnote_for(theme));

    let body = Column::with_children(vec![
        card(Column::with_children(rows), theme),
        Space::new()
            .height(Length::Fixed(style::FOOTNOTE_GAP))
            .into(),
        footnote.into(),
    ])
    .padding(Padding {
        top: style::BLOCK_GAP,
        bottom: style::MAIN_BOTTOM_GAP,
        left: style::PAGE_MARGIN,
        right: style::PAGE_MARGIN,
    })
    .width(Length::Fill);

    page(language.text(Key::Settings), body, theme)
}

/// One row of the main list: badge, label, value and chevron, the whole row a button.
///
/// The message is the caller's, because not every row opens a page: the language row switches the
/// language instead and is otherwise the same row.
fn row<'a>(
    glyph: &'static str,
    color: Color,
    label: &'static str,
    value: String,
    message: Message,
    theme: ThemeMode,
) -> UI<'a> {
    let left = Row::with_children(vec![
        badge(glyph, color),
        Space::new().width(Length::Fixed(style::BADGE_GAP)).into(),
        text(label)
            .size(style::LABEL_FONT)
            .color(style::label_for(theme))
            .into(),
    ])
    .align_y(Alignment::Center);

    let right = Row::with_children(vec![
        text(value)
            .size(style::VALUE_FONT)
            .color(style::muted_for(theme))
            .into(),
        Space::new().width(Length::Fixed(style::VALUE_GAP)).into(),
        text(">")
            .size(style::CHEVRON_FONT)
            .color(style::chevron_for(theme))
            .into(),
    ])
    .align_y(Alignment::Center);

    let contents = Row::with_children(vec![
        left.into(),
        Space::new().width(Length::Fill).into(),
        right.into(),
    ])
    .align_y(Alignment::Center)
    .width(Length::Fill);

    button(
        container(contents)
            .width(Length::Fill)
            .padding([style::ROW_PADDING_V, style::ROW_PADDING_H]),
    )
    .width(Length::Fill)
    .padding(0)
    .style(move |_theme, status| row_style(theme, status))
    .on_press(message)
    .into()
}

/// The badge square at the left of a row.
fn badge<'a>(glyph: &'static str, color: Color) -> UI<'a> {
    container(text(glyph).size(style::BADGE_FONT).color(Color::WHITE))
        .width(Length::Fixed(style::BADGE))
        .height(Length::Fixed(style::BADGE))
        .center_x(Length::Fixed(style::BADGE))
        .center_y(Length::Fixed(style::BADGE))
        .style(move |_theme| container::Style {
            background: Some(color.into()),
            border: Border {
                radius: style::BADGE_RADIUS.into(),
                ..Border::default()
            },
            ..container::Style::default()
        })
        .into()
}

// =============================================================================
// The sub-pages
// =============================================================================

/// The Wi-Fi page: the radio's switch, the scan, what it found, and the connection.
///
/// The password prompt is *not* part of the body: it is a `stack` over the whole page, because a
/// prompt is a modal — the list behind it must not take the finger. See [`password_prompt`].
fn wifi_page<'a>(preferences: SystemPreferences, on: bool, wifi: &Wifi) -> UI<'a> {
    let language = preferences.language;
    let theme = preferences.theme;
    let mut parts: Vec<UI<'a>> = vec![switch_row(
        language.text(Key::Wifi),
        toggle(on, Message::ToggleWifi, theme),
        theme,
    )];

    if !on {
        parts.push(notice_card(language.text(Key::WifiOff), theme));

        return page(
            language.text(Key::Wifi),
            body(parts),
            theme,
        );
    }

    // The scan's own row: what the radio is doing, and the way to ask for another one.
    let scanning = wifi.scan == ScanState::Scanning;
    parts.push(card(action_row(
        language.text(if scanning {
            Key::Scanning
        } else {
            Key::ScanAgain
        }),
        if scanning { None } else { Some(Message::WifiScan) },
        theme,
    ), theme));

    parts.push(network_list(language, wifi, theme));

    if wifi.status.state == WifiState::Connected {
        parts.push(connection_card(language, &wifi.status, theme));
    }

    let page = page(
        language.text(Key::Wifi),
        body(parts),
        theme,
    );

    match password_prompt(preferences, wifi) {
        Some(prompt) => stack![page, prompt].into(),
        None => page,
    }
}

/// The networks the last scan found, best signal first — or a line saying there are none.
fn network_list<'a>(language: Language, wifi: &Wifi, theme: ThemeMode) -> UI<'a> {
    if wifi.access_points.is_empty() {
        return notice_card(language.text(Key::NoNetworks), theme);
    }

    let connected = if wifi.status.state == WifiState::Connected {
        Some(wifi.status.ssid.as_str())
    } else {
        None
    };

    let last = wifi.access_points.len() - 1;
    let mut rows: Vec<UI<'a>> = Vec::new();

    for (index, ap) in wifi.access_points.iter().enumerate() {
        rows.push(network_row(
            language,
            ap,
            connected == Some(ap.ssid.as_str()),
            index,
            theme,
        ));

        if index < last {
            rows.push(separator(theme));
        }
    }

    card(Column::with_children(rows).width(Length::Fill), theme)
}

/// One network: its name, whether it is locked, whether we are on it, and its signal.
///
/// The lock is a *word* and not a padlock, for the same reason the keyboard's shift key says
/// `shift`: the font is a Chinese and Latin subset with no padlock in it. That is no longer the
/// whole story — `pomelo-material-symbols` is an icon font and `Icon::LOCK` is one call site away
/// — but swapping it is a change to this list's layout, not to this sentence.
fn network_row<'a>(language: Language, ap: &ApInfo, connected: bool, index: usize, theme: ThemeMode) -> UI<'a> {
    let mut right: Vec<UI<'a>> = Vec::new();

    if ap.secure {
        right.push(
            text(language.text(Key::Secured))
                .size(style::VALUE_FONT)
                .color(style::notice_for(theme))
                .into(),
        );
        right.push(Space::new().width(Length::Fixed(style::SIGNAL_GAP)).into());
    }

    if connected {
        right.push(
            text(language.text(Key::Connected))
                .size(style::VALUE_FONT)
                .color(style::accent())
                .into(),
        );
        right.push(Space::new().width(Length::Fixed(style::SIGNAL_GAP)).into());
    }

    right.push(signal_bars(ap.signal_bars(), theme));

    let contents = Row::with_children(vec![
        text(ap.ssid.clone())
            .size(style::LABEL_FONT)
            .color(style::label_for(theme))
            .into(),
        Space::new().width(Length::Fill).into(),
        Row::with_children(right)
            .align_y(Alignment::Center)
            .into(),
    ])
    .align_y(Alignment::Center)
    .width(Length::Fill);

    button(
        container(contents)
            .width(Length::Fill)
            .padding([style::ROW_PADDING_V, style::ROW_PADDING_H]),
    )
    .width(Length::Fill)
    .padding(0)
    .style(move |_theme, status| row_style(theme, status))
    .on_press(Message::WifiSelect(index))
    .into()
}

/// Four bars, the lit ones as tall as the signal — the same scale the launcher's status bar draws.
fn signal_bars<'a>(bars: u8, theme: ThemeMode) -> UI<'a> {
    let count = style::SIGNAL_BARS;
    let heights = style::SIGNAL_BAR_H - style::SIGNAL_BAR_MIN_H;

    let bars = (0..count).map(move |index| {
        let height =
            style::SIGNAL_BAR_MIN_H + heights * f32::from(index + 1) / f32::from(count);
        let lit = index < bars;

        container(Space::new())
            .width(Length::Fixed(style::SIGNAL_BAR_W))
            .height(Length::Fixed(height))
            .style(move |_theme| container::Style {
                background: Some(
                    if lit {
                        style::signal_on_for(theme)
                    } else {
                        style::signal_off_for(theme)
                    }
                    .into(),
                ),
                border: Border {
                    radius: 1.0.into(),
                    ..Border::default()
                },
                ..container::Style::default()
            })
            .into()
    });

    Row::with_children(bars)
        .spacing(style::SIGNAL_BAR_GAP)
        .align_y(Alignment::End)
        .into()
}

/// What is known about the connection: the network, the numbers that came with it, and the way out.
fn connection_card<'a>(language: Language, status: &WifiStatus, theme: ThemeMode) -> UI<'a> {
    let mut children = detail_rows(vec![
        (language.text(Key::Network), status.ssid.clone()),
        (language.text(Key::Signal), format!("{} dBm", status.rssi)),
        (language.text(Key::IpAddress), status.ip.clone()),
        (language.text(Key::Gateway), status.gateway.clone()),
        (language.text(Key::SubnetMask), status.netmask.clone()),
    ], theme);

    children.push(separator(theme));
    children.push(action_row(
        language.text(Key::Disconnect),
        Some(Message::WifiDisconnect),
        theme,
    ));

    card(Column::with_children(children).width(Length::Fill), theme)
}

/// A row that offers an action — or, with `message` `None`, a row that names a state instead.
///
/// It is a row and not a card, so that it can be the last line of a card that already exists
/// ([`connection_card`]) as well as a card of its own.
fn action_row<'a>(label: &'static str, message: Option<Message>, theme: ThemeMode) -> UI<'a> {
    let color = match message {
        Some(_) => style::accent(),
        None => style::notice_for(theme),
    };

    let line = Row::with_children(vec![
        text(label).size(style::DETAIL_FONT).color(color).into(),
        Space::new().width(Length::Fill).into(),
        text(">")
            .size(style::CHEVRON_FONT)
            .color(style::chevron_for(theme))
            .into(),
    ])
    .width(Length::Fill)
    .align_y(Alignment::Center);

    let contents = container(line)
        .width(Length::Fill)
        .padding([style::DETAIL_PADDING_V, style::DETAIL_PADDING_H]);

    match message {
        Some(message) => button(contents)
            .width(Length::Fill)
            .padding(0)
            .style(move |_theme, status| row_style(theme, status))
            .on_press(message)
            .into(),
        None => contents.into(),
    }
}

/// A card whose single row names a state instead of offering an action.
fn notice_card<'a>(label: &'static str, theme: ThemeMode) -> UI<'a> {
    card(
        container(
            text(label)
                .size(style::DETAIL_FONT)
                .color(style::notice_for(theme)),
        )
        .width(Length::Fill)
        .padding([style::DETAIL_PADDING_V, style::DETAIL_PADDING_H]),
        theme,
    )
}

/// The password prompt, when one is open: the network, what has been typed, and the keyboard.
///
/// It is the whole screen — a dimmed backdrop that takes the finger, with the card on it — because
/// that is what makes the list behind it unreachable. `None` when no prompt is open, which is the
/// page as it stands.
fn password_prompt<'a>(preferences: SystemPreferences, wifi: &Wifi) -> Option<UI<'a>> {
    let ap = wifi.prompt.and_then(|index| wifi.access_points.get(index))?;
    let language = preferences.language;
    let theme = preferences.theme;

    let mut children: Vec<UI<'a>> = vec![
        text(ap.ssid.clone())
            .size(style::NAV_FONT)
            .color(style::label_for(theme))
            .into(),
        Space::new()
            .height(Length::Fixed(style::PROMPT_GAP))
            .into(),
        text(language.text(Key::Password))
            .size(style::VALUE_FONT)
            .color(style::detail_key_for(theme))
            .into(),
        Space::new()
            .height(Length::Fixed(style::PROMPT_GAP))
            .into(),
        password_line(&wifi.password, wifi.revealed, theme),
    ];

    if wifi.failed {
        children.push(
            Space::new()
                .height(Length::Fixed(style::PROMPT_GAP))
                .into(),
        );
        children.push(
            text(language.text(Key::ConnectFailed))
                .size(style::VALUE_FONT)
                .color(style::error())
                .into(),
        );
    }

    children.push(Space::new().height(Length::Fixed(style::PROMPT_GAP)).into());
    children.push(action_row(
        language.text(if wifi.revealed { Key::Hide } else { Key::Show }),
        Some(Message::WifiReveal),
        theme,
    ));
    children.push(separator(theme));
    children.push(action_row(
        language.text(if wifi.pending {
            Key::Connecting
        } else {
            Key::Connect
        }),
        if wifi.pending {
            None
        } else {
            Some(Message::WifiConnect)
        },
        theme,
    ));
    children.push(separator(theme));
    children.push(action_row(
        language.text(Key::Cancel),
        Some(Message::WifiCancel),
        theme,
    ));
    children.push(Space::new().height(Length::Fixed(style::PROMPT_GAP)).into());
    children.push(
        container(touch_keyboard::band_with_theme(wifi.keyboard, Message::WifiKey, theme))
            .height(Length::Fixed(style::PROMPT_BAND_H))
            .into(),
    );

    // `opaque`: a press that lands on the card must stop there. Without it the press falls through
    // the card (a container does not take one) to the backdrop behind it, which cancels — so a
    // graze on the title would close the prompt.
    let card = opaque(
        container(Column::with_children(children).width(Length::Fill))
            .width(Length::Fill)
            .padding(style::PROMPT_PADDING)
            .style(move |_theme| container::Style {
                background: Some(style::card_for(theme).into()),
                border: Border {
                    color: style::card_border_for(theme),
                    width: style::CARD_BORDER_WIDTH,
                    radius: style::PROMPT_RADIUS.into(),
                },
                ..container::Style::default()
            }),
    );

    // Two layers, and both are load-bearing. The backdrop is a `mouse_area` that cancels, because a
    // modal has to take the finger from the page behind it — the list is a screen of buttons, and a
    // layer that does not capture is a layer the list answers through. Tapping *outside* the card
    // closing the prompt is the same rule read the other way round.
    Some(
        stack![
            mouse_area(
                container(Space::new())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(|_theme| container::Style {
                        background: Some(style::backdrop().into()),
                        ..container::Style::default()
                    })
            )
            .on_press(Message::WifiCancel),
            container(card)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(style::PROMPT_MARGIN)
                .center_x(Length::Fill)
                .center_y(Length::Fill),
        ]
        .into(),
    )
}

/// What has been typed: dots and a caret, or the password itself once it is revealed.
fn password_line<'a>(password: &str, revealed: bool, theme: ThemeMode) -> UI<'a> {
    if revealed {
        return Row::with_children(vec![
            text(password.to_string())
                .size(style::DETAIL_FONT)
                .color(style::label_for(theme))
                .into(),
            Space::new()
                .width(Length::Fixed(style::PASSWORD_DOT_GAP))
                .into(),
            caret(),
        ])
        .align_y(Alignment::Center)
        .height(Length::Fixed(style::CARET_H))
        .into();
    }

    let dot_color = style::label_for(theme);
    let mut children: Vec<UI<'a>> = password
        .chars()
        .map(move |_| {
            container(Space::new())
                .width(Length::Fixed(style::PASSWORD_DOT))
                .height(Length::Fixed(style::PASSWORD_DOT))
                .style(move |_theme| container::Style {
                    background: Some(dot_color.into()),
                    border: Border {
                        radius: (style::PASSWORD_DOT / 2.0).into(),
                        ..Border::default()
                    },
                    ..container::Style::default()
                })
                .into()
        })
        .collect();

    children.push(caret());

    Row::with_children(children)
        .spacing(style::PASSWORD_DOT_GAP)
        .align_y(Alignment::Center)
        .height(Length::Fixed(style::CARET_H))
        .into()
}

/// The caret that says where the next character goes.
///
/// Painted, not typed: the font has no `•` and no block, so what marks the cursor is a rectangle —
/// the same choice the terminal makes for its input line.
fn caret<'a>() -> UI<'a> {
    container(Space::new())
        .width(Length::Fixed(style::CARET_W))
        .height(Length::Fixed(style::CARET_H))
        .style(|_theme| container::Style {
            background: Some(style::accent().into()),
            ..container::Style::default()
        })
        .into()
}

fn memory_page<'a>(preferences: SystemPreferences) -> UI<'a> {
    let language = preferences.language;
    let theme = preferences.theme;
    let bar = usage_bar(
        language.text(Key::MemoryUsage),
        format!("21.7% {}", language.text(Key::Used)),
        style::memory_bar(),
        21.7,
        style::memory_bar(),
        theme,
    );

    let details = vec![
        (
            language.text(Key::Total),
            "8,519,680 bytes (8.5 MB)".to_string(),
        ),
        (
            language.text(Key::InternalSram),
            "512 KB (kernel and DMA)".to_string(),
        ),
        (
            language.text(Key::Psram),
            "8.0 MB Octal-SPI @ 80MHz".to_string(),
        ),
        (
            language.text(Key::HeapUsed),
            "1,852,416 bytes (1.85 MB)".to_string(),
        ),
        (
            language.text(Key::HeapFree),
            "6,667,264 bytes (6.66 MB)".to_string(),
        ),
        (
            language.text(Key::Framebuffer),
            "921.6 KB (480x480 RGB565)".to_string(),
        ),
        (
            language.text(Key::Health),
            "good (0% fragmentation)".to_string(),
        ),
    ];

    page(
        language.text(Key::MemoryTitle),
        body(vec![bar, detail_card(details, theme)]),
        theme,
    )
}

fn storage_page<'a>(preferences: SystemPreferences) -> UI<'a> {
    let language = preferences.language;
    let theme = preferences.theme;
    let bar = usage_bar(
        language.text(Key::LittlefsPartition),
        format!("99.9% {}", language.text(Key::Free)),
        style::green(),
        4.5,
        style::storage_bar(),
        theme,
    );

    let details = vec![
        (language.text(Key::MountPoint), "/storage".to_string()),
        (
            language.text(Key::Filesystem),
            "LittleFS (power-fail safe)".to_string(),
        ),
        (
            language.text(Key::Total),
            "11,534,336 bytes (11.0 MB)".to_string(),
        ),
        (language.text(Key::Used), "8,192 bytes (0.07%)".to_string()),
        (
            language.text(Key::Free),
            "11,526,144 bytes (99.93%)".to_string(),
        ),
        (
            language.text(Key::Files),
            "1 file (welcome.txt)".to_string(),
        ),
        (
            language.text(Key::WearLevelling),
            "active (good)".to_string(),
        ),
    ];

    page(
        language.text(Key::Storage),
        body(vec![bar, detail_card(details, theme)]),
        theme,
    )
}

fn battery_page<'a>(preferences: SystemPreferences, battery: Battery) -> UI<'a> {
    let language = preferences.language;
    let theme = preferences.theme;
    let details = vec![
        (language.text(Key::Level), format!("{}%", battery.percent)),
        (
            language.text(Key::Power),
            if battery.charging {
                "USB-C".to_string()
            } else {
                "Battery".to_string()
            },
        ),
        (
            language.text(Key::Charging),
            if battery.charging {
                language.text(Key::Charging).to_string()
            } else {
                language.text(Key::NotCharging).to_string()
            },
        ),
        (
            language.text(Key::Voltage),
            format!("{} mV", battery.voltage_mv),
        ),
        (language.text(Key::Health), "98% (excellent)".to_string()),
        (language.text(Key::Pmic), "AXP2101 (I2C 0x34)".to_string()),
        (language.text(Key::LowPowerMode), "off (60Hz)".to_string()),
        (language.text(Key::Temperature), "31.4 C".to_string()),
    ];

    page(
        language.text(Key::Battery),
        body(vec![detail_card(details, theme)]),
        theme,
    )
}

fn system_page<'a>(preferences: SystemPreferences) -> UI<'a> {
    let language = preferences.language;
    let theme = preferences.theme;
    let details = vec![
        (
            language.text(Key::Model),
            "Waveshare ESP32-S3 AMOLED 2.16\"".to_string(),
        ),
        (
            language.text(Key::Os),
            "Pomelo OS v0.2.0 (Build 2026.09)".to_string(),
        ),
        (
            language.text(Key::Cpu),
            "Xtensa Dual-Core LX7 @ 240MHz".to_string(),
        ),
        (
            language.text(Key::Display),
            "CO5300 480x480 QSPI AMOLED".to_string(),
        ),
        (language.text(Key::Colour), "100% DCI-P3".to_string()),
        (
            language.text(Key::Touch),
            "CST816 capacitive (I2C)".to_string(),
        ),
        (
            language.text(Key::Renderer),
            "iced widgets over pomelo-gfx".to_string(),
        ),
        (
            language.text(Key::Flash),
            "16 MB Quad-SPI Flash".to_string(),
        ),
    ];

    page(
        language.text(Key::About),
        body(vec![detail_card(details, theme)]),
        theme,
    )
}

fn theme_page<'a>(preferences: SystemPreferences) -> UI<'a> {
    let language = preferences.language;
    let theme = preferences.theme;

    let mode_row = row(
        "M",
        style::badge(SettingsSection::Theme),
        language.text(Key::DarkMode),
        preferences.theme.name(language).to_string(),
        Message::SetTheme(preferences.theme.other()),
        theme,
    );
    let font_row = row(
        "A",
        style::badge(SettingsSection::SystemInfo),
        language.text(Key::FontSize),
        preferences.font_tier.name(language).to_string(),
        Message::CycleFontTier,
        theme,
    );
    let controls = card(Column::with_children(vec![mode_row, separator(theme), font_row]), theme);

    let colors = style::palette();

    let top = Row::with_children(
        colors[..4]
            .iter()
            .map(|color| chip(*color, theme))
            .collect::<Vec<_>>(),
    )
    .spacing(style::CHIP_GAP);
    let bottom = Row::with_children(
        colors[4..]
            .iter()
            .map(|color| chip(*color, theme))
            .collect::<Vec<_>>(),
    )
    .spacing(style::CHIP_GAP);

    let palette = card(
        container(
            Column::with_children(vec![
                text(language.text(Key::Presets))
                    .size(style::DETAIL_FONT)
                    .color(style::label_for(theme))
                    .into(),
                Space::new().height(Length::Fixed(style::CHIP_GAP)).into(),
                top.into(),
                Space::new().height(Length::Fixed(style::CHIP_GAP)).into(),
                bottom.into(),
            ])
            .width(Length::Fill)
            .align_x(Alignment::Center),
        )
        .padding(style::PALETTE_PADDING),
        theme,
    );

    let details = vec![
        (language.text(Key::Style), preferences.theme.name(language).to_string()),
        (language.text(Key::Wallpaper), "macOS Sierra".to_string()),
        (
            language.text(Key::Emissive),
            "AMOLED (no backlight bleed)".to_string(),
        ),
        (
            language.text(Key::RefreshRate),
            "60 Hz Direct QSPI DMA".to_string(),
        ),
        (
            language.text(Key::AntiAliasing),
            "subpixel coverage (analytical AA)".to_string(),
        ),
    ];

    page(
        language.text(Key::ThemeTitle),
        body(vec![controls, palette, detail_card(details, theme)]),
        theme,
    )
}

fn time_page<'a>(preferences: SystemPreferences, is_24h: bool) -> UI<'a> {
    let language = preferences.language;
    let theme = preferences.theme;
    let details = vec![
        (
            language.text(Key::SystemTime),
            if is_24h {
                "10:24".to_string()
            } else {
                "10:24 AM".to_string()
            },
        ),
        (
            language.text(Key::TimeZone),
            "CST (UTC+8, Beijing)".to_string(),
        ),
        (language.text(Key::NtpSync), "on (pool.ntp.org)".to_string()),
        (language.text(Key::Rtc), "ESP32-S3 internal RTC".to_string()),
        (
            language.text(Key::SyncStatus),
            "calibrated (offset < 5ms)".to_string(),
        ),
    ];

    page(
        language.text(Key::Time),
        body(vec![
            switch_row(
                language.text(Key::TwentyFourHour),
                toggle(is_24h, Message::Toggle24Hour, theme),
                theme,
            ),
            detail_card(details, theme),
        ]),
        theme,
    )
}

// =============================================================================
// Layout helpers
// =============================================================================

/// The 44px navigation bar: a centered title.
fn nav_bar<'a>(title: &'static str, theme: ThemeMode) -> UI<'a> {
    let title = container(text(title).size(style::NAV_FONT).color(style::label_for(theme)))
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center)
        .align_y(Alignment::Center);

    container(title)
        .width(Length::Fill)
        .height(Length::Fixed(style::NAV_HEIGHT))
        .padding([0.0, style::NAV_PADDING])
        .style(move |_theme| container::Style {
            background: Some(style::nav_for(theme).into()),
            ..container::Style::default()
        })
        .into()
}

/// `title` bar plus a scrollable column page body.
fn page<'a>(title: &'static str, body: impl Into<UI<'a>>, theme: ThemeMode) -> UI<'a> {
    Column::with_children(vec![
        nav_bar(title, theme),
        // The body takes the rest of the height, so a taller panel shows more of the list
        // instead of leaving a black band under it.
        scrollable(body)
            .direction(Direction::Vertical(Scrollbar::hidden()))
            .width(Length::Fill)
            .height(Length::Fill)
            .into(),
    ])
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

/// A page body: the blocks, `BLOCK_GAP` apart, with `BOTTOM_GAP` under the last,
/// with horizontal and vertical page padding directly applied to the Column.
fn body<'a>(parts: Vec<UI<'a>>) -> Column<'a, Message, Theme, Renderer> {
    Column::with_children(parts)
        .spacing(style::BLOCK_GAP)
        .padding(Padding {
            top: style::BLOCK_GAP,
            bottom: style::BOTTOM_GAP,
            left: style::PAGE_MARGIN,
            right: style::PAGE_MARGIN,
        })
        .width(Length::Fill)
}

/// A rounded card, as wide as the page it sits in.
fn card<'a>(content: impl Into<UI<'a>>, theme: ThemeMode) -> UI<'a> {
    container(content)
        .width(Length::Fill)
        .style(move |_theme| container::Style {
            background: Some(style::card_for(theme).into()),
            border: Border {
                color: style::card_border_for(theme),
                width: style::CARD_BORDER_WIDTH,
                radius: style::CARD_RADIUS.into(),
            },
            ..container::Style::default()
        })
        .into()
}

/// The hairline between two rows of a card.
fn separator<'a>(theme: ThemeMode) -> UI<'a> {
    container(Space::new().width(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fixed(style::ROW_SEPARATOR))
        .style(move |_theme| container::Style {
            background: Some(style::separator_for(theme).into()),
            ..container::Style::default()
        })
        .into()
}

/// The rows of a key/value table: one line each, with hairlines between them.
fn detail_rows<'a>(rows: Vec<(&'static str, String)>, theme: ThemeMode) -> Vec<UI<'a>> {
    let last = rows.len().saturating_sub(1);
    let mut children: Vec<UI<'a>> = Vec::new();

    for (index, (key, value)) in rows.into_iter().enumerate() {
        let line = Row::with_children(vec![
            text(key)
                .size(style::DETAIL_FONT)
                .color(style::detail_key_for(theme))
                .into(),
            Space::new().width(Length::Fill).into(),
            text(value)
                .size(style::DETAIL_FONT)
                .color(style::label_for(theme))
                .into(),
        ])
        .width(Length::Fill)
        .align_y(Alignment::Center);

        children.push(
            container(line)
                .width(Length::Fill)
                .padding([style::DETAIL_PADDING_V, style::DETAIL_PADDING_H])
                .into(),
        );

        if index < last {
            children.push(separator(theme));
        }
    }

    children
}

/// A key/value table in a card.
fn detail_card<'a>(rows: Vec<(&'static str, String)>, theme: ThemeMode) -> UI<'a> {
    card(Column::with_children(detail_rows(rows, theme)).width(Length::Fill), theme)
}

/// A card whose row is a label and a switch.
fn switch_row<'a>(label: &'static str, control: UI<'a>, theme: ThemeMode) -> UI<'a> {
    let line = Row::with_children(vec![
        text(label)
            .size(style::LABEL_FONT)
            .color(style::label_for(theme))
            .into(),
        Space::new().width(Length::Fill).into(),
        control,
    ])
    .width(Length::Fill)
    .align_y(Alignment::Center);

    card(container(line).padding([style::SWITCH_PADDING_V, style::SWITCH_PADDING_H]), theme)
}

/// A card with a labelled progress bar.
///
/// `percent` — not pixels: the two halves of the bar are flex shares, so the same number
/// describes the design's 440pt card and any other width.
fn usage_bar<'a>(
    title: &'static str,
    value: String,
    value_color: Color,
    percent: f32,
    bar_color: Color,
    theme: ThemeMode,
) -> UI<'a> {
    let share = (percent.clamp(0.0, 100.0) * 100.0).round() as u16;
    let rest = 10_000u16.saturating_sub(share);

    let filled = container(Space::new().width(Length::Fill))
        .width(Length::FillPortion(share))
        .height(Length::Fixed(style::BAR_HEIGHT))
        .style(move |_theme| container::Style {
            background: Some(bar_color.into()),
            border: Border {
                radius: style::BAR_RADIUS.into(),
                ..Border::default()
            },
            ..container::Style::default()
        });

    let remaining = container(Space::new().width(Length::Fill))
        .width(Length::FillPortion(rest.max(1)))
        .height(Length::Fixed(style::BAR_HEIGHT));

    let track = container(
        Row::with_children(vec![filled.into(), remaining.into()])
            .width(Length::Fill)
            .height(Length::Fixed(style::BAR_HEIGHT)),
    )
    .width(Length::Fill)
    .height(Length::Fixed(style::BAR_HEIGHT))
    .style(move |_theme| container::Style {
        background: Some(style::bar_track_for(theme).into()),
        border: Border {
            radius: style::BAR_RADIUS.into(),
            ..Border::default()
        },
        ..container::Style::default()
    });

    let heading = Row::with_children(vec![
        text(title)
            .size(style::DETAIL_FONT)
            .color(style::label_for(theme))
            .into(),
        Space::new().width(Length::Fill).into(),
        text(value)
            .size(style::DETAIL_FONT)
            .color(value_color)
            .into(),
    ])
    .width(Length::Fill);

    card(
        container(
            Column::with_children(vec![
                heading.into(),
                Space::new().height(Length::Fixed(style::BAR_GAP)).into(),
                track.into(),
            ])
            .width(Length::Fill),
        )
        .padding(style::USAGE_PADDING),
        theme,
    )
}

/// An iOS switch: a 52x28 track with a 24px knob, drawn from containers.
///
/// The whole track is the button. The knob is positioned by a fill on the side it is moving
/// away from, which is what puts it 2px from the edge it rests against without hard-coding the
/// track's own width into the knob's position.
fn toggle<'a>(on: bool, message: Message, theme: ThemeMode) -> UI<'a> {
    let knob = container(Space::new())
        .width(Length::Fixed(style::KNOB))
        .height(Length::Fixed(style::KNOB))
        .style(|_theme| container::Style {
            background: Some(Color::WHITE.into()),
            border: Border {
                radius: style::KNOB_RADIUS.into(),
                ..Border::default()
            },
            ..container::Style::default()
        });

    let mut parts: Vec<UI<'a>> = Vec::new();

    if on {
        parts.push(Space::new().width(Length::Fill).into());
        parts.push(knob.into());
        parts.push(
            Space::new()
                .width(Length::Fixed(style::TOGGLE_INSET))
                .into(),
        );
    } else {
        parts.push(
            Space::new()
                .width(Length::Fixed(style::TOGGLE_INSET))
                .into(),
        );
        parts.push(knob.into());
        parts.push(Space::new().width(Length::Fill).into());
    }

    button(
        Row::with_children(parts)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_y(Alignment::Center),
    )
    .width(Length::Fixed(style::TOGGLE_W))
    .height(Length::Fixed(style::TOGGLE_H))
    .padding(0)
    .style(move |_theme, status| toggle_style(on, status, theme))
    .on_press(message)
    .into()
}

/// One preset colour, 60x40.
fn chip<'a>(color: Color, theme: ThemeMode) -> UI<'a> {
    container(Space::new())
        .width(Length::Fixed(style::CHIP_W))
        .height(Length::Fixed(style::CHIP_H))
        .style(move |_theme| container::Style {
            background: Some(color.into()),
            border: Border {
                color: style::chip_border_for(theme),
                width: 1.0,
                radius: style::CHIP_RADIUS.into(),
            },
            ..container::Style::default()
        })
        .into()
}

// =============================================================================
// Styles
// =============================================================================

/// A main-list row: nothing at rest, a wash when the finger is on it.
fn row_style(theme: ThemeMode, status: button::Status) -> button::Style {
    let wash = match (status, theme) {
        (button::Status::Hovered, ThemeMode::Dark) => Some(Color::from_rgba(1.0, 1.0, 1.0, 0.05).into()),
        (button::Status::Pressed, ThemeMode::Dark) => Some(Color::from_rgba(1.0, 1.0, 1.0, 0.11).into()),
        (button::Status::Hovered, ThemeMode::Light) => Some(Color::from_rgba(0.0, 0.0, 0.0, 0.04).into()),
        (button::Status::Pressed, ThemeMode::Light) => Some(Color::from_rgba(0.0, 0.0, 0.0, 0.08).into()),
        _ => None,
    };

    button::Style {
        background: wash,
        text_color: style::label_for(theme),
        border: Border::default(),
        shadow: Shadow::default(),
        snap: false,
    }
}

/// A switch's track: green when on, grey when off.
fn toggle_style(on: bool, status: button::Status, theme: ThemeMode) -> button::Style {
    let fill = match (on, status) {
        (true, button::Status::Pressed | button::Status::Hovered) => style::green_pressed(),
        (true, _) => style::green(),
        (false, button::Status::Pressed | button::Status::Hovered) => style::track_off_pressed_for(theme),
        (false, _) => style::track_off_for(theme),
    };

    button::Style {
        background: Some(fill.into()),
        text_color: Color::WHITE,
        border: Border {
            radius: style::TOGGLE_RADIUS.into(),
            ..Border::default()
        },
        shadow: Shadow::default(),
        snap: false,
    }
}
