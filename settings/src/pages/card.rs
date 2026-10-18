//! A card, and the tiles in it.
//!
//! iced has no `ListTile`, so the list's row is ours to name: [`Tile`] is the unit every settings
//! list is built from — the icon and its fill, the label, what sits beside it, what sits at its
//! end, and the message the whole row sends. [`Card`] is the surface those sit on.
//!
//! # A `Card` is not a group of tiles
//!
//! It holds any content at all. A sub-page's head card with a large icon and a title
//! in it, and no tiles — that is [`section_card`]. That is why the hairline belongs to
//! [`Card::tile`] rather than to a loop over tiles: a card cannot know which of its children *are*
//! tiles, but it does know whether it is empty.

use iced::widget::{button, container, text, Column, Row, Space};
use iced::{Alignment, Border, Color, Length, Padding};

use pomelo_material_symbols::Icon;

use crate::pages::common::{
    capsule_button, card_surface, nav_style, row_style, separator, Group, PageItem, UI,
};
use crate::style;
use crate::{Message, SettingsSection};
use pomelo_widgets::ThemeMode;

/// The square at the left of a tile: a glyph and the fill it sits on.
///
/// One field and not two, because they are one thing — a tile that has no square has no fill either,
/// and a bare `color` would have to hold a value nobody reads.
struct Mark {
    icon: Icon,
    color: style::IconColor,
}

/// The horizontal alignment of a key-value row's value text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum ValueAlign {
    Left,
    #[default]
    Right,
}

/// One row of a settings list.
///
/// Named fields rather than a tuple. iced has no `ListTile`, so the row is ours to name — and a
/// six-element tuple is not something a reader can count.
pub(crate) struct Tile<'a> {
    /// The square on the left, for a row that leads somewhere.
    mark: Option<Mark>,
    label: &'static str,
    label_color: Option<iced::Color>,
    value_text: Option<String>,
    value_align: ValueAlign,
    /// What sits between the label and the end of the row, or nothing.
    ///
    /// An `Element` and not a `String`: most rows put a plain value there, and one puts the password
    /// field, which is dots *painted* rather than typed and cannot be spelled. The value rows make
    /// their text with `text(...)`, and the tests find it by that text — which is the ability they
    /// need.
    secondary: Option<UI<'a>>,
    trailing: Option<Icon>,
    /// What a press sends, or `None` for a row that is not pressable.
    message: Option<Message>,
    /// Whether this row is an outlined input box (e.g. password field) rather than a standard info row.
    is_input_field: bool,
}

impl<'a> Tile<'a> {
    /// A tile that opens `section`, ending in the chevron that says so.
    ///
    /// The icon's fill comes from the section too, so a tile cannot be given a colour the palette
    /// has not got — see [`style::IconColor`].
    pub(crate) fn section(icon: Icon, section: SettingsSection, label: &'static str) -> Self {
        Self {
            mark: Some(Mark {
                icon,
                color: style::IconColor::for_section(section),
            }),
            label,
            label_color: None,
            value_text: None,
            value_align: ValueAlign::Right,
            secondary: None,
            trailing: Some(Icon::KEYBOARD_ARROW_RIGHT),
            message: Some(Message::Open(section)),
            is_input_field: false,
        }
    }

    /// A tile without a left badge icon that opens `section`, ending in a right chevron.
    pub(crate) fn nav(label: &'static str, section: SettingsSection) -> Self {
        Self {
            mark: None,
            label,
            label_color: None,
            value_text: None,
            value_align: ValueAlign::Right,
            secondary: None,
            trailing: Some(Icon::KEYBOARD_ARROW_RIGHT),
            message: Some(Message::Open(section)),
            is_input_field: false,
        }
    }

    /// A button row with a custom text color and no chevron or badge.
    pub(crate) fn button(label: &'static str, color: iced::Color, message: Message) -> Self {
        Self {
            mark: None,
            label,
            label_color: Some(color),
            value_text: None,
            value_align: ValueAlign::Right,
            secondary: None,
            trailing: None,
            message: Some(message),
            is_input_field: false,
        }
    }

    /// A single-choice radio option row (displays label, and a checkmark Icon::CHECK on the right if selected).
    pub(crate) fn select_option(label: &'static str, is_selected: bool, message: Message) -> Self {
        Self {
            mark: None,
            label,
            label_color: None,
            value_text: None,
            value_align: ValueAlign::Right,
            secondary: None,
            trailing: if is_selected {
                Some(Icon::CHECK)
            } else {
                None
            },
            message: Some(message),
            is_input_field: false,
        }
    }

    /// A tile whose message is not "open this section" — a row that switches something instead.
    pub(crate) fn action(
        icon: Icon,
        color: style::IconColor,
        label: &'static str,
        message: Message,
    ) -> Self {
        Self {
            mark: Some(Mark { icon, color }),
            label,
            label_color: None,
            value_text: None,
            value_align: ValueAlign::Right,
            secondary: None,
            trailing: None,
            message: Some(message),
            is_input_field: false,
        }
    }

    /// A tile with an icon badge and an iOS switch toggle on the right.
    pub(crate) fn toggle(
        icon: Icon,
        color: style::IconColor,
        label: &'static str,
        on: bool,
        message: Message,
        theme: ThemeMode,
    ) -> Self {
        Self {
            mark: Some(Mark { icon, color }),
            label,
            label_color: None,
            value_text: None,
            value_align: ValueAlign::Right,
            secondary: Some(crate::pages::common::toggle(on, message, theme)),
            trailing: None,
            message: None,
            is_input_field: false,
        }
    }

    /// A standard read-only key-value information row (label on the left, value on the right).
    ///
    /// It has no outline border, no chevron, and no click wash. It is styled consistently with
    /// A standard key-value row with unified theme colors (defaults to right-aligned).
    pub(crate) fn key_value(label: &'static str, value: impl Into<String>) -> Self {
        Self {
            mark: None,
            label,
            label_color: None,
            value_text: Some(value.into()),
            value_align: ValueAlign::Right,
            secondary: None,
            trailing: None,
            message: None,
            is_input_field: false,
        }
    }

    /// Sets the alignment for the key-value text (Left or Right).
    pub(crate) fn value_align(mut self, align: ValueAlign) -> Self {
        self.value_align = align;
        self
    }

    /// A row that is a label and an outlined box its value is typed into (e.g. Wi-Fi password).
    pub(crate) fn field(label: &'static str) -> Self {
        Self {
            mark: None,
            label,
            label_color: None,
            value_text: None,
            value_align: ValueAlign::Right,
            secondary: None,
            trailing: None,
            message: None,
            is_input_field: true,
        }
    }

    /// The same tile, with something beside its label.
    pub(crate) fn secondary(mut self, secondary: impl Into<UI<'a>>) -> Self {
        self.secondary = Some(secondary.into());
        self
    }

    /// The row, as a widget.
    pub(crate) fn view(self, theme: ThemeMode) -> UI<'a> {
        let Tile {
            mark,
            label,
            label_color,
            value_text,
            value_align,
            secondary,
            trailing,
            message,
            is_input_field,
        } = self;

        let mut left: Vec<UI<'a>> = Vec::new();

        if let Some(mark) = mark {
            left.push(badge(mark.icon, mark.color, style::Badge::ROW));
            left.push(Space::new().width(Length::Fixed(style::BADGE_GAP)).into());
        }

        let text_color = label_color.unwrap_or_else(|| style::label_for(theme));
        left.push(
            text(label)
                .size(style::LABEL_FONT)
                .color(text_color)
                .into(),
        );

        let left = Row::with_children(left).align_y(Alignment::Center);

        // An input field (e.g. Wi-Fi password entry) has an outlined box for typing into.
        if is_input_field {
            let mut row: Vec<UI<'a>> = vec![left.into()];

            if let Some(secondary) = secondary {
                row.push(Space::new().width(Length::Fixed(style::VALUE_GAP)).into());
                row.push(secondary);
            }

            return container(
                Row::with_children(row)
                    .align_y(Alignment::Center)
                    .width(Length::Fill),
            )
            .width(Length::Fill)
            .padding([style::ROW_PADDING_V, style::ROW_PADDING_H])
            .style(move |_theme| container::Style {
                border: Border {
                    color: style::separator_for(theme),
                    width: style::CARD_BORDER_WIDTH,
                    radius: style::FIELD_RADIUS.into(),
                },
                ..container::Style::default()
            })
            .into();
        }

        // Prepare the value element, either from value_text or secondary:
        let value_elem: Option<UI<'a>> = if let Some(val_str) = value_text {
            let align_x = match value_align {
                ValueAlign::Left => iced::alignment::Horizontal::Left,
                ValueAlign::Right => iced::alignment::Horizontal::Right,
            };
            Some(
                container(
                    text(val_str)
                        .size(style::VALUE_FONT)
                        .color(style::muted_for(theme))
                        .wrapping(text::Wrapping::Glyph)
                        .align_x(align_x),
                )
                .max_width(280.0)
                .into(),
            )
        } else {
            secondary
        };

        let contents = if value_align == ValueAlign::Left && value_elem.is_some() {
            let mut row: Vec<UI<'a>> = vec![
                left.into(),
                Space::new().width(Length::Fixed(style::VALUE_GAP * 2.0)).into(),
                value_elem.unwrap(),
                Space::new().width(Length::Fill).into(),
            ];
            if let Some(trailing) = trailing {
                let icon_color = if trailing == Icon::CHECK {
                    style::blue()
                } else {
                    style::chevron_for(theme)
                };
                row.push(
                    text(trailing.glyph())
                        .font(pomelo_material_symbols::font())
                        .size(style::CHEVRON_FONT)
                        .color(icon_color)
                        .into(),
                );
            }
            Row::with_children(row).align_y(Alignment::Center).width(Length::Fill)
        } else {
            let mut right: Vec<UI<'a>> = Vec::new();

            if let Some(val) = value_elem {
                right.push(val);
                right.push(Space::new().width(Length::Fixed(style::VALUE_GAP)).into());
            }

            if let Some(trailing) = trailing {
                let icon_color = if trailing == Icon::CHECK {
                    style::blue()
                } else {
                    style::chevron_for(theme)
                };
                right.push(
                    text(trailing.glyph())
                        .font(pomelo_material_symbols::font())
                        .size(style::CHEVRON_FONT)
                        .color(icon_color)
                        .into(),
                );
            }

            Row::with_children(vec![
                left.into(),
                Space::new().width(Length::Fill).into(),
                Row::with_children(right).align_y(Alignment::Center).into(),
            ])
            .align_y(Alignment::Center)
            .width(Length::Fill)
        };

        if let Some(message) = message {
            let btn = button(
                container(contents)
                    .width(Length::Fill)
                    .padding([style::ROW_PADDING_V, style::ROW_PADDING_H]),
            )
            .width(Length::Fill)
            .padding(0)
            .style(move |_theme, status| row_style(theme, status))
            .on_press(message);

            pomelo_widgets::drag_disambiguator(btn).into()
        } else {
            container(contents)
                .width(Length::Fill)
                .padding([style::ROW_PADDING_V, style::ROW_PADDING_H])
                .into()
        }
    }
}

/// A card: a rounded surface holding a column, with a hairline between two tiles.
///
/// A builder and not a widget — what a card *is* is `container` plus `Column`, and both already
/// exist. Implementing `Widget` would buy tree-level control (a tag, state, an overlay) that a
/// column of things does not want. iced ships no `TileGroup` either, for the same reason: a list is
/// a layout, and layouts are composed.
pub(crate) struct Card<'a> {
    children: Vec<UI<'a>>,
    theme: ThemeMode,
}

impl<'a> Card<'a> {
    pub(crate) fn new(theme: ThemeMode) -> Self {
        Self {
            children: Vec::new(),
            theme,
        }
    }


    /// Adds a tile, with a hairline above it unless it is the card's first child.
    ///
    /// Above, and not below: "not the last one" is a fact about a card's *end*, which the loop that
    /// builds it knows and a card does not; "not the first one" is a fact about its *contents*,
    /// which is exactly what a builder has.
    pub(crate) fn tile(mut self, tile: Tile<'a>) -> Self {
        if !self.children.is_empty() {
            self.children.push(separator(self.theme));
        }

        self.children.push(tile.view(self.theme));
        self
    }

    pub(crate) fn view(self) -> UI<'a> {
        card_surface(
            Column::with_children(self.children).width(Length::Fill),
            self.theme,
        )
    }

    pub(crate) fn into_group(self) -> Group<'a> {
        Group::from_surface(self.view())
    }
}

impl<'a> From<Card<'a>> for Group<'a> {
    fn from(card: Card<'a>) -> Self {
        card.into_group()
    }
}

impl<'a> From<Card<'a>> for PageItem<'a> {
    fn from(card: Card<'a>) -> Self {
        PageItem::Group(card.into())
    }
}

/// The head of a sub-page: the section's icon, the page's title beside it, and the way back above
/// both — the arrow [`capsule_button`] draws, in [`nav_style`]'s fill.
///
/// The same card the list is made of, holding no tiles at all — which is why [`Card`] is a surface
/// and not a group of tiles. It is what lets a sub-page go without a navigation bar: the heading
/// belongs *in* the page and scrolls with it, and a bar is a thing that covers content.
///
/// The icon is the one the list row carried, in the fill that row carried, so walking in from the
/// list and arriving here shows the same square in the same colour.
/// The navigation back bar at the top of a sub-page.
///
/// Contains the capsule back button and standard top/bottom spacing.
pub(crate) fn back_bar<'a>(theme: ThemeMode) -> UI<'a> {
    let nav = Row::with_children(vec![
        capsule_button(Icon::ARROW_BACK, Some(Message::Back), nav_style, theme),
        Space::new().width(Length::Fill).into(),
    ])
    .width(Length::Fill);

    container(nav)
        .width(Length::Fill)
        .padding(Padding {
            top: style::NAV_TOP_GAP,
            bottom: style::NAV_GAP,
            left: style::PAGE_MARGIN,
            right: style::PAGE_MARGIN,
        })
        .into()
}

/// The large icon badge and title card for major section sub-pages.
///
/// Placed as an independent card inside a page's `body`, separate from the navigation back bar.
pub(crate) fn section_card<'a>(
    icon: Icon,
    section: SettingsSection,
    title: &'static str,
    theme: ThemeMode,
) -> Group<'a> {
    let title_row = Row::with_children(vec![
        badge(icon, style::IconColor::for_section(section), style::Badge::HEAD),
        Space::new().width(Length::Fixed(style::HEAD_GAP)).into(),
        text(title)
            .size(style::TITLE_FONT)
            .color(style::label_for(theme))
            .into(),
    ])
    .align_y(Alignment::Center)
    .width(Length::Fill);

    Group::from_surface(card_surface(container(title_row).padding(style::HEAD_PADDING), theme))
}

/// The icon square at the left of a tile or at the head of a page: a glyph out of the icon font, on
/// a solid of the palette.
fn badge<'a>(icon: Icon, color: style::IconColor, size: style::Badge) -> UI<'a> {
    container(
        text(icon.glyph())
            .font(pomelo_material_symbols::font())
            .size(size.glyph)
            .color(Color::WHITE),
    )
    .width(Length::Fixed(size.side))
    .height(Length::Fixed(size.side))
    .center_x(Length::Fixed(size.side))
    .center_y(Length::Fixed(size.side))
    .style(move |_theme| container::Style {
        background: Some(color.color().into()),
        border: Border {
            radius: size.radius.into(),
            ..Border::default()
        },
        ..container::Style::default()
    })
    .into()
}
