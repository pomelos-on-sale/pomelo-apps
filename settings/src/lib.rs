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
use pomelo_hal::{Board, PersistentConfig, SdCardInfo, SdCardStatus, StorageStats, WifiState};

pub use pomelo_widgets::{FontSizeTier, Language, SleepTimeout, SystemPreferences, ThemeMode};
pub use i18n::{Key, LanguageExt};

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
}

pub(crate) const SECTION_COUNT: usize = 11;

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
    /// The language row was pressed, and the interface is now in `Language`.
    SetLanguage(Language),
    /// Set the appearance theme mode.
    SetTheme(ThemeMode),
    /// Cycle to the next font size tier.
    CycleFontTier,
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
/// from, the battery reading the platform pushes in, and the routing between them. A page's own
/// state and rules live with its widgets — see [`pages::wifi`], the one page that has any.
pub struct Settings {
    section: SettingsSection,
    scroll_offsets: [f32; SECTION_COUNT],
    is_24h_format: bool,
    battery: Battery,
    /// The system preferences (language, theme, font size tier).
    preferences: SystemPreferences,
    /// The Wi-Fi page, which owns the board: it is the one page that talks to hardware rather than
    /// reading a value the platform pushed in.
    wifi: Wifi,
    show_format_confirm: bool,
    is_formatting_sd: bool,
    show_backup_confirm: bool,
    show_restore_confirm: bool,
    is_backing_up: bool,
    is_restoring: bool,
    has_backup: bool,
    backup_notice: Option<(String, String)>,
    sd_status: SdCardStatus,
    sd_stats: Option<StorageStats>,
    sd_info: Option<SdCardInfo>,
    show_sleep_timeout_dialog: bool,
    daily_schedule_config: pages::apps::DailyScheduleConfig,
    show_clear_schedule_confirm: bool,
    is_testing_schedule_sync: bool,
    schedule_sync_notice: Option<(String, String)>,
}

impl Settings {
    /// The app at the main list, the way the original starts, with `board` for the pages that need
    /// hardware.
    pub fn new(board: Arc<Board>) -> Self {
        let preferences = SystemPreferences::load();
        let _ = board.set_sleep_timeout(preferences.sleep_timeout.as_duration());
        let mut settings = Self {
            section: SettingsSection::Main,
            scroll_offsets: [0.0; SECTION_COUNT],
            is_24h_format: true,
            battery: Battery::default(),
            preferences,
            wifi: Wifi::new(board),
            show_format_confirm: false,
            is_formatting_sd: false,
            show_backup_confirm: false,
            show_restore_confirm: false,
            is_backing_up: false,
            is_restoring: false,
            has_backup: false,
            backup_notice: None,
            sd_status: SdCardStatus::NotInserted,
            sd_stats: None,
            sd_info: None,
            show_sleep_timeout_dialog: false,
            daily_schedule_config: pages::apps::DailyScheduleConfig::load(),
            show_clear_schedule_confirm: false,
            is_testing_schedule_sync: false,
            schedule_sync_notice: None,
        };

        // One read at boot, so the switch and the main list's Wi-Fi row are true before the page is
        // ever opened.
        settings.wifi.read();
        settings.refresh_storage();

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

    /// Sets the screen auto-sleep inactivity timeout.
    pub fn set_sleep_timeout(&mut self, timeout: SleepTimeout) {
        self.preferences.sleep_timeout = timeout;
        let _ = self.preferences.save();
        let _ = self.wifi.board().set_sleep_timeout(timeout.as_duration());
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
        if self.show_sleep_timeout_dialog {
            self.show_sleep_timeout_dialog = false;
            return Some(Task::none());
        }

        if self.show_format_confirm {
            self.show_format_confirm = false;
            return Some(Task::none());
        }

        if self.show_backup_confirm {
            self.show_backup_confirm = false;
            return Some(Task::none());
        }

        if self.show_restore_confirm {
            self.show_restore_confirm = false;
            return Some(Task::none());
        }

        if self.show_clear_schedule_confirm {
            self.show_clear_schedule_confirm = false;
            return Some(Task::none());
        }

        if self.schedule_sync_notice.is_some() {
            self.schedule_sync_notice = None;
            return Some(Task::none());
        }

        if self.backup_notice.is_some() {
            self.backup_notice = None;
            return Some(Task::none());
        }

        if self.wifi.has_prompt() {
            self.wifi.cancel_prompt();

            // The prompt sits over the page; the body under it did not move.
            return Some(Task::none());
        }

        if self.section == SettingsSection::AppDailySchedule {
            if self.is_testing_schedule_sync {
                let _ = self.wifi.board().http().cancel();
                self.is_testing_schedule_sync = false;
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
        let storage = self.wifi.board().storage();
        self.sd_status = storage.sd_status();
        self.sd_stats = storage.sd_stats().ok();
        self.sd_info = storage.sd_card_info();
        self.has_backup = storage.has_backup();
    }

    /// Refreshes the daily schedule config from disk.
    pub fn refresh_daily_schedule(&mut self) {
        self.daily_schedule_config = pages::apps::DailyScheduleConfig::load();
    }

    /// The current daily schedule configuration.
    pub fn daily_schedule_config(&self) -> &pages::apps::DailyScheduleConfig {
        &self.daily_schedule_config
    }

    /// Whether an asynchronous schedule sync test is currently underway.
    pub fn is_testing_schedule_sync(&self) -> bool {
        self.is_testing_schedule_sync
    }

    /// The current active schedule sync notice, if any.
    pub fn schedule_sync_notice(&self) -> Option<&(String, String)> {
        self.schedule_sync_notice.as_ref()
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
            Message::PromptSleepTimeout => self.show_sleep_timeout_dialog = true,
            Message::CancelSleepTimeout => self.show_sleep_timeout_dialog = false,
            Message::CycleSleepTimeout => {
                self.set_sleep_timeout(self.preferences.sleep_timeout.cycle());
            }
            Message::SetSleepTimeout(timeout) => {
                self.show_sleep_timeout_dialog = false;
                self.set_sleep_timeout(timeout);
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
            Message::PromptFormatSd => self.show_format_confirm = true,
            Message::CancelFormatSd => self.show_format_confirm = false,
            Message::ConfirmFormatSd => {
                self.show_format_confirm = false;
                self.is_formatting_sd = true;
                let _ = self.wifi.board().storage().format_sd();
                self.is_formatting_sd = false;
                self.refresh_storage();
            }
            Message::PromptBackupAppData => self.show_backup_confirm = true,
            Message::CancelBackupAppData => self.show_backup_confirm = false,
            Message::ConfirmBackupAppData => {
                self.show_backup_confirm = false;
                self.is_backing_up = true;
                let res = self.wifi.board().storage().backup_app_data();
                self.is_backing_up = false;
                self.refresh_storage();
                let lang = self.preferences.language;
                match res {
                    Ok(stats) => {
                        let msg = format!(
                            "{} ({} files, {:.1} KB)",
                            lang.text(Key::BackupSuccess),
                            stats.files_count,
                            stats.bytes_count as f64 / 1024.0
                        );
                        self.backup_notice = Some((lang.text(Key::BackupSuccess).to_string(), msg));
                    }
                    Err(e) => {
                        let msg = format!("{}: {}", lang.text(Key::BackupFailed), e);
                        self.backup_notice = Some((lang.text(Key::BackupFailed).to_string(), msg));
                    }
                }
            }
            Message::PromptRestoreAppData => self.show_restore_confirm = true,
            Message::CancelRestoreAppData => self.show_restore_confirm = false,
            Message::ConfirmRestoreAppData => {
                self.show_restore_confirm = false;
                self.is_restoring = true;
                let res = self.wifi.board().storage().restore_app_data();
                self.is_restoring = false;
                self.refresh_storage();
                let lang = self.preferences.language;
                match res {
                    Ok(_) => {
                        self.backup_notice = Some((
                            lang.text(Key::RestoreSuccess).to_string(),
                            lang.text(Key::RestoreSuccess).to_string(),
                        ));
                    }
                    Err(e) => {
                        let msg = format!("{}: {}", lang.text(Key::RestoreFailed), e);
                        self.backup_notice = Some((lang.text(Key::RestoreFailed).to_string(), msg));
                    }
                }
            }
            Message::DismissBackupNotice => self.backup_notice = None,

            Message::DailyScheduleImportClipboard => {}
            Message::DailyScheduleClipboardImported(content) => {
                let clipboard = content.unwrap_or_default();
                if let Some(url) = pages::apps::sanitize_calendar_url(&clipboard) {
                    self.daily_schedule_config.subscription_url = Some(url);
                    let lang = self.preferences.language;
                    let err_title = if lang == Language::English { "Import Failed" } else { "导入失败" };
                    if let Err(e) = self.daily_schedule_config.save() {
                        self.schedule_sync_notice = Some((
                            err_title.to_string(),
                            format!("{}: {}", err_title, e),
                        ));
                    } else {
                        self.schedule_sync_notice = Some((
                            lang.text(Key::LinkImported).to_string(),
                            lang.text(Key::LinkImported).to_string(),
                        ));
                    }
                } else {
                    let lang = self.preferences.language;
                    self.schedule_sync_notice = Some((
                        lang.text(Key::ClipboardInvalid).to_string(),
                        lang.text(Key::ClipboardInvalid).to_string(),
                    ));
                }
            }
            Message::DailySchedulePromptClear => {
                self.show_clear_schedule_confirm = true;
            }
            Message::DailyScheduleCancelClear => {
                self.show_clear_schedule_confirm = false;
            }
            Message::DailyScheduleConfirmClear => {
                self.show_clear_schedule_confirm = false;
                self.daily_schedule_config.subscription_url = None;
                let _ = self.daily_schedule_config.save();
                let cache_path = pomelo_hal::storage::app_file_path("DAILY_SCHEDULE", "today_cache.toml");
                let _ = std::fs::remove_file(cache_path);
            }
            Message::DailyScheduleTestSync => {
                if let Some(url) = &self.daily_schedule_config.subscription_url {
                    let mut clean_url = url.trim().to_string();
                    if clean_url.starts_with("webcal://") {
                        clean_url = format!("https://{}", &clean_url[9..]);
                    }
                    let res = self.wifi.board().http().start(&clean_url);
                    let lang = self.preferences.language;
                    match res {
                        Ok(()) => {
                            self.is_testing_schedule_sync = true;
                        }
                        Err(e) => {
                            self.is_testing_schedule_sync = false;
                            self.schedule_sync_notice = Some((
                                lang.text(Key::SyncFailed).to_string(),
                                format!("{}: {:?}", lang.text(Key::SyncFailed), e),
                            ));
                        }
                    }
                }
            }
            Message::DailyScheduleSyncFrame => {
                if !self.is_testing_schedule_sync {
                    return;
                }
                let mut http = self.wifi.board().http();
                match http.state() {
                    pomelo_hal::HttpState::Running { .. } => {}
                    pomelo_hal::HttpState::Done => {
                        self.is_testing_schedule_sync = false;
                        let lang = self.preferences.language;
                        let status_code = http.status_code().unwrap_or(200);
                        if (200..300).contains(&status_code) {
                            let payload = http.take_result().unwrap_or_default();
                            let title = lang.text(Key::SyncSuccess).to_string();
                            let msg = if lang == Language::English {
                                format!("Sync verified (HTTP {}, {} bytes)", status_code, payload.len())
                            } else {
                                format!("同步验证成功（HTTP {}, 共 {} 字节）", status_code, payload.len())
                            };
                            self.schedule_sync_notice = Some((title, msg));
                        } else {
                            let _ = http.take_result();
                            let title = lang.text(Key::SyncFailed).to_string();
                            let msg = if lang == Language::English {
                                format!("HTTP Error: Status {}", status_code)
                            } else {
                                format!("HTTP 请求失败: 状态码 {}", status_code)
                            };
                            self.schedule_sync_notice = Some((title, msg));
                        }
                    }
                    pomelo_hal::HttpState::Error => {
                        self.is_testing_schedule_sync = false;
                        let lang = self.preferences.language;
                        let code_str = http.status_code().map(|c| format!(" (HTTP {})", c)).unwrap_or_default();
                        let _ = http.take_result();
                        self.schedule_sync_notice = Some((
                            lang.text(Key::SyncFailed).to_string(),
                            format!("{}{}", lang.text(Key::SyncFailed), code_str),
                        ));
                    }
                    pomelo_hal::HttpState::Idle => {
                        self.is_testing_schedule_sync = false;
                    }
                }
            }
            Message::DailyScheduleDismissNotice => {
                self.schedule_sync_notice = None;
            }

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
        } else if self.is_testing_schedule_sync {
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
        } else if section == SettingsSection::Apps || section == SettingsSection::AppDailySchedule {
            self.refresh_daily_schedule();
        }

        to_top()
    }

    /// Describes the interface for the current state.
    pub fn view(&self) -> UI<'_> {
        use iced::widget::container;
        use iced::Length;
        use pages::{
            apps::{apps_page, daily_schedule_page},
            backup::backup_page, battery::battery_page, main_page::main_page, memory::memory_page,
            storage::storage_page, system::system_page, theme::theme_page, time::time_page,
            wifi::wifi_page,
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
            SettingsSection::Storage => {
                storage_page(
                    self.preferences,
                    self.sd_status,
                    self.sd_stats,
                    self.sd_info.clone(),
                    self.is_formatting_sd,
                    self.show_format_confirm,
                )
            }
            SettingsSection::Battery => battery_page(self.preferences, self.battery),
            SettingsSection::Backup => {
                backup_page(
                    self.preferences,
                    self.sd_status,
                    self.has_backup,
                    self.is_backing_up,
                    self.is_restoring,
                    self.show_backup_confirm,
                    self.show_restore_confirm,
                    self.backup_notice.as_ref(),
                )
            }
            SettingsSection::SystemInfo => system_page(self.preferences),
            SettingsSection::Theme => {
                theme_page(self.preferences, self.show_sleep_timeout_dialog)
            }
            SettingsSection::Time => time_page(self.preferences, self.is_24h_format),
            SettingsSection::Apps => {
                apps_page(self.preferences, self.daily_schedule_config.subscription_url.is_some())
            }
            SettingsSection::AppDailySchedule => {
                daily_schedule_page(
                    self.preferences,
                    &self.daily_schedule_config,
                    self.is_testing_schedule_sync,
                    self.show_clear_schedule_confirm,
                    self.schedule_sync_notice.as_ref(),
                )
            }
        };

        // The surface the whole app sits on. Painted here rather than left to a theme, because a
        // theme is chosen by *whoever owns the loop*: run `settings` on its own and that is this
        // crate's, but on the board the launcher draws this app inside its own tree under its own
        // theme — whose background is the launcher's deep navy — and an app that says nothing about
        // its own background is wearing the launcher's. Every page of this app is designed against
        // black on an AMOLED panel, where an unlit pixel costs nothing. See `style::background_for`.
        let theme = self.preferences.theme;

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

// =============================================================================
// All page-builder functions have been moved to src/pages/.
// =============================================================================

