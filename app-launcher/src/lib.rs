//! The launcher, built from iced widgets — **a standard iced program**.
//!
//! Deliberately free of raster images: iced's `image` feature pulls the `image` crate into the
//! firmware, and the wallpaper and icons we already bake are RGB565 blits that `pomelo-gfx` does
//! natively. So the wallpaper is a gradient and every picture on the screen is a glyph out of
//! `pomelo_material_symbols` — the grid's on an accent square, the status bar's on its black band —
//! one font of the whole Material Symbols catalogue, which costs nothing beyond the text this
//! launcher draws anyway.
//!
//! # Hosting an app is merging its subscription
//!
//! The six apps are hosted as **widgets**: this launcher calls their `view` and `update` itself, so
//! their state is its state. What a widget cannot carry is an app's *subscriptions* — the two that
//! animate (`hello`, `music-player`) and the one whose *layout* follows the screen (`terminal`) all
//! express that as a `Subscription`, and a subscription belongs to whoever owns the loop.
//!
//! So the launcher, which owns the loop for its children as far as they are concerned, says in
//! [`Launcher::subscription`] which of them is alive: the one on screen, and nothing else. That is
//! the declarative form of "only the app you are looking at animates" — off screen, an app is not
//! subscribed, so it costs no frames at all.
//!
//! # The screen's size is the platform's to know
//!
//! Nothing here may assume what size it is drawing into: the grid's pages, the pager's thresholds
//! and the terminal's band are all the screen's size, and the screen is a window on a desktop and a
//! panel on the board. So the launcher subscribes to `window::resize_events()` *always* — it is an
//! event and not a clock, so it costs no frames while nothing changes — and hands what it hears to
//! the hosted app whose layout depends on it ([`Launcher::hand_over_size`]). An app opened later was
//! not on screen when the platform announced the size at boot, which is why the hand-off exists at
//! all.
//!
//! # Where the board comes from
//!
//! [`program`] takes the board its apps share, and that board is the composition root's: the
//! firmware injects the ESP32-S3 one, this project's `main.rs` the HAL's desktop simulator. Nothing
//! here decides which hardware this is — the same argument as the music player's.
//!
//! # The grid is paged, and the pager is this file's
//!
//! A page is [`PER_PAGE`] apps, one to a quadrant, so the six the catalogue has are two pages. A
//! finger turning one is a **drag**: down on the grid, across, up. iced has no widget for that — a
//! `button` captures the press it is given, and `Scrollable` scrolls for a wheel, a touch or its own
//! scrollbar and for nothing else — so the gesture is read here.
//!
//! The drag decides a turn and the finger leaving *is* the turn: past halfway the page changes, and
//! short of it nothing moved. There is no slide, for the same reason there is never a second page on
//! screen: a page that slid would need two pages and the frames to move them, and a page change here
//! is one frame — the one after the release.
//!
//! Key layout notes:
//!
//! * a page evenly distributes apps across rows and columns using flexible spaces;
//! * only the app's icon is the pressable touch target, leaving surrounding spaces for pager swipe gestures.

mod apps;
mod icon;
mod status;
mod style;
mod subscription;
mod view;
pub mod tasks;

#[cfg(test)]
mod tests;

use std::sync::Arc;

use iced::theme::Palette;
use iced::{Color, Size, Task, Theme};

use calculator::Calculator;
use demo_counter::Counter;
use hello::Hello;
use music_player::Player;
use pomelo_hal::Board;
use settings::Settings;
use terminal::Terminal;

use subscription::current_time_info;

pub use apps::{Entry, CATALOGUE};
pub use pomelo_material_symbols::Icon;
pub use pomelo_widgets::{AppIcon, BitmapIcon, FontSizeTier, Language, SystemPreferences, ThemeMode};
pub use style::{
    DOT_REST, DOT_UP, LABEL, PER_PAGE, SCREEN, STATUS_BG, STATUS_BG_DARK, STATUS_BG_LIGHT,
    STATUS_HEIGHT, STATUS_INSET,
};
pub use tasks::{SystemTaskId, SystemTaskMessage, TaskManager, TaskStatus};

/// The launcher as an iced program, with `board` for its apps.
///
/// The result is iced's own `Application`, which is both a builder — `main.rs` calls `run()` on it —
/// and a [`Program`](iced::Program), so the platform can run it directly (`app_launcher::program(board).run()`).
/// One definition, the same wiring in both places: the subject of the launcher is not a *shape* of
/// program, it is these four functions.
///
/// Text draws with the platform's default font — a Simplified-Chinese subset of Source Han Sans, installed by the
/// host unless an app installs one of its own first; see `pomelo_iced_host::fonts`. The icon font is
/// a second face on top of that, and it is installed here, through iced's own channel: the settings
/// travel with the program, so the desktop window and the board's loop both get it, and neither
/// needs to know that a widget wanted it.
pub fn program(
    board: Arc<Board>,
) -> iced::Application<impl iced::Program<State = Launcher, Message = Message, Theme = Theme>> {
    iced::application(
        move || Launcher::boot(Arc::clone(&board)),
        Launcher::update,
        Launcher::view,
    )
    .font(pomelo_material_symbols::FONT)
    .theme(Launcher::theme)
    .subscription(Launcher::subscription)
}

/// Which screen is up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Grid,
    App(usize),
}

/// The entries in [`CATALOGUE`] that have an iced app behind them.
///
/// Indices, because that is what the grid hands over when a tile is tapped. The tests assert that
/// each of these still names the app it says it does, and every entry has one: the catalogue and
/// the list of apps are the same six things.
pub const TERMINAL: usize = 0;
pub const CALCULATOR: usize = 1;
pub const DEMO_COUNTER: usize = 2;
pub const COUNTER: usize = DEMO_COUNTER;
pub const HELLO: usize = 3;
pub const SETTINGS: usize = 4;
pub const MUSIC: usize = 5;

/// What the launcher reacts to.
///
/// No longer `Eq`: the terminal's own `Message` carries a `Size`, and a size is a pair of floats.
#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    /// An icon was tapped.
    Open(usize),
    /// The paged grid turned to `page`.
    PageChanged(usize),
    /// The back button, or hardware button 1 (moves to background).
    Back,
    /// The exit / kill button, or hardware button 2 (kills app and frees memory).
    Exit,
    /// The status bar's readings, driven by the [`Subscription`].
    ///
    /// Produced periodically by [`status_stream`] and whenever underlying
    /// hardware events occur (battery level, charging state, Wi-Fi status changes, clock minute updates).
    Status(String, u8, bool, u8),
    /// The screen changed size: a window on a desktop, the panel on the board.
    Resized(Size),
    /// System background task status notification.
    SystemTask(tasks::SystemTaskMessage),
    /// A message from one of the apps this launcher hosts.
    Calculator(calculator::Message),
    Counter(demo_counter::Message),
    Hello(hello::Message),
    Settings(settings::Message),
    Music(music_player::Message),
    Terminal(terminal::Message),
}

/// The launcher.
pub struct Launcher {
    screen: Screen,
    calculator: Option<Calculator>,
    counter: Option<Counter>,
    hello: Option<Hello>,
    settings: Option<Settings>,
    music: Option<Player>,
    terminal: Option<Terminal>,
    /// Apps currently running in memory (foreground or background).
    running_apps: Vec<usize>,
    board: Arc<Board>,
    /// The screen's size, as the platform last reported it. The grid's pages and the pager's
    /// thresholds are laid out for it, and it is what the apps whose layout depends on the size are
    /// told — see [`Launcher::hand_over_size`]. Everything else here fills whatever it is given.
    size: Size,
    /// Which page of the grid is currently up.
    page: usize,
    clock: String,
    battery: u8,
    charging: bool,
    wifi: u8,
    preferences: SystemPreferences,
    task_manager: TaskManager,
}

impl Launcher {
    /// The launcher and every app it hosts, all sharing `board`.
    ///
    /// Hosted apps are lazily loaded on demand when first opened, rather than during boot,
    /// eliminating startup CPU overhead, track scanning, and initial heap allocations.
    pub fn new(board: Arc<Board>) -> Self {
        // The status bar starts from what the board says, and is continually updated in real-time
        // via Subscription (see [`status_stream`]).
        let (clock, _) = current_time_info();
        let battery = board.power().battery_percent().unwrap_or(0);
        let charging = board.power().is_charging().unwrap_or(false);
        let signal = board.wifi().status().signal_bars();

        Self {
            screen: Screen::Grid,
            calculator: None,
            counter: None,
            hello: None,
            settings: None,
            music: None,
            terminal: None,
            running_apps: Vec::new(),
            board,
            // What the screen is until the platform says: the design panel. The first `Resized`
            // always arrives — the window manager's on a desktop, the host's first iteration on the
            // board — so this is what at most one frame is drawn from, and never what a layout is
            // decided by. A launcher is *told* how big the screen is; it may not assume it.
            size: Size::new(SCREEN as f32, SCREEN as f32),
            page: 0,
            clock,
            battery,
            charging,
            wifi: signal,
            preferences: SystemPreferences::default(),
            task_manager: TaskManager::new(),
        }
    }

    /// Initializes the launcher state and launches initial boot tasks (e.g. Wi-Fi autoconnect).
    pub fn boot(board: Arc<Board>) -> (Self, Task<Message>) {
        let mut launcher = Self::new(Arc::clone(&board));
        launcher
            .task_manager
            .set_status(tasks::SystemTaskId::WifiAutoConnect, tasks::TaskStatus::Running);
        let boot_task = tasks::perform(board, tasks::SystemTaskId::WifiAutoConnect);
        (launcher, boot_task)
    }

    /// Access the system task manager.
    pub fn task_manager(&self) -> &TaskManager {
        &self.task_manager
    }

    /// The screen the launcher is laying out for, as the platform last reported it.
    ///
    /// Read by the host and by the tests. It is the platform's number and never a constant: the
    /// grid's pages and the pager's thresholds are this width, and the apps whose layout follows the
    /// screen are handed it — see [`Launcher::hand_over_size`].
    pub fn screen_size(&self) -> Size {
        self.size
    }

    /// The active system preferences.
    pub fn preferences(&self) -> SystemPreferences {
        self.preferences
    }

    /// Sets the active system preferences.
    pub fn set_preferences(&mut self, preferences: SystemPreferences) {
        self.preferences = preferences;
        self.propagate_preferences();
    }

    /// Propagates the active preferences to all loaded apps.
    fn propagate_preferences(&mut self) {
        let prefs = self.preferences;
        if let Some(app) = &mut self.calculator {
            app.set_preferences(prefs);
        }
        if let Some(app) = &mut self.counter {
            app.set_preferences(prefs);
        }
        if let Some(app) = &mut self.hello {
            app.set_preferences(prefs);
        }
        if let Some(app) = &mut self.music {
            app.set_preferences(prefs);
        }
        if let Some(app) = &mut self.settings {
            app.set_preferences(prefs);
        }
        if let Some(app) = &mut self.terminal {
            app.set_preferences(prefs);
        }
    }

    /// The system language, read from active preferences.
    pub fn language(&self) -> Language {
        self.preferences.language
    }

    /// Sets the system language.
    pub fn set_language(&mut self, language: Language) {
        self.preferences.language = language;
        self.propagate_preferences();
    }

    pub fn get_or_create_calculator(&mut self) -> &mut Calculator {
        if self.calculator.is_none() {
            let mut app = Calculator::new();
            app.set_preferences(self.preferences);
            self.calculator = Some(app);
        }
        self.calculator.as_mut().unwrap()
    }

    pub fn get_or_create_counter(&mut self) -> &mut Counter {
        if self.counter.is_none() {
            let mut app = Counter::new();
            app.set_preferences(self.preferences);
            self.counter = Some(app);
        }
        self.counter.as_mut().unwrap()
    }

    pub fn get_or_create_hello(&mut self) -> &mut Hello {
        if self.hello.is_none() {
            let mut app = Hello::new();
            app.set_preferences(self.preferences);
            self.hello = Some(app);
        }
        self.hello.as_mut().unwrap()
    }

    pub fn get_or_create_settings(&mut self) -> &mut Settings {
        if self.settings.is_none() {
            let mut app = Settings::new(Arc::clone(&self.board));
            app.set_preferences(self.preferences);
            let voltage_mv = self.board.power().battery_voltage_mv().unwrap_or(0) as u16;
            app.set_battery(settings::Battery {
                percent: self.battery,
                charging: self.charging,
                voltage_mv,
            });
            self.settings = Some(app);
        }
        self.settings.as_mut().unwrap()
    }

    pub fn get_or_create_music(&mut self) -> &mut Player {
        if self.music.is_none() {
            let mut app = Player::new(Arc::clone(&self.board));
            app.set_preferences(self.preferences);
            self.music = Some(app);
        }
        self.music.as_mut().unwrap()
    }

    pub fn get_or_create_terminal(&mut self) -> &mut Terminal {
        if self.terminal.is_none() {
            let mut app = Terminal::new();
            app.set_preferences(self.preferences);
            app.update(terminal::Message::Resized(self.size));
            self.terminal = Some(app);
        }
        self.terminal.as_mut().unwrap()
    }

    // Hosted apps getter methods: lazily creates the app instance if it has not been accessed yet.
    pub fn calculator(&mut self) -> &Calculator {
        self.get_or_create_calculator()
    }

    pub fn counter(&mut self) -> &Counter {
        self.get_or_create_counter()
    }

    pub fn hello(&mut self) -> &Hello {
        self.get_or_create_hello()
    }

    pub fn settings(&mut self) -> &Settings {
        self.get_or_create_settings()
    }

    pub fn music(&mut self) -> &Player {
        self.get_or_create_music()
    }

    pub fn terminal(&mut self) -> &Terminal {
        self.get_or_create_terminal()
    }

    /// Pushes the status bar's three readings into the launcher, for tests and the subscription.
    pub fn set_status(&mut self, clock: impl Into<String>, battery: u8, charging: bool, wifi: u8) {
        self.clock = clock.into();
        self.battery = battery.min(100);
        self.charging = charging;
        self.wifi = wifi.min(style::WIFI_BARS);

        if let Some(settings) = &mut self.settings {
            let voltage_mv = self.board.power().battery_voltage_mv().unwrap_or(0) as u16;
            settings.set_battery(settings::Battery {
                percent: self.battery,
                charging,
                voltage_mv,
            });
        }
    }

    /// Checks if a hosted app is currently running in memory (foreground or background).
    pub fn is_app_running(&self, index: usize) -> bool {
        self.running_apps.contains(&index)
    }

    /// Returns the indices of all hosted apps currently running in memory.
    pub fn running_apps(&self) -> &[usize] {
        &self.running_apps
    }

    /// Terminates a hosted app, removing it from running apps and completely freeing its memory.
    pub fn kill_app(&mut self, index: usize) {
        self.running_apps.retain(|&i| i != index);
        match index {
            CALCULATOR => self.calculator = None,
            COUNTER => self.counter = None,
            HELLO => self.hello = None,
            SETTINGS => self.settings = None,
            MUSIC => self.music = None,
            TERMINAL => self.terminal = None,
            _ => {}
        }
        if self.screen == Screen::App(index) {
            self.screen = Screen::Grid;
        }
    }

    /// The theme: the palette and the background the platform paints behind the tree.
    pub fn theme(&self) -> Theme {
        match self.preferences.theme {
            ThemeMode::Dark => Theme::custom(
                "PomeloDark",
                Palette {
                    background: Color::from_rgb8(20, 22, 38),
                    ..Palette::DARK
                },
            ),
            ThemeMode::Light => Theme::custom(
                "PomeloLight",
                Palette {
                    background: Color::from_rgb8(242, 242, 247),
                    ..Palette::LIGHT
                },
            ),
        }
    }

    /// Reacts to one message.
    ///
    /// The only work that comes back is the settings app's: it scrolls its own body when the page
    /// changes, and a widget operation has to travel as a [`Task`] from whoever owns the loop.
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Open(index) => {
                self.screen = Screen::App(index);
                if !self.running_apps.contains(&index) {
                    self.running_apps.push(index);
                }

                // Eagerly instantiate the opened app on first launch and restart any state
                match index {
                    TERMINAL => {
                        self.get_or_create_terminal();
                    }
                    CALCULATOR => {
                        self.get_or_create_calculator();
                    }
                    COUNTER => {
                        self.get_or_create_counter();
                    }
                    HELLO => {
                        let hello = self.get_or_create_hello();
                        hello.restart();
                    }
                    SETTINGS => {
                        self.get_or_create_settings();
                    }
                    MUSIC => {
                        self.get_or_create_music();
                    }
                    _ => {}
                }

                // An app whose layout follows the screen is told the size this launcher knows — the
                // one the platform reported. The hand-off is not a courtesy: the platform announces
                // the size *once*, when the window or the panel opens, and an app that is not on
                // screen at that moment never hears it. See [`Launcher::hand_over_size`].
                self.hand_over_size();
            }
            Message::PageChanged(page) => {
                self.page = page;
            }
            Message::Back => return self.go_back(),
            Message::Exit => match self.screen {
                Screen::App(index) => {
                    self.kill_app(index);
                }
                Screen::Grid => {
                    if let Some(&last) = self.running_apps.last() {
                        self.kill_app(last);
                    }
                }
            },
            Message::Status(clock, battery, charging, wifi) => {
                self.set_status(clock, battery, charging, wifi);

                let is_connected = self.wifi > 0
                    || self.board.wifi().status().state == pomelo_hal::WifiState::Connected;
                let is_synced = self.board.time().is_synced();
                if is_connected
                    && !is_synced
                    && !self.task_manager.is_running(tasks::SystemTaskId::TimeSync)
                {
                    self.task_manager
                        .set_status(tasks::SystemTaskId::TimeSync, tasks::TaskStatus::Running);
                    return tasks::perform(Arc::clone(&self.board), tasks::SystemTaskId::TimeSync);
                }
            }
            Message::SystemTask(task_msg) => {
                match task_msg {
                    tasks::SystemTaskMessage::Started(id) => {
                        self.task_manager.set_status(id, tasks::TaskStatus::Running);
                    }
                    tasks::SystemTaskMessage::Finished { id, result } => {
                        let status = match &result {
                            Ok(()) => tasks::TaskStatus::Success,
                            Err(e) => tasks::TaskStatus::Failed(e.clone()),
                        };
                        self.task_manager.set_status(id, status);

                        match id {
                            tasks::SystemTaskId::WifiAutoConnect => {
                                let is_connected = self.board.wifi().status().state
                                    == pomelo_hal::WifiState::Connected;
                                let is_synced = self.board.time().is_synced();
                                if is_connected
                                    && !is_synced
                                    && !self.task_manager.is_running(tasks::SystemTaskId::TimeSync)
                                {
                                    self.task_manager.set_status(
                                        tasks::SystemTaskId::TimeSync,
                                        tasks::TaskStatus::Running,
                                    );
                                    return tasks::perform(
                                        Arc::clone(&self.board),
                                        tasks::SystemTaskId::TimeSync,
                                    );
                                }
                            }
                            tasks::SystemTaskId::TimeSync => {
                                if result.is_ok() {
                                    let (clock, _) = current_time_info();
                                    self.clock = clock;
                                    let now_unix = self.board.time().now_unix();
                                    self.board.emit_event(pomelo_hal::SystemEvent::TimeSynced {
                                        unix_secs: now_unix,
                                    });
                                }
                            }
                        }
                    }
                }
            }
            Message::Resized(size) => {
                self.size = size;
                self.hand_over_size();
            }
            Message::Calculator(message) => self.get_or_create_calculator().update(message),
            Message::Counter(message) => self.get_or_create_counter().update(message),
            Message::Hello(message) => self.get_or_create_hello().update(message),
            Message::Music(message) => self.get_or_create_music().update(message),
            Message::Terminal(message) => self.get_or_create_terminal().update(message),
            Message::Settings(settings::Message::Back) => return self.go_back(),
            Message::Settings(message) => {
                let (task, new_prefs) = {
                    let settings = self.get_or_create_settings();

                    (settings.update(message), settings.preferences())
                };

                if self.preferences != new_prefs {
                    self.preferences = new_prefs;
                    self.propagate_preferences();
                }

                return task.map(Message::Settings);
            }
        }

        Task::none()
    }

    /// Tells the app on screen the size of the screen, if its layout depends on it.
    ///
    /// The terminal is the one app that does: its keyboard band is a share of the screen's *height*
    /// (clamped — see `touch_keyboard::band_height`) and its transcript wraps to the screen's width.
    /// Its keyboard's width is nobody's business any more, which is why the settings app — whose
    /// whole layout is flex boxes — is not told anything.
    ///
    /// The terminal can also hear the platform itself while it is on screen —
    /// [`Launcher::subscription`] merges its subscription — but it was not on screen when the size
    /// was announced, so an app opened later would lay its band out for the design panel until
    /// something resized the window. This is the hand-off that makes its first frame right instead.
    fn hand_over_size(&mut self) {
        if let Screen::App(TERMINAL) = self.screen {
            if let Some(terminal) = &mut self.terminal {
                terminal.update(terminal::Message::Resized(self.size));
            }
        }
    }

    /// Navigates back: asks the currently active app to go back if it has sub-pages
    /// (e.g. settings using enum state machine mode A). If the app consumes the back
    /// press, the launcher stays in the app. Otherwise (or if the app has no sub-pages),
    /// the launcher backgrounds the app and returns to the grid. In the grid, back does nothing.
    ///
    /// Whatever the app's back press produced comes back out: the settings app scrolls its own
    /// body, and a widget operation has to travel as a [`Task`] from whoever owns the loop.
    fn go_back(&mut self) -> Task<Message> {
        let (consumed, task) = match self.screen {
            Screen::Grid => (true, Task::none()),
            Screen::App(SETTINGS) => {
                if let Some(settings) = &mut self.settings {
                    let back = settings.go_back();
                    let new_prefs = settings.preferences();

                    if self.preferences != new_prefs {
                        self.preferences = new_prefs;
                        self.propagate_preferences();
                    }

                    match back {
                        Some(task) => (true, task.map(Message::Settings)),
                        None => (false, Task::none()),
                    }
                } else {
                    (false, Task::none())
                }
            }
            Screen::App(_) => (false, Task::none()),
        };

        if !consumed {
            self.screen = Screen::Grid;
        }

        task
    }
}
