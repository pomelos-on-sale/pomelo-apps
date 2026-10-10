//! The Wi-Fi page: radio switch, scan results, connection card, and password prompt.

pub(crate) mod list;
pub(crate) mod prompt;

pub(crate) use list::*;
pub(crate) use prompt::*;

use std::sync::Arc;
use std::time::Duration;

use iced::time::Instant;
use iced::widget::stack;
use pomelo_hal::wifi_credentials::WifiCredentials;
use pomelo_hal::{ApInfo, Board, ScanState, SystemEvent, WifiState, WifiStatus};
use pomelo_material_symbols::Icon;
use pomelo_widgets::touch_keyboard::{KeyAction, KeyboardMode};
use pomelo_widgets::SystemPreferences;

use crate::i18n::{Key, LanguageExt as _};
use crate::pages::card::Header;
use crate::pages::common::{body, notice_card, page, switch_row, text_button, toggle, UI};
use crate::{Message, SettingsSection};

/// How often an open Wi-Fi page reads the radio.
///
/// The page is told about every frame while it is up (see `Settings::subscription`) and reads the
/// board this often. A read is a mutex and a few atomics; four times a second is quicker than a
/// finger and far slower than the board's own loop.
const READ_EVERY: Duration = Duration::from_millis(250);

/// The Wi-Fi page: its state and its rules, next to the widgets that draw it.
///
/// This is the only page with a conversation of its own — the radio is asked, waited on, and asked
/// again — so it is the only page that owns state. The board it talks to lives here rather than on
/// `Settings`, which is what the rest of the app needs none of.
pub struct Wifi {
    /// The radio. The one hardware this page talks to, and the reason it holds anything at all.
    board: Arc<Board>,
    /// Whether the radio is on. Mirrors the board; the page's switch is what changes it.
    enabled: bool,
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
    /// What that connect asked for, held until the radio answers.
    ///
    /// The password lives here and not in the HAL because the file is the app's: the app asked for
    /// the connection, so the app is what knows to write it when one comes up.
    attempt: Option<WifiCredentials>,
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
    pub(crate) fn new(board: Arc<Board>) -> Self {
        let enabled = board.wifi().is_enabled()
            || board.wifi().saved().map(|s| s.enabled).unwrap_or(false);
        Self {
            board,
            enabled,
            access_points: Vec::new(),
            scan: ScanState::Idle,
            status: WifiStatus::default(),
            prompt: None,
            password: String::new(),
            keyboard: KeyboardMode::Lower,
            revealed: false,
            pending: false,
            attempt: None,
            failed: false,
            read_at: None,
            taken: false,
        }
    }

    // =========================================================================
    // What the host reads
    // =========================================================================

    /// Whether the radio is on.
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// What the last scan found, best signal first.
    pub fn access_points(&self) -> &[ApInfo] {
        &self.access_points
    }

    /// The connection as of the last read.
    pub fn status(&self) -> &WifiStatus {
        &self.status
    }

    /// The network the password prompt is open for, if it is open.
    pub fn prompt(&self) -> Option<&ApInfo> {
        self.prompt
            .and_then(|index| self.access_points.get(index))
    }

    /// Whether a prompt is open at all.
    pub(crate) fn has_prompt(&self) -> bool {
        self.prompt.is_some()
    }

    /// What has been typed into the prompt.
    pub fn password(&self) -> &str {
        &self.password
    }

    /// Whether the prompt is drawing the password in the clear.
    pub fn revealed(&self) -> bool {
        self.revealed
    }

    /// Whether this page is waiting on the radio for something only the radio can say.
    ///
    /// A scan is in flight, or a finished scan's results have not been taken yet, or a connection
    /// is being waited for. Nothing else needs the loop: the page redraws when a finger touches it.
    pub(crate) fn is_waiting(&self, section: SettingsSection) -> bool {
        if section != SettingsSection::Wifi || !self.enabled {
            return false;
        }

        let waiting_for_results = self.scan == ScanState::Done && !self.taken;

        // `ScanState::Error` is deliberately not "waiting": nothing will arrive to stop it, and a
        // subscription that never ends is the bug this method exists to prevent.
        self.scan == ScanState::Scanning || waiting_for_results || self.pending
    }

    // =========================================================================
    // The rules: one method per message, and every one of them asks the board
    // rather than guessing what it said.
    // =========================================================================

    /// Flips the radio's switch, and asks for a scan when it goes on.
    pub(crate) fn toggle(&mut self) {
        let on = !self.enabled;

        if self.board.wifi().set_enabled(on).is_err() {
            return;
        }

        self.enabled = on;
        self.remember_switch();
        self.board
            .emit_event(SystemEvent::WifiStatusChanged(self.board.wifi().status()));

        if on {
            self.start_scan();
        } else {
            // Nothing to list and nothing to connect to: the page says so.
            self.access_points.clear();
            self.scan = ScanState::Idle;
            self.taken = false;
            self.cancel_prompt();
            self.read();
        }
    }

    /// Writes the switch choice down to persistent storage.
    ///
    /// Even if no network has been configured yet, the radio enable/disable choice is
    /// independently remembered so the board respects the user's preference on next boot.
    fn remember_switch(&self) {
        let mut wifi = self.board.wifi();

        let saved = match wifi.saved() {
            Some(mut s) => {
                if s.enabled == self.enabled {
                    return;
                }
                s.enabled = self.enabled;
                s
            }
            None => WifiCredentials {
                enabled: self.enabled,
                ssid: String::new(),
                password: String::new(),
                autoconnect: true,
            },
        };

        if let Err(error) = wifi.remember(&saved) {
            eprintln!(
                "[wifi] the radio is {}, but the file says otherwise: {error}",
                self.enabled
            );
        }
    }

    /// Asks the radio for a scan.
    pub(crate) fn start_scan(&mut self) {
        if !self.enabled {
            return;
        }

        if self.board.wifi().scan_start().is_err() {
            self.scan = ScanState::Error;
            return;
        }

        self.scan = ScanState::Scanning;
        self.taken = false;
        self.read_at = None;
    }

    /// Reads the radio once, copying what it says into the page's state.
    pub(crate) fn read(&mut self) {
        let wifi = self.board.wifi();

        self.enabled = wifi.is_enabled()
            || wifi.saved().map(|s| s.enabled).unwrap_or(false);
        self.scan = wifi.scan_state();

        // `Done` stays `Done` until the next scan starts, so the list is taken once rather than
        // rebuilt four times a second for as long as the page is open.
        if self.scan == ScanState::Done && !self.taken {
            if let Ok(found) = wifi.scan_results() {
                let mut found = found;
                // Best signal first: the network a finger is looking for is at the top.
                found.sort_by_key(|ap| std::cmp::Reverse(ap.rssi));
                self.access_points = found;
                self.taken = true;
            }
        }

        self.status = wifi.status();
    }

    /// A frame arrived while the page is open: read the radio, a few times a second.
    pub(crate) fn poll(&mut self, now: Instant) {
        let due = match self.read_at {
            Some(last) => now.duration_since(last) >= READ_EVERY,
            None => true,
        };

        if !due {
            return;
        }

        self.read_at = Some(now);
        self.read();

        // A prompt that was waiting on the radio: the connection either came up, or it did not.
        if self.pending {
            eprintln!("[wifi] waiting — {}", self.answer());

            match self.status.state {
                WifiState::Connected => self.connected(),
                WifiState::Disconnected => {
                    // The driver clears the status when it gives up, so `status.ssid` is empty by the
                    // time this line runs and naming it that way printed `""`. The name comes from
                    // the list the attempt was made from — and while `pending` is set, the prompt is
                    // what says which entry that was.
                    let ssid = self
                        .prompt
                        .and_then(|index| self.access_points.get(index))
                        .map(|ap| ap.ssid.clone());

                    eprintln!("[wifi] gave up waiting for {ssid:?} — {}", self.answer());
                    self.pending = false;
                    self.attempt = None;
                    self.failed = true;
                }
                _ => {}
            }
        }
    }

    /// A network was pressed: connect to it, or ask for its password first.
    pub(crate) fn select(&mut self, index: usize) {
        let Some(ap) = self.access_points.get(index) else {
            return;
        };

        let (ssid, secure) = (ap.ssid.clone(), ap.secure);

        if !secure {
            self.request_connect(&ssid, "");
            return;
        }

        // A prompt for a network starts empty, whatever was typed for the last one — and hidden
        // again: a password left in the clear is not a state to inherit.
        self.password.clear();
        self.keyboard = KeyboardMode::Lower;
        self.revealed = false;
        self.failed = false;
        self.pending = false;
        self.attempt = None;
        self.prompt = Some(index);
    }

    /// Sets the password string from the text input widget.
    pub(crate) fn set_password(&mut self, password: String) {
        self.password = password;
    }

    /// A key of the prompt's keyboard.
    pub(crate) fn key(&mut self, action: KeyAction) {
        match action {
            KeyAction::Char(character) => self.password.push(character),
            KeyAction::Space => self.password.push(' '),
            KeyAction::Backspace => {
                self.password.pop();
            }
            KeyAction::SwitchMode(mode) => self.keyboard = mode,
            KeyAction::Enter => self.connect(),
        }
    }

    /// The prompt's connect button, and its return key.
    pub(crate) fn connect(&mut self) {
        let Some(index) = self.prompt else {
            return;
        };
        let Some(ssid) = self.access_points.get(index).map(|ap| ap.ssid.clone()) else {
            return;
        };

        let password = self.password.clone();

        self.request_connect(&ssid, &password);
    }

    /// Asks the radio to connect, and says what happened.
    fn request_connect(&mut self, ssid: &str, password: &str) {
        self.failed = false;

        // The length and not the password. The password itself was printed while the authmode
        // threshold was being chased — the question then was "did the box hold what was typed", and
        // a length cannot answer it. That question is answered, and what is left is the only secret
        // this app ever holds; an `eprintln!` on the board goes out on the UART, and a log that
        // travels ends up in a bug report.
        eprintln!(
            "[wifi] connect({ssid:?}, {} chars, prompt={})",
            password.chars().count(),
            self.prompt.is_some()
        );

        // The radio's answer is taken as a value: the borrow that asks for it lasts as long as the
        // call, and `read` below needs the board back.
        let asked = self.board.wifi().connect(ssid, password);

        if let Err(error) = asked {
            // Refused before it tried: no password on a locked network, or a network it does not
            // know. A prompt that is open is the thing that can say so; a tap on an open network
            // has nothing on screen to explain, so the list stands as it was.
            eprintln!("[wifi] refused before trying: {error:?}");
            self.read();
            eprintln!("[wifi] after the refusal — {}", self.answer());
            self.failed = self.prompt.is_some();
            return;
        }

        // Held until the radio answers, and written only if it answers well. Asked for by hand, so
        // it is wanted: connecting to a network once and then not offering it at boot would be a
        // strange thing to have remembered.
        self.attempt = Some(WifiCredentials {
            enabled: true,
            ssid: ssid.to_string(),
            password: password.to_string(),
            autoconnect: true,
        });

        self.pending = self.prompt.is_some();
        self.read();
        eprintln!(
            "[wifi] asked, pending={} — {}",
            self.pending,
            self.answer()
        );

        if self.pending && self.status.state == WifiState::Connected {
            // A board that connects in one step (the simulator does) is done in this message.
            self.connected();
        }
    }

    /// The radio's answer, as one line for the log.
    fn answer(&self) -> String {
        let status = &self.status;
        format!(
            "state={:?} ssid={:?} ip={:?} gateway={:?}",
            status.state, status.ssid, status.ip, status.gateway
        )
    }

    /// The connection came up: the prompt has nothing left to ask.
    fn connected(&mut self) {
        self.pending = false;
        self.failed = false;
        self.prompt = None;
        self.password.clear();
        self.keyboard = KeyboardMode::Lower;
        self.revealed = false;

        self.read();

        // The one moment the file is written: the radio said the password worked, so the password is
        // worth keeping. Written when it was *typed* instead, a mistyped one would destroy the one
        // that worked — which is what the NVS copy this replaces did.
        if let Some(credentials) = self.attempt.take() {
            if let Err(error) = self.board.wifi().remember(&credentials) {
                eprintln!("[wifi] connected, but the file could not be written: {error}");
            }
        }

        eprintln!("[wifi] connected — {}", self.answer());
    }

    /// Closes the prompt without connecting.
    pub(crate) fn cancel_prompt(&mut self) {
        self.prompt = None;
        self.password.clear();
        self.keyboard = KeyboardMode::Lower;
        self.revealed = false;
        self.failed = false;
        self.pending = false;
        self.attempt = None;
    }

    /// Leaves the network we are on.
    pub(crate) fn disconnect(&mut self) {
        let _ = self.board.wifi().disconnect();

        self.read();
    }

    /// Draws what has been typed in the clear, or as dots again.
    pub(crate) fn reveal(&mut self) {
        self.revealed = !self.revealed;
    }
}

/// The Wi-Fi page: the radio's switch, the scan, what it found, and the connection.
///
/// The password prompt is *not* part of the body: it is a `stack` over the whole page, because a
/// prompt is a modal — the list behind it must not take the finger. See [`password_prompt`].
pub(crate) fn wifi_page<'a>(preferences: SystemPreferences, on: bool, wifi: &Wifi) -> UI<'a> {
    let language = preferences.language;
    let theme = preferences.theme;
    // Built here and not twice: the page has two exits — radio off, and radio on — and both of them
    // are the same page with a different body under the same head.
    let head = Header::section(
        Icon::WIFI,
        SettingsSection::Wifi,
        language.text(Key::Wifi),
    )
    .view(theme);
    let mut parts: Vec<UI<'a>> = vec![switch_row(
        language.text(Key::Wifi),
        toggle(on, Message::ToggleWifi, theme),
        theme,
    )];

    if !on {
        parts.push(notice_card(language.text(Key::WifiOff), theme));

        return page(head, body(parts));
    }

    // What the radio is doing, and the way to ask for another one. A text button and not a card: a
    // card would give "scan again" the weight of the rows above and below it, and it is not a
    // setting — it is a request.
    let scanning = wifi.scan == ScanState::Scanning;
    parts.push(text_button(
        language.text(if scanning {
            Key::Scanning
        } else {
            Key::ScanAgain
        }),
        if scanning { None } else { Some(Message::WifiScan) },
        theme,
    ));

    parts.push(network_list(language, wifi, theme));

    if wifi.status.state == WifiState::Connected {
        parts.push(connection_card(language, &wifi.status, theme));
    }

    let pg = page(head, body(parts));

    // The prompt is not part of the page; it is a layer *over* it. One `Stack` for the two of them,
    // and the prompt's layer on top — see `password_prompt`.
    match password_prompt(preferences, wifi) {
        Some(prompt) => stack![pg, prompt].into(),
        None => pg,
    }
}
