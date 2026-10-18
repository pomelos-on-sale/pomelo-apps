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
//! than the panel scrolls. Switching pages opens the new page at the top: the body is one
//! `scrollable` with one id, and [`Settings::update`] sends it `operation::snap_to` when the page
//! under it changes. The widget that used to do this (`page.rs`) existed only because this platform
//! could not run a widget operation at all; it can now, so the reset is iced's own.

mod i18n;
pub mod pages;
pub mod style;

use std::sync::Arc;

use iced::theme::Palette;
use iced::time::Instant;
use iced::widget::{operation, scrollable};
use iced::{Element, Subscription, Task, Theme};
use pomelo_hal::{Board, WifiState};

pub use pomelo_widgets::{
    toast_view, AppPreferences, FontSizeTier, Language, ScreenOffTimeout, SleepTimeout,
    SystemPreferences, ThemeMode, Toast,
};
pub use i18n::{Key, LanguageExt};
pub use pages::apps::{ColorRule, PresetColor};

use pages::common::BODY;
use pages::wifi::Wifi;
use pomelo_widgets::touch_keyboard::KeyAction;

pub use style::SCREEN;

/// Sends the page body back to the top.
///
/// A change of section is a different page inside the *same* `scrollable`: the tree node survives
/// the rebuild, so its offset would survive too, and the sub-page would open wherever the previous
/// one had been scrolled to. Asking for the top is a widget operation, which is what iced has for
/// exactly this.
fn to_top() -> Task<Message> {
    operation::snap_to(BODY, scrollable::RelativeOffset::START)
}

/// Restores the page body to a previous vertical scroll offset.
fn restore_scroll(y: f32) -> Task<Message> {
    if y <= 0.0 {
        to_top()
    } else {
        operation::scroll_to(BODY, scrollable::AbsoluteOffset { x: 0.0, y })
    }
}

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
    /// Backup & Restore: backup AppData to SD card and restore.
    Backup,
    /// System information.
    SystemInfo,
    /// Display and theme.
    Theme,
    /// Date and time.
    Time,
    /// Applications management page.
    Apps,
    /// Daily Schedule app settings.
    AppDailySchedule,
    /// Daily Schedule subscription link details.
    AppDailyScheduleUrl,
    /// Daily Schedule event title color presets.
    AppDailyScheduleColors,
    /// Screen-off timeout selection page.
    ScreenOffTimeout,
    /// Auto-sleep timeout selection page.
    SleepTimeout,
    /// Interface language selection page.
    Language,
}

pub(crate) const SECTION_COUNT: usize = 16;

impl SettingsSection {
    pub const fn index(self) -> usize {
        match self {
            Self::Main => 0,
            Self::Wifi => 1,
            Self::Memory => 2,
            Self::Storage => 3,
            Self::Battery => 4,
            Self::Backup => 5,
            Self::SystemInfo => 6,
            Self::Theme => 7,
            Self::Time => 8,
            Self::Apps => 9,
            Self::AppDailySchedule => 10,
            Self::AppDailyScheduleUrl => 11,
            Self::AppDailyScheduleColors => 12,
            Self::ScreenOffTimeout => 13,
            Self::SleepTimeout => 14,
            Self::Language => 15,
        }
    }
}

/// What the app reacts to.
#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    /// The page body was scrolled to an offset.
    Scrolled(scrollable::AbsoluteOffset),
    /// A row of the main list was pressed.
    Open(SettingsSection),
    /// The back button (or the hardware back key) was pressed.
    Back,
    /// The Wi-Fi switch was flipped.
    ToggleWifi,
    /// The 24-hour-time switch was flipped.
    Toggle24Hour,
    /// Toggle stay awake on USB.
    ToggleStayAwakeOnUsb,
    /// The language row was pressed, and the interface is now in `Language`.
    SetLanguage(Language),
    /// Set the appearance theme mode.
    SetTheme(ThemeMode),
    /// Cycle to the next font size tier.
    CycleFontTier,
    /// Prompt modal dialog to select screen-off inactivity timeout.
    PromptScreenOffTimeout,
    /// Cancel or dismiss the screen-off timeout selection dialog.
    CancelScreenOffTimeout,
    /// Cycle to the next screen-off inactivity timeout option.
    CycleScreenOffTimeout,
    /// Set the screen-off inactivity timeout option.
    SetScreenOffTimeout(ScreenOffTimeout),
    /// Prompt modal dialog to select screen auto-sleep inactivity timeout.
    PromptSleepTimeout,
    /// Cancel or dismiss the sleep timeout selection dialog.
    CancelSleepTimeout,
    /// Cycle to the next screen auto-sleep inactivity timeout option.
    CycleSleepTimeout,
    /// Set the screen auto-sleep inactivity timeout option.
    SetSleepTimeout(SleepTimeout),

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
    /// Asynchronously received clipboard text for Wi-Fi password.
    WifiClipboardPasted(Option<String>),
    /// The prompt's connect button — or its return key.
    WifiConnect,
    /// The prompt's cancel button.
    WifiCancel,
    /// The prompt's show/hide button: draw the password in the clear, or as dots again.
    WifiReveal,
    /// The connection card's disconnect row.
    WifiDisconnect,
    /// The password was edited in the text input widget.
    WifiPasswordChanged(String),

    /// Prompt confirmation modal to format the SD card.
    PromptFormatSd,
    /// Confirm and trigger SD card formatting on the device.
    ConfirmFormatSd,
    /// Cancel SD card formatting prompt.
    CancelFormatSd,

    /// Prompt confirmation modal to back up AppData.
    PromptBackupAppData,
    /// Confirm and trigger AppData backup.
    ConfirmBackupAppData,
    /// Cancel AppData backup prompt.
    CancelBackupAppData,

    /// Prompt confirmation modal to restore AppData.
    PromptRestoreAppData,
    /// Confirm and trigger AppData restore.
    ConfirmRestoreAppData,
    /// Cancel AppData restore prompt.
    CancelRestoreAppData,

    /// Dismiss backup or restore result notice dialog.
    DismissBackupNotice,
    /// Dismiss active toast banner.
    DismissToast(u32),

    /// Import calendar subscription link from system clipboard.
    DailyScheduleImportClipboard,
    /// Asynchronously received calendar subscription text from clipboard.
    DailyScheduleClipboardImported(Option<String>),
    /// Prompt confirmation modal to clear calendar subscription.
    DailySchedulePromptClear,
    /// Confirm clearing calendar subscription and cache.
    DailyScheduleConfirmClear,
    /// Cancel clearing calendar subscription prompt.
    DailyScheduleCancelClear,
    /// Trigger a test sync of the configured calendar URL.
    DailyScheduleTestSync,
    /// Frame tick while waiting for calendar sync test to complete.
    DailyScheduleSyncFrame,
    /// Dismiss daily schedule notice dialog.
    DailyScheduleDismissNotice,
    /// Toggle daily schedule reminder sound setting.
    DailyScheduleToggleReminderSound,

    /// Prompt modal to add a new calendar event color rule.
    DailySchedulePromptAddRule,
    /// Prompt modal to edit an existing calendar event color rule.
    DailySchedulePromptEditRule(usize),
    /// Cancel color rule prompt modal.
    DailyScheduleCancelRule,
    /// Save added or edited color rule.
    DailyScheduleSaveRule,
    /// Delete an existing color rule.
    DailyScheduleDeleteRule(usize),
    /// Color rule keyword text changed.
    DailyScheduleRuleKeywordChanged(String),
    /// Color rule touch keyboard key pressed.
    DailyScheduleRuleKey(KeyAction),
    /// Color rule preset color selected in dropdown.
    DailyScheduleRuleColorSelected(pages::apps::PresetColor),
    /// Asynchronously received keyword text from clipboard for color rule.
    DailyScheduleRuleClipboardImported(Option<String>),
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
///
/// What is left here is what the *app* has: which page is up, the preferences every page draws
/// from, the battery reading the platform pushes in, and the routing between them. Subpage
/// state and operations live with their respective modules:
/// - [`pages::wifi::Wifi`]
/// - [`pages::storage::StorageState`]
/// - [`pages::backup::BackupState`]
/// - [`pages::theme::ThemePageState`]
/// - [`pages::apps::DailyScheduleSettingsState`]
pub struct Settings {
    board: Arc<Board>,
    section: SettingsSection,
    scroll_offsets: [f32; SECTION_COUNT],
    is_24h_format: bool,
    battery: Battery,
    /// The system preferences (language, theme, font size tier).
    preferences: SystemPreferences,
    /// The Wi-Fi page, which owns the board: it is the one page that talks to hardware rather than
    /// reading a value the platform pushed in.
    wifi: Wifi,
    storage: pages::storage::StorageState,
    backup: pages::backup::BackupState,
    theme_state: pages::theme::ThemePageState,
    daily_schedule: pages::apps::DailyScheduleSettingsState,
    pub toast: Toast,
}

impl Settings {
    /// The app at the main list, the way the original starts, with `board` for the pages that need
    /// hardware.
    pub fn new(board: Arc<Board>) -> Self {
        let preferences = SystemPreferences::load();
        let _ = board.set_screen_off_timeout(preferences.screen_off_timeout.as_duration());
        let _ = board.set_sleep_timeout(preferences.sleep_timeout.as_duration());
        let _ = board.set_stay_awake_on_usb(preferences.stay_awake_on_usb);
        let mut settings = Self {
            board: Arc::clone(&board),
            section: SettingsSection::Main,
            scroll_offsets: [0.0; SECTION_COUNT],
            is_24h_format: true,
            battery: Battery::default(),
            preferences,
            wifi: Wifi::new(board),
            storage: pages::storage::StorageState::default(),
            backup: pages::backup::BackupState::default(),
            theme_state: pages::theme::ThemePageState::default(),
            daily_schedule: pages::apps::DailyScheduleSettingsState::default(),
            toast: Toast::new(),
        };

        // One read at boot, so the switch and the main list's Wi-Fi row are true before the page is
        // ever opened.
        settings.wifi.read();
        settings.refresh_storage();
        settings.refresh_daily_schedule();

        settings
    }

    /// Returns the recorded vertical scroll offset for `section`.
    pub fn scroll_offset(&self, section: SettingsSection) -> f32 {
        self.scroll_offsets[section.index()]
    }

    /// Sets the recorded vertical scroll offset for `section`.
    pub fn set_scroll_offset(&mut self, section: SettingsSection, offset: f32) {
        self.scroll_offsets[section.index()] = offset.max(0.0);
    }

    /// Shows a floating toast message for standard 2.0s duration.
    pub fn show_toast(&mut self, text: impl Into<String>) -> Task<Message> {
        self.toast.show(text, Message::DismissToast)
    }

    /// The page that is open. Read by the host and by the tests.
    pub fn section(&self) -> SettingsSection {
        self.section
    }

    /// The Wi-Fi page: its state, and the reads the host and the tests need from it.
    ///
    /// One door rather than a dozen. The reads that used to sit here — the switch, the scan, the
    /// prompt, the password — were all of this page's fields, so they say so now.
    pub fn wifi(&self) -> &Wifi {
        &self.wifi
    }

    /// Whether the clock is 24-hour.
    pub fn is_24h_format(&self) -> bool {
        self.is_24h_format
    }

    /// The battery reading the battery page shows.
    pub fn battery(&self) -> Battery {
        self.battery
    }

    /// Switches to `language`.
    pub fn set_language(&mut self, language: Language) {
        self.preferences.language = language;
        let _ = self.preferences.save();
    }

    /// The aggregated system preferences.
    pub fn preferences(&self) -> SystemPreferences {
        self.preferences
    }

    /// Sets the aggregated system preferences.
    pub fn set_preferences(&mut self, preferences: SystemPreferences) {
        self.preferences = preferences;
    }

    /// Sets the visual theme mode.
    pub fn set_theme_mode(&mut self, theme: ThemeMode) {
        self.preferences.theme = theme;
        let _ = self.preferences.save();
    }

    /// Sets the font size tier.
    pub fn set_font_tier(&mut self, tier: FontSizeTier) {
        self.preferences.font_tier = tier;
        let _ = self.preferences.save();
    }

    /// Sets the screen-off inactivity timeout.
    pub fn set_screen_off_timeout(&mut self, timeout: ScreenOffTimeout) {
        self.preferences.screen_off_timeout = timeout;
        let _ = self.preferences.save();
        let _ = self.wifi.board().set_screen_off_timeout(timeout.as_duration());
    }

    /// Sets the screen auto-sleep inactivity timeout.
    pub fn set_sleep_timeout(&mut self, timeout: SleepTimeout) {
        self.preferences.sleep_timeout = timeout;
        let _ = self.preferences.save();
        let _ = self.wifi.board().set_sleep_timeout(timeout.as_duration());
    }

    /// Sets whether to stay awake while connected to USB.
    pub fn set_stay_awake_on_usb(&mut self, enable: bool) {
        self.preferences.stay_awake_on_usb = enable;
        let _ = self.preferences.save();
        let _ = self.wifi.board().set_stay_awake_on_usb(enable);
    }

    /// Opens `section`.
    ///
    /// A change of section opens the new page at the top, which [`Settings::open`] arranges;
    /// setting the section it is already on changes nothing, so the offset survives — which is
    /// exactly what the original's `SettingsModel::set_section` did.
    pub fn set_section(&mut self, section: SettingsSection) {
        self.section = section;
    }

    /// Returns to the main list, reporting whether there was anywhere to return from and what work
    /// the press produced.
    ///
    /// `None` means the back press was *not* consumed: the launcher backgrounds the app instead,
    /// which is the contract the original's `go_back` had. `Some(task)` carries the work the press
    /// produced — sending the page it returns to back to the top.
    ///
    /// The password prompt is a page of its own while it is open, so back closes it before it
    /// leaves the Wi-Fi page — and it closes it *here* rather than in `update`, because the launcher
    /// intercepts `Message::Back` and calls this directly: a modal that only closed when the app was
    /// running standalone would be a modal that never closes on the board.
    pub fn go_back(&mut self) -> Option<Task<Message>> {
        if self.theme_state.has_active_modal() {
            self.theme_state.dismiss_modal();
            return Some(Task::none());
        }

        if self.storage.has_active_modal() {
            self.storage.dismiss_modal();
            return Some(Task::none());
        }

        if self.backup.has_active_modal() {
            self.backup.dismiss_modal();
            return Some(Task::none());
        }

        if self.daily_schedule.has_active_modal() {
            self.daily_schedule.dismiss_modal();
            return Some(Task::none());
        }

        if self.wifi.has_prompt() {
            self.wifi.cancel_prompt();

            // The prompt sits over the page; the body under it did not move.
            return Some(Task::none());
        }

        if self.section == SettingsSection::ScreenOffTimeout
            || self.section == SettingsSection::SleepTimeout
        {
            self.section = SettingsSection::Theme;
            let offset = self.scroll_offset(SettingsSection::Theme);
            Some(restore_scroll(offset))
        } else if self.section == SettingsSection::AppDailyScheduleUrl {
            self.section = SettingsSection::AppDailySchedule;
            let offset = self.scroll_offset(SettingsSection::AppDailySchedule);
            Some(restore_scroll(offset))
        } else if self.section == SettingsSection::AppDailyScheduleColors {
            self.section = SettingsSection::AppDailySchedule;
            let offset = self.scroll_offset(SettingsSection::AppDailySchedule);
            Some(restore_scroll(offset))
        } else if self.section == SettingsSection::AppDailySchedule {
            if self.daily_schedule.is_testing_sync {
                let _ = self.board.http().cancel();
                self.daily_schedule.is_testing_sync = false;
            }
            self.section = SettingsSection::Apps;
            let offset = self.scroll_offset(SettingsSection::Apps);
            Some(restore_scroll(offset))
        } else if self.section == SettingsSection::Main {
            None
        } else {
            self.section = SettingsSection::Main;
            let offset = self.scroll_offset(SettingsSection::Main);
            Some(restore_scroll(offset))
        }
    }

    /// Takes a battery reading the platform pushed in.
    pub fn set_battery(&mut self, reading: Battery) {
        self.battery = reading;
    }

    /// Refreshes the cached storage subsystem readings (status, capacity stats, card info).
    pub fn refresh_storage(&mut self) {
        self.storage.refresh(&self.board);
        self.backup.refresh(&self.board);
    }

    /// Refreshes the daily schedule config from disk.
    pub fn refresh_daily_schedule(&mut self) {
        self.daily_schedule.refresh(&self.board);
    }

    /// Storage subsystem state and operations.
    pub fn storage(&self) -> &pages::storage::StorageState {
        &self.storage
    }

    /// Backup subsystem state and operations.
    pub fn backup(&self) -> &pages::backup::BackupState {
        &self.backup
    }

    /// Display and theme page state.
    pub fn theme_state(&self) -> &pages::theme::ThemePageState {
        &self.theme_state
    }

    /// The current daily schedule configuration.
    pub fn daily_schedule_config(&self) -> &pages::apps::DailyScheduleConfig {
        &self.daily_schedule.config
    }

    /// Whether an asynchronous schedule sync test is currently underway.
    pub fn is_testing_schedule_sync(&self) -> bool {
        self.daily_schedule.is_testing_sync
    }

    /// The current active schedule sync notice, if any.
    pub fn schedule_sync_notice(&self) -> Option<&(String, String)> {
        self.daily_schedule.sync_notice.as_ref()
    }

    /// The active color rule modal state, if open.
    pub fn daily_schedule_color_rule_modal(&self) -> Option<&pages::apps::ColorRuleModalState> {
        self.daily_schedule.color_rule_modal.as_ref()
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

    /// Reacts to one message, and returns whatever work it produced.
    ///
    /// Only a change of page produces any: the body is a single `scrollable`, and it has to be sent
    /// back to the top when the page under it changes. See the module docs.
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Scrolled(offset) => {
                self.set_scroll_offset(self.section, offset.y);
                return Task::none();
            }
            Message::Open(section) => return self.open(section),
            Message::Back => return self.go_back().unwrap_or_else(Task::none),
            Message::PromptScreenOffTimeout => return self.open(SettingsSection::ScreenOffTimeout),
            Message::CancelScreenOffTimeout => {
                if self.section == SettingsSection::ScreenOffTimeout {
                    return self.go_back().unwrap_or_else(Task::none);
                }
                return Task::none();
            }
            Message::PromptSleepTimeout => return self.open(SettingsSection::SleepTimeout),
            Message::CancelSleepTimeout => {
                if self.section == SettingsSection::SleepTimeout {
                    return self.go_back().unwrap_or_else(Task::none);
                }
                return Task::none();
            }
            Message::WifiSelect(index) => {
                let secure = self
                    .wifi
                    .access_points()
                    .get(index)
                    .map(|ap| ap.secure)
                    .unwrap_or(false);
                self.react(message);
                if secure {
                    return iced::widget::operation::focus(pages::wifi::PASSWORD_INPUT_ID);
                }
                return Task::none();
            }
            Message::WifiKey(KeyAction::Paste) => {
                return iced::clipboard::read().map(Message::WifiClipboardPasted);
            }
            Message::WifiKey(action) => {
                self.react(message);
                if action != KeyAction::Enter {
                    return iced::widget::operation::focus(pages::wifi::PASSWORD_INPUT_ID);
                }
                return Task::none();
            }
            Message::WifiClipboardPasted(content) => {
                if let Some(text) = content {
                    self.wifi.paste(&text);
                }
                return iced::widget::operation::focus(pages::wifi::PASSWORD_INPUT_ID);
            }
            Message::DailyScheduleImportClipboard => {
                return iced::clipboard::read().map(Message::DailyScheduleClipboardImported);
            }
            Message::DailyScheduleRuleKey(KeyAction::Paste) => {
                return iced::clipboard::read().map(Message::DailyScheduleRuleClipboardImported);
            }
            Message::DailyScheduleRuleClipboardImported(content) => {
                if let Some(text) = content {
                    self.daily_schedule.paste_rule_keyword(&text);
                }
                return Task::none();
            }
            Message::DismissToast(id) => {
                self.toast.dismiss(id);
                return Task::none();
            }
            Message::ConfirmFormatSd => {
                if let Some(msg) = self.storage.confirm_format(&self.board, self.preferences.language) {
                    return self.show_toast(msg);
                }
                return Task::none();
            }
            Message::ConfirmBackupAppData => {
                if let Some(msg) = self.backup.confirm_backup(&self.board, self.preferences.language) {
                    return self.show_toast(msg);
                }
                return Task::none();
            }
            Message::ConfirmRestoreAppData => {
                if let Some(msg) = self.backup.confirm_restore(&self.board, self.preferences.language) {
                    return self.show_toast(msg);
                }
                return Task::none();
            }
            Message::DailyScheduleClipboardImported(content) => {
                self.daily_schedule.import_clipboard(content, self.preferences.language);
                if let Some((_, msg)) = self.daily_schedule.sync_notice.as_ref() {
                    return self.show_toast(msg.clone());
                }
                return Task::none();
            }
            Message::DailyScheduleConfirmClear => {
                self.daily_schedule.confirm_clear();
                if let Some((_, msg)) = self.daily_schedule.sync_notice.as_ref() {
                    return self.show_toast(msg.clone());
                }
                return Task::none();
            }
            Message::DailyScheduleSyncFrame => {
                self.daily_schedule.handle_sync_frame(&self.board, self.preferences.language);
                if !self.daily_schedule.is_testing_sync {
                    if let Some((_, msg)) = self.daily_schedule.sync_notice.as_ref() {
                        return self.show_toast(msg.clone());
                    }
                }
                return Task::none();
            }
            Message::WifiReveal => {
                self.react(message);
                return iced::widget::operation::focus(pages::wifi::PASSWORD_INPUT_ID);
            }
            _ => {}
        }

        // Everything else changes what the page *says*, not which page it is.
        self.react(message);

        Task::none()
    }

    /// The messages that change the state of the page on screen, and nothing else.
    fn react(&mut self, message: Message) {
        match message {
            Message::ToggleWifi => self.wifi.toggle(),
            Message::Toggle24Hour => self.is_24h_format = !self.is_24h_format,
            Message::SetLanguage(language) => self.set_language(language),
            Message::SetTheme(theme) => self.set_theme_mode(theme),
            Message::CycleFontTier => self.set_font_tier(self.preferences.font_tier.cycle()),
            Message::PromptScreenOffTimeout => self.theme_state.prompt_screen_off_timeout(),
            Message::CancelScreenOffTimeout => self.theme_state.cancel_screen_off_timeout(),
            Message::CycleScreenOffTimeout => {
                self.set_screen_off_timeout(self.preferences.screen_off_timeout.cycle());
            }
            Message::SetScreenOffTimeout(timeout) => {
                self.theme_state.cancel_screen_off_timeout();
                self.set_screen_off_timeout(timeout);
            }
            Message::PromptSleepTimeout => self.theme_state.prompt_sleep_timeout(),
            Message::CancelSleepTimeout => self.theme_state.cancel_sleep_timeout(),
            Message::CycleSleepTimeout => {
                self.set_sleep_timeout(self.preferences.sleep_timeout.cycle());
            }
            Message::SetSleepTimeout(timeout) => {
                self.theme_state.cancel_sleep_timeout();
                self.set_sleep_timeout(timeout);
            }
            Message::ToggleStayAwakeOnUsb => {
                self.set_stay_awake_on_usb(!self.preferences.stay_awake_on_usb);
            }
            Message::WifiScan => self.wifi.start_scan(),
            Message::WifiFrame(now) => self.wifi.poll(now),
            Message::WifiSelect(index) => self.wifi.select(index),
            Message::WifiKey(action) => self.wifi.key(action),
            Message::WifiClipboardPasted(_) => {}
            Message::WifiConnect => self.wifi.connect(),
            Message::WifiCancel => self.wifi.cancel_prompt(),
            Message::WifiReveal => self.wifi.reveal(),
            Message::WifiDisconnect => self.wifi.disconnect(),
            Message::WifiPasswordChanged(password) => self.wifi.set_password(password),

            Message::DismissToast(_) => {}

            Message::PromptFormatSd => self.storage.prompt_format(),
            Message::CancelFormatSd => self.storage.cancel_format(),
            Message::ConfirmFormatSd => {}

            Message::PromptBackupAppData => self.backup.prompt_backup(),
            Message::CancelBackupAppData => self.backup.cancel_backup(),
            Message::ConfirmBackupAppData => {}

            Message::PromptRestoreAppData => self.backup.prompt_restore(),
            Message::CancelRestoreAppData => self.backup.cancel_restore(),
            Message::ConfirmRestoreAppData => {}
            Message::DismissBackupNotice => self.backup.dismiss_notice(),

            Message::DailyScheduleImportClipboard => {}
            Message::DailyScheduleClipboardImported(_) => {}
            Message::DailySchedulePromptClear => self.daily_schedule.prompt_clear(),
            Message::DailyScheduleCancelClear => self.daily_schedule.cancel_clear(),
            Message::DailyScheduleConfirmClear => {}
            Message::DailyScheduleTestSync => self.daily_schedule.test_sync(&self.board, self.preferences.language),
            Message::DailyScheduleSyncFrame => {}
            Message::DailyScheduleDismissNotice => self.daily_schedule.dismiss_notice(),
            Message::DailyScheduleToggleReminderSound => self.daily_schedule.toggle_reminder_sound(),
            Message::DailySchedulePromptAddRule => self.daily_schedule.prompt_add_rule(),
            Message::DailySchedulePromptEditRule(index) => self.daily_schedule.prompt_edit_rule(index),
            Message::DailyScheduleCancelRule => self.daily_schedule.cancel_rule(),
            Message::DailyScheduleSaveRule => self.daily_schedule.save_rule(),
            Message::DailyScheduleDeleteRule(index) => self.daily_schedule.delete_rule(index),
            Message::DailyScheduleRuleKeywordChanged(kw) => self.daily_schedule.rule_keyword_changed(kw),
            Message::DailyScheduleRuleKey(action) => self.daily_schedule.rule_key(action),
            Message::DailyScheduleRuleColorSelected(color) => self.daily_schedule.rule_color_selected(color),
            Message::DailyScheduleRuleClipboardImported(_) => {}

            // A change of page is `update`'s, above.
            Message::Open(_) | Message::Back | Message::Scrolled(_) => {}
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
        self.wifi.is_waiting(self.section)
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
        } else if self.daily_schedule.is_testing_sync {
            iced::window::frames().map(|_| Message::DailyScheduleSyncFrame)
        } else {
            Subscription::none()
        }
    }

    // =========================================================================
    // Opening a page: the one thing left here that is the app's rather than a
    // page's, because it is about which page is up.
    // =========================================================================

    /// Opens `section`, and does what opening it means.
    ///
    /// Only the Wi-Fi page has an opening move: a scan. Coming *back* to it does not re-scan — the
    /// results are still good, and the scan row is there to ask again.
    ///
    /// A page that actually changed opens at the top; re-opening the page already showing leaves it
    /// where it was, which is what the original's `set_section` did.
    fn open(&mut self, section: SettingsSection) -> Task<Message> {
        if self.section == section {
            return Task::none();
        }

        let entering = section == SettingsSection::Wifi;

        self.set_section(section);
        self.set_scroll_offset(section, 0.0);

        if entering {
            self.wifi.start_scan();
        } else if section == SettingsSection::Storage || section == SettingsSection::Backup {
            self.refresh_storage();
        } else if section == SettingsSection::Apps
            || section == SettingsSection::AppDailySchedule
            || section == SettingsSection::AppDailyScheduleUrl
            || section == SettingsSection::AppDailyScheduleColors
        {
            self.refresh_daily_schedule();
        }

        to_top()
    }

    /// Describes the interface for the current state.
    pub fn view(&self) -> UI<'_> {
        use iced::widget::container;
        use iced::Length;
        use pages::{
            apps::{
                apps_page, daily_schedule_colors_page, daily_schedule_page,
                daily_schedule_url_page,
            },
            backup::backup_page, battery::battery_page, language::language_page, main_page::main_page, memory::memory_page,
            storage::storage_page, system::system_page,
            theme::{screen_off_timeout_page, sleep_timeout_page, theme_page},
            time::time_page, wifi::wifi_page,
        };

        // No per-section wrapper any more: the body is one `scrollable` with one id, and `update`
        // sends it back to the top when the page changes. See the module docs.
        let page = match self.section {
            SettingsSection::Main => main_page(
                self.preferences,
                (self.wifi.status().state == WifiState::Connected)
                    .then_some(self.wifi.status().ssid.as_str()),
            ),
            SettingsSection::Wifi => wifi_page(self.preferences, self.wifi.enabled(), &self.wifi),
            SettingsSection::Memory => memory_page(self.preferences),
            SettingsSection::Storage => storage_page(self.preferences, &self.storage),
            SettingsSection::Battery => battery_page(self.preferences, self.battery),
            SettingsSection::Backup => {
                backup_page(self.preferences, self.storage.sd_status, &self.backup)
            }
            SettingsSection::SystemInfo => system_page(self.preferences),
            SettingsSection::Theme => theme_page(self.preferences, &self.theme_state),
            SettingsSection::Time => time_page(self.preferences, self.is_24h_format),
            SettingsSection::Apps => {
                apps_page(self.preferences, self.daily_schedule.config.subscription_url.is_some())
            }
            SettingsSection::AppDailySchedule => {
                daily_schedule_page(self.preferences, &self.daily_schedule)
            }
            SettingsSection::AppDailyScheduleUrl => {
                daily_schedule_url_page(self.preferences, &self.daily_schedule)
            }
            SettingsSection::AppDailyScheduleColors => {
                daily_schedule_colors_page(self.preferences, &self.daily_schedule)
            }
            SettingsSection::ScreenOffTimeout => {
                screen_off_timeout_page(self.preferences)
            }
            SettingsSection::SleepTimeout => {
                sleep_timeout_page(self.preferences)
            }
            SettingsSection::Language => {
                language_page(self.preferences)
            }
        };

        // The surface the whole app sits on. Painted here rather than left to a theme, because a
        // theme is chosen by *whoever owns the loop*: run `settings` on its own and that is this
        // crate's, but on the board the launcher draws this app inside its own tree under its own
        // theme — whose background is the launcher's deep navy — and an app that says nothing about
        // its own background is wearing the launcher's. Every page of this app is designed against
        // black on an AMOLED panel, where an unlit pixel costs nothing. See `style::background_for`.
        let theme = self.preferences.theme;
        let page = page.push_opt(toast_view(&self.toast, theme));

        container(page)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_theme| container::Style {
                background: Some(style::background_for(theme).into()),
                ..container::Style::default()
            })
            .into()
    }
}

impl AppPreferences for Settings {
    fn preferences(&self) -> SystemPreferences {
        self.preferences
    }

    fn set_preferences(&mut self, preferences: SystemPreferences) {
        self.preferences = preferences;
    }
}


// =============================================================================
// All page-builder functions have been moved to src/pages/.
// =============================================================================

