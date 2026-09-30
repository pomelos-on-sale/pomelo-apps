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
//! Two consequences worth knowing before changing this file:
//!
//! * a tile is a `MouseArea` around a styled box rather than a `Button`, because a button that
//!   takes presses keeps the drag from ever being seen (the tree gets events child first) — and
//!   there is no hover wash either, for the same reason;
//! * the four quadrants are the layout: a tile is half the page's width and half the grid's height,
//!   the app is centred in it, and the *whole* quadrant is what a finger has to hit.

mod apps;
mod status;
mod style;

use std::sync::Arc;

use iced::theme::Palette;
use iced::widget::{button, column, container, mouse_area, stack, text, Column, Row, Space};
use iced::{
    Alignment, Border, Color, Element, Length, Padding, Point, Shadow, Size, Subscription, Theme,
};

use calculator::Calculator;
use counter::Counter;
use hello::Hello;
use music_player::Player;
use pomelo_hal::Board;
use settings::Settings;
use terminal::Terminal;

pub use apps::{Entry, CATALOGUE};
pub use pomelo_widgets::{FontSizeTier, Language, SystemPreferences, ThemeMode};
pub use style::{
    DOT_REST, DOT_UP, LABEL, PER_PAGE, SCREEN, STATUS_BG, STATUS_HEIGHT, STATUS_INSET,
};

/// The launcher as an iced program, with `board` for its apps.
///
/// The result is iced's own `Application`, which is both a builder — `main.rs` calls `run()` on it —
/// and a [`Program`](iced::Program), so a platform that owns the loop can host it instead
/// (`pomelo_iced_host::Host::new(program, board)`). One definition, the same wiring in both places:
/// the subject of the launcher is not a *shape* of program, it is these four functions.
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
pub const COUNTER: usize = 2;
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
    /// A finger went down: on `Some(index)`'s tile, or on the bare screen (`None`).
    ///
    /// Which of the two it was is iced's answer rather than a hit test here: a `MouseArea` around a
    /// tile takes a press that lands on it — events reach the tree child first, so the launcher's
    /// own area never sees that one — and the launcher's takes every other. Either way a page turn
    /// may start, and either way this is the tile that looks pressed.
    Pressed(Option<usize>),
    /// The finger moved, in the launcher's own coordinates.
    ///
    /// Hover moves arrive here too, which is why a press is what arms a drag: a move on its own
    /// turns nothing.
    Moved(Point),
    /// The finger left the screen.
    Released,
    /// A finger left the screen while it was still over `index`'s tile: a tap, unless it slid first.
    ///
    /// The tile works out *which* tile that was — the same question a `Button` answers for itself —
    /// and the launcher only has to decide whether the finger had turned the page by then.
    Tapped(usize),
    /// The back button, or hardware button 1.
    Back,
    /// The status bar's readings, pushed in by the platform.
    ///
    /// A message and not a method call, because this app is a `Program`: the platform's way in is
    /// [`Host::update`](pomelo_iced_host::Host::update), and the clock, the battery and the signal
    /// are the platform's to know. The same shape the terminal's size arrives in.
    Status(String, u8, u8),
    /// The screen changed size: a window on a desktop, the panel on the board.
    Resized(Size),
    /// A message from one of the apps this launcher hosts.
    Calculator(calculator::Message),
    Counter(counter::Message),
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
    /// The screen's size, as the platform last reported it. The grid's pages and the pager's
    /// thresholds are laid out for it, and it is what the apps whose layout depends on the size are
    /// told — see [`Launcher::hand_over_size`]. Everything else here fills whatever it is given.
    size: Size,
    /// The page turn, if any, and which page the grid is on.
    pager: Pager,
    /// Where the finger was last seen, in pixels across the screen.
    ///
    /// A press is where a drag is measured from, and a `MouseArea` reports one without a position.
    /// It does not need to: the platform sends the move that brought the finger here *before* the
    /// press — a touch that appears does exactly that — and a mouse has been moving all along, so
    /// the last move is the press to within the pixel a finger costs.
    pointer: Option<f32>,
    /// The tile the finger is on, which is the one that looks pressed.
    ///
    /// There is no matching hover: a panel has no pointer that rests somewhere, so a tile is lit by
    /// a finger and by nothing else. That is also why a tile is a `MouseArea` and not a `Button` — a
    /// button's hover comes free, and its capture of the press is what a page turn cannot live with.
    pressed: Option<usize>,
    clock: String,
    battery: u8,
    wifi: u8,
    preferences: SystemPreferences,
}

/// The paged grid: which page is up, and the swipe in flight.
///
/// The pager is a *view of the finger*, not a widget. Which tile a press landed on, whether the
/// finger is still on it — those are iced's answers, taken in [`Launcher::tile`]. What no widget
/// knows is the thing a page turn is: a gesture that starts on a widget and ends somewhere else, so
/// it is kept here.
///
/// A turn has no animation and no second page: the drag decides it and the finger leaving *is* the
/// change ([`Pager::released`]), so the grid ever draws one page.
#[derive(Debug, Default)]
struct Pager {
    /// The page that is up.
    page: usize,
    /// The finger on the grid, while one is down.
    drag: Option<Drag>,
    /// Whether the finger has slid since it went down.
    ///
    /// Kept past the release, because the tile's own release message arrives *first* and asks: a
    /// swipe that opened the app it ended on is this app's oldest complaint.
    swiped: bool,
    /// How far the finger has pulled, in pixels: `0` at rest, and the number the page turn is
    /// decided by ([`style::SWIPE_COMMIT`]).
    pull: f32,
}

/// A finger on the grid.
#[derive(Debug, Clone, Copy)]
struct Drag {
    /// Where the finger went down, which is where the pull is measured from. `None` until somewhere
    /// to measure from is known: the first move of a board that has never been touched.
    from: Option<f32>,
    /// Which way the finger has slid: `-1` back, `+1` on, and `0` while it is a tap with a twitch
    /// in it — or while it is pulling at an end of the grid, where there is no neighbour to turn to.
    step: i32,
}

impl Pager {
    /// A finger went down at `from`: a drag may start.
    fn pressed(&mut self, from: Option<f32>) {
        self.drag = Some(Drag { from, step: 0 });
        self.swiped = false;
        self.pull = 0.0;
    }

    /// The finger moved to `x`, in pixels across the screen.
    fn dragged(&mut self, x: f32, pages: usize) {
        let Some(drag) = &mut self.drag else {
            return;
        };

        // A press that arrived with nothing to measure from — the first touch on a board that has
        // never been touched — starts where the finger got to first.
        let from = *drag.from.get_or_insert(x);
        let delta = from - x;

        // Which neighbour the finger is pulling in, and whether it is pulling hard enough to be
        // pulling at all. Off either end of the grid there is no neighbour to turn to, and below
        // the slop this is a tap with a twitch in it: a finger on a panel is never quite still, and
        // the app it went down on must still open.
        drag.step = if delta > style::SLOP && self.page + 1 < pages {
            1
        } else if delta < -style::SLOP && self.page > 0 {
            -1
        } else {
            0
        };

        self.swiped = drag.step != 0;

        // How far the finger has travelled, measured from where it went *down* — the slop came off
        // this number once, and that made every page turn late by it.
        self.pull = if drag.step == 0 { 0.0 } else { delta.abs() };
    }

    /// The finger left: a page it travelled far enough for turns, and short of that nothing did.
    ///
    /// This is the whole turn. There is nothing to animate and nothing to land: the page is changed
    /// here, so the frame after this one draws the new page where the old one was.
    fn released(&mut self, pages: usize) {
        let step = self.drag.take().map_or(0, |drag| drag.step);
        let arrived = self.pull >= style::SWIPE_COMMIT;

        self.pull = 0.0;

        if arrived {
            let page = self.page as i32 + step;

            self.page = page.clamp(0, pages.saturating_sub(1) as i32) as usize;
        }
    }

    /// Whether the finger slid since it went down: a tap that did is not a tap.
    fn swiped(&self) -> bool {
        self.swiped
    }
}

impl Launcher {
    /// The launcher and every app it hosts, all sharing `board`.
    pub fn new(board: Arc<Board>) -> Self {
        // The status bar starts from what the board says, so a board nobody pushes readings into —
        // the desktop simulator, a panel test — shows the truth instead of a constant. The platform
        // pushes updates in through `Message::Status`; see [`Launcher::set_status`].
        let battery = board.power().battery_percent().unwrap_or(0);
        let signal = board.wifi().status().signal_bars();

        Self {
            screen: Screen::Grid,
            calculator: Calculator::new(),
            counter: Counter::new(),
            hello: Hello::new(),
            settings: Settings::new(Arc::clone(&board)),
            music: Player::new(board),
            // The terminal lays itself out for the screen it is given, keyboard included; a fresh
            // one starts at the design size and is told better when the platform says so.
            terminal: Terminal::new(),
            // What the screen is until the platform says: the design panel. The first `Resized`
            // always arrives — the window manager's on a desktop, the host's first iteration on the
            // board — so this is what at most one frame is drawn from, and never what a layout is
            // decided by. A launcher is *told* how big the screen is; it may not assume it.
            size: Size::new(SCREEN as f32, SCREEN as f32),
            pager: Pager::default(),
            pointer: None,
            pressed: None,
            clock: String::from("10:24"),
            battery,
            wifi: signal,
            preferences: SystemPreferences::default(),
        }
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
        self.settings.set_preferences(preferences);
    }

    /// The active interface language.
    pub fn language(&self) -> Language {
        self.preferences.language
    }

    /// Sets the active interface language.
    pub fn set_language(&mut self, language: Language) {
        self.preferences.language = language;
        self.settings.set_language(language);
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
    }

    /// The screen showing the grid.
    ///
    /// The whole screen is wrapped in a `MouseArea`, which is what makes a drag possible at all: it
    /// is the launcher's own area, so it sees every move and every release, and it sees a press
    /// wherever the tiles do not (a press that lands on one is taken by that tile, one level down).
    ///
    /// The grid is the page that is up and nothing else — a swipe is decided while the finger is
    /// down and happens when it leaves — and it takes everything between the status bar and the
    /// dots: four quadrants, one app in the middle of each.
    fn launcher(&self) -> Element<'_, Message> {
        let screen = column![
            self.status_bar(),
            self.page(self.pager.page),
            self.dots(),
            Space::new().height(Length::Fixed(style::GUTTER)),
        ]
        .height(Length::Fill);

        mouse_area(container(screen).width(Length::Fill).height(Length::Fill))
            .on_press(Message::Pressed(None))
            .on_move(Message::Moved)
            .on_release(Message::Released)
            .into()
    }

    /// One page of the grid: up to [`PER_PAGE`] tiles, one to a quadrant.
    ///
    /// The quadrants are the layout, not decoration: the rows are equal shares of the grid's height
    /// and the cells equal shares of its width, so where an app is drawn is a quarter of the screen
    /// — and the whole quarter is the target a finger has to hit, while the box that lights up under
    /// it is the app's own size (see [`Launcher::tile`]).
    fn page(&self, page: usize) -> Element<'_, Message> {
        let first = page * style::PER_PAGE;

        let rows = (0..style::ROWS).map(|row| {
            let tiles = (0..style::COLUMNS).map(|column| {
                let index = first + row * style::COLUMNS + column;

                match CATALOGUE.get(index) {
                    Some(_) => self.tile(index),
                    // A page that is not full keeps the shape of one that is: an empty cell, so the
                    // tiles it does have are the size they would be and in the place they would be.
                    None => Space::new().width(Length::Fill).into(),
                }
            });

            Row::with_children(tiles)
                .spacing(style::GUTTER)
                .height(Length::Fill)
                .into()
        });

        Column::with_children(rows)
            .spacing(style::GUTTER)
            .padding(Padding {
                left: style::GUTTER,
                right: style::GUTTER,
                ..Padding::ZERO
            })
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// One dot per page, the one that is up lit.
    fn dots(&self) -> Element<'_, Message> {
        let up = self.pager.page;

        let dots = (0..self.pages()).map(|page| {
            let colour = if page == up {
                style::DOT_UP
            } else {
                style::DOT_REST
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

    /// An app that fills the panel, with the launcher's back button stacked over it.
    ///
    /// A sub-app is a widget tree, not a second loop: the launcher renders its `view` and routes
    /// its messages, which is what `Element::map` is for. The back button is stacked rather than
    /// given a row of its own because every app here is designed to fill 480x480 -- and it lands on
    /// whatever empty corner the app has, which for the calculator is the left half of its display
    /// card and for the signature is the wash.
    ///
    /// The lifetime is named rather than elided because the two are the *same* one: the app's
    /// `view` borrows the app, which lives in the launcher, so both the child and the back button
    /// borrow the launcher for as long as the screen lasts.
    fn hosted<'a>(&'a self, index: usize, app: Element<'a, Message>) -> Element<'a, Message> {
        let back = container(self.back(index))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Start)
            .align_y(Alignment::Start)
            .padding(style::GUTTER)
            .into();

        stack([app, back]).into()
    }

    fn calculator_screen(&self) -> Element<'_, Message> {
        self.hosted(CALCULATOR, self.calculator.view().map(Message::Calculator))
    }

    fn counter_screen(&self) -> Element<'_, Message> {
        self.hosted(COUNTER, self.counter.view().map(Message::Counter))
    }

    /// The signature, hosted -- and the only app here whose picture is a function of the clock.
    ///
    /// A hosted app cannot ask for frames itself — a widget has no subscription — so the launcher
    /// merges the one it *does* have in [`Launcher::subscription`], and only while that app is on
    /// screen: an animation behind the grid would keep the loop awake to draw something nobody can
    /// see.
    fn hello_screen(&self) -> Element<'_, Message> {
        self.hosted(HELLO, self.hello.view().map(Message::Hello))
    }

    fn music_screen(&self) -> Element<'_, Message> {
        self.hosted(MUSIC, self.music.view().map(Message::Music))
    }

    fn terminal_screen(&self) -> Element<'_, Message> {
        self.hosted(TERMINAL, self.terminal.view().map(Message::Terminal))
    }

    /// The settings app, which is the one app here that brings its own back button: it navigates
    /// *inside* itself, so a second one from the launcher would be a second way out of a page. Its
    /// `go_back` reports whether it consumed the press, and that answer is what backgrounds it.
    fn settings_screen(&self) -> Element<'_, Message> {
        self.settings.view().map(Message::Settings)
    }

    /// The button that goes back to the grid.
    ///
    /// Its accent is what the tests find it by on the panel, and what makes it look like it
    /// belongs to the app it came from.
    fn back(&self, index: usize) -> Element<'_, Message> {
        let entry = &CATALOGUE[index];
        let label_text = self.preferences.language.back_label();
        let label_size = self.preferences.font_tier.label_size();

        button(text(label_text).size(label_size))
            .on_press(Message::Back)
            .padding(12)
            .style(move |theme, status| button::Style {
                background: Some(entry.color().into()),
                ..tile_style(theme, status)
            })
            .into()
    }

    /// One app: a tappable tile, so the whole square is the target and not just the glyph.
    ///
    /// A `MouseArea` around a styled box rather than a `Button`, and the pager is why: a button
    /// *captures* the press it is given, and the press is where a page turn starts — one that lands
    /// on a tile is the only press a finger makes. What the button did for this tile it still does:
    /// the wash under the finger comes from [`Launcher::tile_status`] and the release is the tap.
    /// What is lost is what a panel does not have — the hand cursor, the keyboard, and the hover
    /// wash, which would need a message to ask for (a rebuilt tile reports a hover for a cursor that
    /// never moved, and a message is a redraw).
    fn tile(&self, index: usize) -> Element<'_, Message> {
        let entry = &CATALOGUE[index];

        let icon = container(
            text(entry.icon.glyph())
                .size(style::GLYPH)
                .font(pomelo_material_symbols::font()),
        )
        .center_x(Length::Fixed(style::ICON))
        .center_y(Length::Fixed(style::ICON))
        .style(move |_theme| container::Style {
            background: Some(entry.color().into()),
            border: Border {
                radius: style::ICON_RADIUS.into(),
                ..Border::default()
            },
            ..container::Style::default()
        });

        let label_text = entry.localized_name(self.preferences.language);
        let label_size = self.preferences.font_tier.label_size();

        let contents = column![icon, text(label_text).size(label_size)]
            .spacing(style::GLYPH_GAP)
            .align_x(Alignment::Center);

        let status = self.tile_status(index);

        // The wash is the size of the *app* — the icon and its label, with a little air around them
        // — while the target is the whole quadrant: a finger does not have to be on the picture to
        // open what it stands for.
        mouse_area(
            container(
                container(contents)
                    .padding(style::TILE_PADDING)
                    .style(move |theme| tile_box(theme, status)),
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .width(Length::Fill)
            .height(Length::Fill),
        )
        .on_press(Message::Pressed(Some(index)))
        .on_release(Message::Tapped(index))
        .into()
    }

    /// How a tile looks: pressed, or nothing at all.
    ///
    /// `button::Status` for a tile that is not a button, because the tile's colours are still the
    /// button's — [`tile_style`] decides them, and the back button is a real one.
    fn tile_status(&self, index: usize) -> button::Status {
        if self.pressed == Some(index) {
            button::Status::Pressed
        } else {
            button::Status::Active
        }
    }

    /// The clock, the signal and the battery.
    ///
    /// The three readings are the platform's to push, so the bar is built from what was pushed and
    /// not read from the board here: a `Program` cannot hold a subscription to a timer, and the
    /// board is the platform's half of the pair. What each reading looks like -- and why the signal
    /// has four bars while the icon set has three -- is `status`'s to say.
    fn status_bar(&self) -> Element<'_, Message> {
        status::view(&self.clock, self.battery, self.wifi)
    }
}

impl Launcher {
    /// The app's subscriptions: the screen's size, and whichever hosted app is on screen.
    ///
    /// This is the launcher's half of hosting. A hosted app is a widget, so its `view` and `update`
    /// are the launcher's to call — but a *subscription* is a claim on the loop, and there is one
    /// loop. The apps that have one are the two whose frames move (`hello`, `music-player`), the one
    /// whose layout follows the screen (`terminal`), and the settings app while its Wi-Fi
    /// page is open — that page waits on a radio, and waiting is what a subscription is for. An app
    /// that is not on screen has no subscription, which is what "a signature behind the grid must not
    /// keep the loop awake" means now that it is stated rather than asked.
    ///
    /// The size is the launcher's own, and it is subscribed to from the first frame: the grid's
    /// quadrants and the pager are the screen's width, and a launcher that never heard the platform
    /// would lay itself out for a panel it is not on. `window::resize_events()` is an event stream
    /// and not a clock — it produces a message when the window changes size and nothing at all in
    /// between — so having it on permanently costs no frames.
    ///
    /// The grid has no subscription of its own: a page turn is decided by the finger and drawn on
    /// the frame the release produces, so there is nothing to animate and no frames to ask for.
    pub fn subscription(&self) -> Subscription<Message> {
        let app = match self.screen {
            Screen::App(HELLO) => self.hello.subscription().map(Message::Hello),
            Screen::App(MUSIC) => self.music.subscription().map(Message::Music),
            Screen::App(TERMINAL) => self.terminal.subscription().map(Message::Terminal),
            Screen::App(SETTINGS) => self.settings.subscription().map(Message::Settings),
            _ => Subscription::none(),
        };

        let resized = iced::window::resize_events().map(|(_window, size)| Message::Resized(size));

        Subscription::batch([resized, app])
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
                self.forget_press();
                self.screen = Screen::App(index);

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
            Message::Back => {
                self.forget_press();
                self.screen = Screen::Grid;
            }

            // A finger went down — on a tile, or on the screen behind the tiles — and that is what
            // arms a page turn: the turn itself is decided by where the finger goes from here.
            Message::Pressed(tile) => {
                self.pressed = tile;
                self.pager.pressed(self.pointer);
            }
            Message::Moved(point) => {
                self.pointer = Some(point.x);
                self.pager.dragged(point.x, self.pages());
            }
            Message::Released => {
                self.pressed = None;
                self.pager.released(self.pages());
            }
            Message::Tapped(index) => {
                // A release over the tile it went down on, which is a tap only while the finger did
                // not *slide*: a swipe that opened the app it started on was this app's oldest
                // complaint.
                if !self.pager.swiped() {
                    self.update(Message::Open(index));
                }
            }
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
            // The settings app navigates itself and its own button *is* its back: `go_back` moves
            // up a page and reports whether it had anywhere to go. `false` is the press it did not
            // consume, which is the launcher's cue to leave the app -- the same contract the
            // original's launcher was written against.
            Message::Settings(settings::Message::Back) => {
                if !self.settings.go_back() {
                    self.forget_press();
                    self.screen = Screen::Grid;
                }
                self.preferences = self.settings.preferences();
            }
            Message::Settings(message) => {
                self.settings.update(message);
                self.preferences = self.settings.preferences();
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

    /// Forgets the tile the finger was on.
    ///
    /// A tile's wash is this app's state, and the tile it belongs to is rebuilt whenever the screen
    /// changes or the page turns: the finger that was on it is gone — it is what asked for the change
    /// — so the wash goes with it.
    fn forget_press(&mut self) {
        self.pressed = None;
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

/// A tile's box: [`tile_style`]'s colours, as a container rather than as a button.
///
/// A tile is not a `Button` any more (see [`Launcher::tile`]), but it should not look any different,
/// so the colours stay in one place and are worn two ways.
fn tile_box(theme: &Theme, status: button::Status) -> container::Style {
    let style = tile_style(theme, status);

    container::Style {
        background: style.background,
        border: style.border,
        shadow: style.shadow,
        ..container::Style::default()
    }
}

/// A tile: nothing at rest, a wash when the finger is on it.
fn tile_style(_theme: &Theme, status: button::Status) -> button::Style {
    button::Style {
        background: match status {
            button::Status::Hovered => Some(Color::from_rgba(1.0, 1.0, 1.0, 0.05).into()),
            button::Status::Pressed => Some(Color::from_rgba(1.0, 1.0, 1.0, 0.11).into()),
            _ => None,
        },
        text_color: Color::WHITE,
        border: Border {
            radius: style::TILE_RADIUS.into(),
            ..Border::default()
        },
        shadow: Shadow::default(),
        snap: false,
    }
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
            assert_ne!(entry.localized_name(Language::Chinese), entry.localized_name(Language::English));
            assert!(!entry.localized_name(Language::Chinese).is_empty());
            assert!(!entry.localized_name(Language::English).is_empty());
        }

        assert_eq!(CATALOGUE[TERMINAL].localized_name(Language::Chinese), "终端");
        assert_eq!(CATALOGUE[CALCULATOR].localized_name(Language::Chinese), "计算器");
        assert_eq!(CATALOGUE[COUNTER].localized_name(Language::Chinese), "计数器");
        assert_eq!(CATALOGUE[HELLO].localized_name(Language::Chinese), "你好");
        assert_eq!(CATALOGUE[SETTINGS].localized_name(Language::Chinese), "设置");
        assert_eq!(CATALOGUE[MUSIC].localized_name(Language::Chinese), "音乐");
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

        // Cycle font tier via Settings message
        launcher.update(Message::Settings(settings::Message::CycleFontTier));
        assert_eq!(launcher.preferences().font_tier, FontSizeTier::Large);
        assert_eq!(launcher.preferences().font_tier.base_size(), 21.0);

        launcher.update(Message::Settings(settings::Message::CycleFontTier));
        assert_eq!(launcher.preferences().font_tier, FontSizeTier::Standard);
        assert_eq!(launcher.preferences().font_tier.base_size(), 18.0);

        // Set preferences directly on launcher
        let custom_prefs = SystemPreferences::new(Language::Chinese, ThemeMode::Dark, FontSizeTier::Large);
        launcher.set_preferences(custom_prefs);
        assert_eq!(launcher.preferences(), custom_prefs);
        assert_eq!(launcher.settings().preferences(), custom_prefs);
    }
}
