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
mod status;
mod style;

use std::sync::Arc;

use iced::theme::Palette;
use iced::widget::{button, column, container, text, Column, Row, Space};
use iced::{
    Alignment, Border, Color, Element, Length, Shadow, Size, Subscription, Theme,
};

use calculator::Calculator;
use demo_counter::Counter;
use hello::Hello;
use music_player::Player;
use pomelo_hal::Board;
use settings::Settings;
use terminal::Terminal;

pub use apps::{Entry, CATALOGUE};
pub use pomelo_material_symbols::Icon;
pub use pomelo_widgets::{FontSizeTier, Language, SystemPreferences, ThemeMode};
pub use style::{
    DOT_REST, DOT_UP, LABEL, PER_PAGE, SCREEN, STATUS_BG, STATUS_BG_DARK, STATUS_BG_LIGHT,
    STATUS_HEIGHT, STATUS_INSET,
};

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
        move || Launcher::new(Arc::clone(&board)),
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
    /// hardware events occur (battery level, Wi-Fi status changes, clock minute updates).
    Status(String, u8, u8),
    /// The screen changed size: a window on a desktop, the panel on the board.
    Resized(Size),
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
    calculator: Calculator,
    counter: Counter,
    hello: Hello,
    settings: Settings,
    music: Player,
    terminal: Terminal,
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
    wifi: u8,
    preferences: SystemPreferences,
}

impl Launcher {
    /// The launcher and every app it hosts, all sharing `board`.
    pub fn new(board: Arc<Board>) -> Self {
        // The status bar starts from what the board says, and is continually updated in real-time
        // via Subscription (see [`status_stream`]).
        let (clock, _) = current_time_info();
        let battery = board.power().battery_percent().unwrap_or(0);
        let signal = board.wifi().status().signal_bars();

        let mut launcher = Self {
            screen: Screen::Grid,
            calculator: Calculator::new(),
            counter: Counter::new(),
            hello: Hello::new(),
            settings: Settings::new(Arc::clone(&board)),
            music: Player::new(Arc::clone(&board)),
            // The terminal lays itself out for the screen it is given, keyboard included; a fresh
            // one starts at the design size and is told better when the platform says so.
            terminal: Terminal::new(),
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
            wifi: signal,
            preferences: SystemPreferences::default(),
        };
        let charging = launcher.board.power().is_charging().unwrap_or(false);
        let voltage_mv = launcher.board.power().battery_voltage_mv().unwrap_or(0) as u16;
        launcher.settings.set_battery(settings::Battery {
            percent: battery,
            charging,
            voltage_mv,
        });
        launcher.propagate_preferences();
        launcher
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

    /// Propagates the active preferences to all hosted apps.
    fn propagate_preferences(&mut self) {
        let prefs = self.preferences;
        self.calculator.set_preferences(prefs);
        self.counter.set_preferences(prefs);
        self.hello.set_preferences(prefs);
        self.settings.set_preferences(prefs);
        self.music.set_preferences(prefs);
        self.terminal.set_preferences(prefs);
    }

    /// The active interface language.
    pub fn language(&self) -> Language {
        self.preferences.language
    }

    /// Sets the active interface language.
    pub fn set_language(&mut self, language: Language) {
        self.preferences.language = language;
        self.propagate_preferences();
    }

    /// The hosted apps, for the host and the tests.
    pub fn calculator(&self) -> &Calculator {
        &self.calculator
    }

    pub fn counter(&self) -> &Counter {
        &self.counter
    }

    pub fn hello(&self) -> &Hello {
        &self.hello
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn music(&self) -> &Player {
        &self.music
    }

    pub fn terminal(&self) -> &Terminal {
        &self.terminal
    }

    /// Sets what the status bar shows.
    ///
    /// The platform owns these, and pushes them — rather than iced pulling them from a timer,
    /// which would need an async executor this stack does not have. `wifi` is a count of bars on
    /// the HAL's own `0..=`[`style::WIFI_BARS`] scale.
    pub fn set_status(&mut self, clock: impl Into<String>, battery: u8, wifi: u8) {
        self.clock = clock.into();
        self.battery = battery.min(100);
        self.wifi = wifi.min(style::WIFI_BARS);

        let charging = self.board.power().is_charging().unwrap_or(false);
        let voltage_mv = self.board.power().battery_voltage_mv().unwrap_or(0) as u16;
        self.settings.set_battery(settings::Battery {
            percent: self.battery,
            charging,
            voltage_mv,
        });
    }

    /// Whether `index` app is currently running in memory (foreground or background).
    pub fn is_app_running(&self, index: usize) -> bool {
        self.running_apps.contains(&index)
    }

    /// The list of apps currently running in memory (foreground or background).
    pub fn running_apps(&self) -> &[usize] {
        &self.running_apps
    }

    /// Kills the app, resetting its state and releasing heap/audio memory.
    pub fn kill_app(&mut self, index: usize) {
        self.running_apps.retain(|&i| i != index);
        match index {
            TERMINAL => {
                self.terminal = Terminal::new();
                self.terminal.set_preferences(self.preferences);
                self.hand_over_size();
            }
            CALCULATOR => {
                self.calculator = Calculator::new();
                self.calculator.set_preferences(self.preferences);
            }
            COUNTER => {
                self.counter = Counter::new();
                self.counter.set_preferences(self.preferences);
            }
            HELLO => {
                self.hello = Hello::new();
                self.hello.set_preferences(self.preferences);
            }
            SETTINGS => {
                self.settings = Settings::new(Arc::clone(&self.board));
                self.settings.set_preferences(self.preferences);
            }
            MUSIC => {
                self.music = Player::new(Arc::clone(&self.board));
                self.music.set_preferences(self.preferences);
            }
            _ => {}
        }

        if self.screen == Screen::App(index) {
            self.screen = Screen::Grid;
        }
    }

    /// The screen showing the grid.
    ///
    /// The screen showing the grid.
    ///
    /// Paged with [`pomelo_widgets::pager`], providing horizontal swipe gestures with
    /// interactive previews and threshold snapping.
    fn launcher(&self) -> Element<'_, Message> {
        let pages: Vec<Element<'_, Message>> =
            (0..self.pages()).map(|p| self.page(p)).collect();

        let paged_grid = pomelo_widgets::pager(pages)
            .current_page(self.page)
            .swipe_commit(style::SWIPE_COMMIT)
            .touch_slop(style::SLOP)
            .on_change(Message::PageChanged);

        let screen = column![
            self.status_bar(),
            paged_grid,
            self.dots(),
            Space::new().height(Length::Fixed(style::GUTTER)),
        ]
        .height(Length::Fill);

        container(screen).width(Length::Fill).height(Length::Fill).into()
    }

    /// One page of the grid: up to [`PER_PAGE`] tiles, evenly distributed in rows and columns.
    ///
    /// Both the columns (horizontal) and rows (vertical) are evenly spaced with flexible spaces
    /// (`space-evenly`), ensuring identical gaps between adjacent icons and towards the screen boundaries.
    fn page(&self, page: usize) -> Element<'_, Message> {
        let first = page * style::PER_PAGE;

        let mut page_column = Column::new()
            .width(Length::Fill)
            .height(Length::Fill);

        for row in 0..style::ROWS {
            page_column = page_column.push(Space::new().height(Length::Fill));

            let mut row_widget = Row::new()
                .width(Length::Fill)
                .align_y(Alignment::Center);

            for col in 0..style::COLUMNS {
                let index = first + row * style::COLUMNS + col;
                let tile: Element<'_, Message> = match CATALOGUE.get(index) {
                    Some(_) => self.tile(index),
                    None => self.placeholder_tile(),
                };

                row_widget = row_widget
                    .push(Space::new().width(Length::Fill))
                    .push(tile);
            }

            row_widget = row_widget.push(Space::new().width(Length::Fill));
            page_column = page_column.push(row_widget);
        }

        page_column = page_column.push(Space::new().height(Length::Fill));
        page_column.into()
    }

    /// One dot per page, the one that is up lit.
    fn dots(&self) -> Element<'_, Message> {
        let up = self.page;
        let is_light = self.preferences.theme.is_light();

        let dots = (0..self.pages()).map(|page| {
            let colour = if is_light {
                if page == up {
                    (31, 35, 40)
                } else {
                    (209, 213, 219)
                }
            } else {
                if page == up {
                    style::DOT_UP
                } else {
                    style::DOT_REST
                }
            };

            container(Space::new())
                .width(Length::Fixed(style::DOT))
                .height(Length::Fixed(style::DOT))
                .style(move |_theme| container::Style {
                    background: Some(Color::from_rgb8(colour.0, colour.1, colour.2).into()),
                    border: Border {
                        radius: (style::DOT / 2.0).into(),
                        ..Border::default()
                    },
                    ..container::Style::default()
                })
                .into()
        });

        container(Row::with_children(dots).spacing(style::DOT_GAP))
            .center_x(Length::Fill)
            .into()
    }

    /// How many pages the catalogue makes.
    fn pages(&self) -> usize {
        CATALOGUE.len().div_ceil(style::PER_PAGE)
    }


    fn calculator_screen(&self) -> Element<'_, Message> {
        self.calculator.view().map(Message::Calculator)
    }

    fn counter_screen(&self) -> Element<'_, Message> {
        self.counter.view().map(Message::Counter)
    }

    /// The signature, hosted -- and the only app here whose picture is a function of the clock.
    ///
    /// A hosted app cannot ask for frames itself — a widget has no subscription — so the launcher
    /// merges the one it *does* have in [`Launcher::subscription`], and only while that app is on
    /// screen: an animation behind the grid would keep the loop awake to draw something nobody can
    /// see.
    fn hello_screen(&self) -> Element<'_, Message> {
        self.hello.view().map(Message::Hello)
    }

    fn music_screen(&self) -> Element<'_, Message> {
        self.music.view().map(Message::Music)
    }

    fn terminal_screen(&self) -> Element<'_, Message> {
        self.terminal.view().map(Message::Terminal)
    }

    /// The settings app, which is the one app here that brings its own back button: it navigates
    /// *inside* itself, so a second one from the launcher would be a second way out of a page. Its
    /// `go_back` reports whether it consumed the press, and that answer is what backgrounds it.
    fn settings_screen(&self) -> Element<'_, Message> {
        self.settings.view().map(Message::Settings)
    }


    /// One app tile: an icon button and label, sized strictly to [`style::TILE_WIDTH`] width.
    ///
    /// Only the app's application icon is tappable. Surrounding spaces and margins allow swipe
    /// gestures to pass through cleanly to [`pomelo_widgets::pager`].
    fn tile(&self, index: usize) -> Element<'_, Message> {
        let entry = &CATALOGUE[index];

        let icon_button = button(
            container(
                text(entry.icon.glyph())
                    .size(style::GLYPH)
                    .font(pomelo_material_symbols::font()),
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill),
        )
        .width(Length::Fixed(style::ICON))
        .height(Length::Fixed(style::ICON))
        .padding(0)
        .on_press(Message::Open(index))
        .style(move |_theme, status| icon_style(entry, status));

        let label_raw = entry.localized_name(self.preferences.language);
        let label_display = style::truncate_label(label_raw, style::LABEL_MAX_WIDTH, style::LABEL);
        let label_size = style::LABEL;
        let label_color = if self.preferences.theme.is_light() {
            Color::from_rgb8(17, 24, 39)
        } else {
            Color::WHITE
        };

        let contents = column![
            icon_button,
            text(label_display)
                .size(label_size)
                .color(label_color)
                .wrapping(text::Wrapping::None),
        ]
        .spacing(style::GLYPH_GAP)
        .align_x(Alignment::Center);

        container(contents)
            .width(Length::Fixed(style::TILE_WIDTH))
            .center_x(Length::Fixed(style::TILE_WIDTH))
            .into()
    }

    /// An empty placeholder tile maintaining the exact geometry as [`tile`].
    fn placeholder_tile(&self) -> Element<'_, Message> {
        let label_size = style::LABEL;
        let placeholder = column![
            Space::new()
                .width(Length::Fixed(style::ICON))
                .height(Length::Fixed(style::ICON)),
            text(" ")
                .size(label_size)
                .color(Color::TRANSPARENT)
                .wrapping(text::Wrapping::None),
        ]
        .spacing(style::GLYPH_GAP)
        .align_x(Alignment::Center);

        container(placeholder)
            .width(Length::Fixed(style::TILE_WIDTH))
            .center_x(Length::Fixed(style::TILE_WIDTH))
            .into()
    }

    /// The clock, the signal and the battery.
    ///
    /// The three readings are the platform's to push, so the bar is built from what was pushed and
    /// not read from the board here: a `Program` cannot hold a subscription to a timer, and the
    /// board is the platform's half of the pair. What each reading looks like -- and why the signal
    /// has four bars while the icon set has three -- is `status`'s to say.
    fn status_bar(&self) -> Element<'_, Message> {
        let bg_icons: Vec<Icon> = self
            .running_apps
            .iter()
            .filter(|&&index| match self.screen {
                Screen::App(current) => current != index,
                Screen::Grid => true,
            })
            .filter_map(|&index| CATALOGUE.get(index).map(|entry| entry.icon))
            .collect();

        status::view(
            &self.clock,
            self.battery,
            self.wifi,
            &bg_icons,
            self.preferences.theme,
        )
    }
}

impl Launcher {
    /// The app's subscriptions: the screen's size, keyboard shortcuts, and hosted app subscriptions.
    ///
    /// Background apps like `music-player` and `settings` (Wi-Fi scanning) keep their subscriptions
    /// alive in the background, while on-screen apps get their subscriptions.
    pub fn subscription(&self) -> Subscription<Message> {
        let resized = iced::window::resize_events().map(|(_window, size)| Message::Resized(size));

        let keyboard = iced::keyboard::listen().filter_map(|event| {
            if let iced::keyboard::Event::KeyPressed {
                key,
                modified_key,
                physical_key,
                ..
            } = event
            {
                if key.as_ref() == iced::keyboard::Key::Character("q")
                    || key.as_ref() == iced::keyboard::Key::Character("Q")
                    || modified_key.as_ref() == iced::keyboard::Key::Character("q")
                    || modified_key.as_ref() == iced::keyboard::Key::Character("Q")
                    || matches!(
                        physical_key,
                        iced::keyboard::key::Physical::Code(iced::keyboard::key::Code::KeyQ)
                    )
                {
                    return Some(Message::Back);
                }

                if key.as_ref() == iced::keyboard::Key::Character("w")
                    || key.as_ref() == iced::keyboard::Key::Character("W")
                    || modified_key.as_ref() == iced::keyboard::Key::Character("w")
                    || modified_key.as_ref() == iced::keyboard::Key::Character("W")
                    || matches!(
                        physical_key,
                        iced::keyboard::key::Physical::Code(iced::keyboard::key::Code::KeyW)
                    )
                {
                    return Some(Message::Exit);
                }
            }
            None
        });

        let mut subs = vec![
            resized,
            keyboard,
            Subscription::run_with(
                StatusSubscription {
                    board: Arc::clone(&self.board),
                },
                status_stream,
            ),
        ];

        match self.screen {
            Screen::App(HELLO) => subs.push(self.hello.subscription().map(Message::Hello)),
            Screen::App(TERMINAL) => subs.push(self.terminal.subscription().map(Message::Terminal)),
            _ => {}
        }

        if self.running_apps.contains(&MUSIC) {
            subs.push(self.music.subscription().map(Message::Music));
        }
        if self.running_apps.contains(&SETTINGS) {
            subs.push(self.settings.subscription().map(Message::Settings));
        }

        Subscription::batch(subs)
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
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Open(index) => {
                self.screen = Screen::App(index);
                if !self.running_apps.contains(&index) {
                    self.running_apps.push(index);
                }

                // Opening an app starts it. The original launcher got this for free -- it built a
                // fresh model inside its launch closure -- and it is what makes the signature draw
                // itself again when it is opened a second time, rather than showing the picture the
                // first run ended on.
                if index == HELLO {
                    self.hello.restart();
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
            Message::Back => self.go_back(),
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
            Message::Status(clock, battery, wifi) => self.set_status(clock, battery, wifi),
            Message::Resized(size) => {
                self.size = size;
                self.hand_over_size();
            }
            Message::Calculator(message) => self.calculator.update(message),
            Message::Counter(message) => self.counter.update(message),
            Message::Hello(message) => self.hello.update(message),
            Message::Music(message) => self.music.update(message),
            Message::Terminal(message) => self.terminal.update(message),
            Message::Settings(settings::Message::Back) => self.go_back(),
            Message::Settings(message) => {
                self.settings.update(message);
                let new_prefs = self.settings.preferences();
                if self.preferences != new_prefs {
                    self.preferences = new_prefs;
                    self.propagate_preferences();
                }
            }
        }
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
            self.terminal.update(terminal::Message::Resized(self.size));
        }
    }

    /// Navigates back: asks the currently active app to go back if it has sub-pages
    /// (e.g. settings using enum state machine mode A). If the app consumes the back
    /// press, the launcher stays in the app. Otherwise (or if the app has no sub-pages),
    /// the launcher backgrounds the app and returns to the grid. In the grid, back does nothing.
    fn go_back(&mut self) {
        let consumed = match self.screen {
            Screen::Grid => true,
            Screen::App(SETTINGS) => {
                let handled = self.settings.go_back();
                let new_prefs = self.settings.preferences();
                if self.preferences != new_prefs {
                    self.preferences = new_prefs;
                    self.propagate_preferences();
                }
                handled
            }
            Screen::App(_) => false,
        };

        if !consumed {
            self.screen = Screen::Grid;
        }
    }

    /// Describes the interface for the current state.
    pub fn view(&self) -> Element<'_, Message> {
        match self.screen {
            Screen::Grid => self.launcher(),
            Screen::App(TERMINAL) => self.terminal_screen(),
            Screen::App(CALCULATOR) => self.calculator_screen(),
            Screen::App(COUNTER) => self.counter_screen(),
            Screen::App(HELLO) => self.hello_screen(),
            Screen::App(SETTINGS) => self.settings_screen(),
            Screen::App(MUSIC) => self.music_screen(),
            // An index the catalogue does not have. The grid cannot produce one, but `usize` is not
            // exhaustible, so the arm exists and shows the only screen that is always there.
            Screen::App(_) => self.launcher(),
        }
    }
}

/// An app icon button: accent color at rest, subtly lit when pressed.
fn icon_style(entry: &'static Entry, status: button::Status) -> button::Style {
    let base_color = entry.color();
    let bg = match status {
        button::Status::Pressed => {
            let (r, g, b) = entry.accent;
            Color::from_rgb(
                ((r as f32 * 1.35).min(255.0)) / 255.0,
                ((g as f32 * 1.35).min(255.0)) / 255.0,
                ((b as f32 * 1.35).min(255.0)) / 255.0,
            )
        }
        _ => base_color,
    };

    button::Style {
        background: Some(bg.into()),
        text_color: Color::WHITE,
        border: Border {
            radius: style::ICON_RADIUS.into(),
            ..Border::default()
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

/// Returns the current (formatted clock string, minute index in day).
fn current_time_info() -> (String, u32) {
    if let Ok(duration) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        let total_secs = duration.as_secs();
        // UTC+8 offset (China Standard Time / Beijing Time: 8 hours = 28,800 seconds)
        let local_secs = total_secs + 28800;
        let day_secs = local_secs % 86400;
        let hours = (day_secs / 3600) as u32;
        let minutes = ((day_secs % 3600) / 60) as u32;
        (format!("{:02}:{:02}", hours, minutes), hours * 60 + minutes)
    } else {
        ("10:24".to_string(), 10 * 60 + 24)
    }
}

#[derive(Clone)]
struct StatusSubscription {
    board: Arc<Board>,
}

impl std::hash::Hash for StatusSubscription {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        "app_launcher_status_subscription".hash(state);
    }
}

impl PartialEq for StatusSubscription {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.board, &other.board)
    }
}

impl Eq for StatusSubscription {}

fn status_stream(sub: &StatusSubscription) -> impl iced::futures::Stream<Item = Message> {
    let board = Arc::clone(&sub.board);
    let (mut tx, rx) = iced::futures::channel::mpsc::channel(16);

    // 1. Listen for hardware events emitted by pomelo-hal Board
    let tx_event = tx.clone();
    let board_for_event = Arc::clone(&board);
    board.on_event(move |event| {
        match event {
            pomelo_hal::SystemEvent::BatteryChanged { percent, .. } => {
                let (clock, _) = current_time_info();
                let wifi = board_for_event.wifi().status().signal_bars();
                let mut tx = tx_event.clone();
                let _ = tx.try_send(Message::Status(clock, *percent, wifi));
            }
            pomelo_hal::SystemEvent::WifiStatusChanged(wifi_status) => {
                let (clock, _) = current_time_info();
                let battery = board_for_event.power().battery_percent().unwrap_or(0);
                let mut tx = tx_event.clone();
                let _ = tx.try_send(Message::Status(clock, battery, wifi_status.signal_bars()));
            }
            pomelo_hal::SystemEvent::InputAction(action) => {
                let mut tx = tx_event.clone();
                match action {
                    pomelo_hal::InputAction::Back => {
                        let _ = tx.try_send(Message::Back);
                    }
                    pomelo_hal::InputAction::Exit => {
                        let _ = tx.try_send(Message::Exit);
                    }
                }
            }
        }
    });

    // 2. Background periodic tick & clock update thread
    std::thread::spawn(move || {
        let mut last_minute = u32::MAX;

        loop {
            // Advance time-driven simulated subsystems (Wi-Fi, audio, battery, etc.)
            board.tick();

            let (clock_str, current_minute) = current_time_info();
            if current_minute != last_minute {
                last_minute = current_minute;
                let battery = board.power().battery_percent().unwrap_or(0);
                let wifi = board.wifi().status().signal_bars();

                if tx.try_send(Message::Status(clock_str, battery, wifi)).is_err() {
                    // Receiver was dropped, exit thread cleanly
                    break;
                }
            }

            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    });

    rx
}

#[cfg(test)]
mod tests {
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
        assert!(launcher.settings().password_prompt().is_some());

        // Hardware Back (Message::Back) closes prompt, remains on Wifi subpage
        launcher.update(Message::Back);
        assert!(launcher.settings().password_prompt().is_none());
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

        launcher.update(Message::Status("14:30".to_string(), 85, 3));
        assert_eq!(launcher.clock, "14:30");
        assert_eq!(launcher.battery, 85);
        assert_eq!(launcher.wifi, 3);
        assert_eq!(launcher.settings().battery().percent, 85);
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
}


