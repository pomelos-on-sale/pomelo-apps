//! The counter, built from iced widgets — **a standard iced program**.
//!
//! The smallest app here, and the one that says the least about the platform: one number, one
//! button, and a card that splits its height 2:3:2 between the title, the number and the button --
//! which is what makes it a layout test rather than a logic test. The number is 96 pt and the panel
//! is 480 px, so it only stays on one line because the type does not scale with the panel.
//!
//! Nothing here names a platform, a panel or a renderer. `main.rs` runs it, `iced::application` asks
//! for everything else, and which layer answers depends on who builds this crate: iced's own
//! `iced_winit` and a window in this project, or the board's panel in `firmware/`.

pub mod style;

use iced::widget::{button, column, container, text};
use iced::{theme::Palette, Alignment, Border, Color, Element, Length, Shadow, Theme};

pub use style::SCREEN;

/// What the counter reacts to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Message {
    /// The button.
    Tap,
}

/// The counter.
pub struct Counter {
    count: u32,
}

impl Counter {
    pub fn new() -> Self {
        Self { count: 0 }
    }

    /// What the display shows. Read by the host and by the tests.
    pub fn count(&self) -> u32 {
        self.count
    }

    /// The number, two digits wide so that 9 and 10 do not shift the layout under the finger.
    fn number(&self) -> Element<'_, Message> {
        container(
            text(format!("{:02}", self.count))
                .size(style::NUMBER_FONT)
                .color(Color::WHITE),
        )
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
    }

    /// The button, which is what a finger is aimed at.
    fn button(&self) -> Element<'_, Message> {
        button(
            container(text("+ 1 TAP").size(style::LABEL_FONT).color(Color::WHITE))
                .center_x(Length::Fill)
                .center_y(Length::Fill),
        )
        .padding(style::BUTTON_PADDING)
        .width(Length::Shrink)
        .height(Length::Shrink)
        .style(|_theme, status| {
            let fill = match status {
                button::Status::Pressed | button::Status::Hovered => style::BUTTON_PRESSED,
                _ => style::BUTTON,
            };

            button::Style {
                background: Some(fill.into()),
                text_color: Color::WHITE,
                border: Border {
                    radius: style::BUTTON_RADIUS.into(),
                    ..Border::default()
                },
                shadow: Shadow::default(),
                snap: false,
            }
        })
        .on_press(Message::Tap)
        .into()
    }

    /// The card the counter lives in.
    ///
    /// The 2:3:2 split the original spells `Expanded(flex)` is a `FillPortion` here, and
    /// the card itself is a `Fill`: it takes the height the footer leaves it, so the absolute
    /// measurements are the type sizes and the button's padding, not the card's height.
    fn card(&self) -> Element<'_, Message> {
        let body = column![
            band(
                container(
                    text("TOUCH COUNTER")
                        .size(style::TITLE_FONT)
                        .color(style::TITLE)
                )
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into(),
                2
            ),
            band(self.number(), 3),
            band(
                container(self.button())
                    .center_x(Length::Fill)
                    .center_y(Length::Fill)
                    .into(),
                2
            ),
        ]
        .align_x(Alignment::Center);

        container(body)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_theme| container::Style {
                background: Some(style::CARD.into()),
                border: Border {
                    color: style::CARD_BORDER,
                    width: style::CARD_BORDER_WIDTH,
                    radius: style::CARD_RADIUS.into(),
                },
                ..container::Style::default()
            })
            .into()
    }
}

/// One band of the card's 2:3:2 split.
///
/// A function and not a closure: a closure taking `Element<'_, ..>` and returning one ties the two
/// lifetimes together, and `Container` is invariant over its lifetime.
fn band<'a>(child: Element<'a, Message>, share: u16) -> Element<'a, Message> {
    container(child).center_y(Length::FillPortion(share)).into()
}

impl Default for Counter {
    fn default() -> Self {
        Self::new()
    }
}

impl Counter {
    /// The counter's theme.
    ///
    /// A solid background, not a wallpaper primitive: the compositor paints the background over the
    /// damage rectangle only, while a full-screen primitive costs the whole screen every frame. The
    /// launcher's theme carries the measurement.
    pub fn theme(&self) -> Theme {
        Theme::custom(
            "Pomelo",
            Palette {
                background: style::BACKGROUND,
                ..Palette::DARK
            },
        )
    }

    /// Reacts to a tap.
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Tap => self.count += 1,
        }
    }

    /// The whole screen.
    pub fn view(&self) -> Element<'_, Message> {
        container(
            column![
                self.card(),
                container(
                    text("Powered by iced")
                        .size(style::FOOTER_FONT)
                        .color(style::FOOTER)
                )
                .width(Length::Fill)
                .center_x(Length::Fill),
            ]
            .spacing(style::CARD_GAP),
        )
        .padding(style::PAGE_PADDING)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}
