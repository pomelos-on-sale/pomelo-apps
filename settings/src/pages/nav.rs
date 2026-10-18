//! Sub-page navigation top bar, section header banner, and badge primitives.

use iced::widget::{button, container, text, Row, Space};
use iced::{Alignment, Border, Color, Length, Padding};
use pomelo_material_symbols::Icon;
use pomelo_widgets::{Section, ThemeMode};

use crate::pages::common::UI;
use crate::style;
use crate::{Message, SettingsSection};

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
/// Conforms to [`Section`] design with rounded card surface and padding.
pub(crate) fn section_card<'a>(
    icon: Icon,
    section: SettingsSection,
    title: &'static str,
    theme: ThemeMode,
) -> UI<'a> {
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

    Section::new(container(title_row).padding(style::HEAD_PADDING), theme).view()
}

/// The icon square at the left of a tile or at the head of a page: a glyph out of the icon font, on
/// a solid of the palette.
pub(crate) fn badge<'a>(icon: Icon, color: style::IconColor, size: style::Badge) -> UI<'a> {
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

/// A round-ended button carrying a glyph (e.g. back button).
pub(crate) fn capsule_button<'a>(
    icon: Icon,
    message: Option<Message>,
    look: fn(ThemeMode, button::Status) -> button::Style,
    theme: ThemeMode,
) -> UI<'a> {
    button(
        container(
            text(icon.glyph())
                .font(pomelo_material_symbols::font())
                .size(style::NAV_GLYPH),
        )
        .center_x(Length::Fixed(style::NAV_BUTTON_W))
        .center_y(Length::Fixed(style::NAV_BUTTON_H)),
    )
    .padding(0)
    .style(move |_theme, status| look(theme, status))
    .on_press_maybe(message)
    .into()
}

/// The pill shape a capsule button takes in the navigation bar.
pub(crate) fn nav_style(theme: ThemeMode, status: button::Status) -> button::Style {
    let fill = if matches!(status, button::Status::Pressed | button::Status::Hovered) {
        style::muted_for(theme)
    } else {
        style::card_for(theme)
    };

    button::Style {
        background: Some(fill.into()),
        text_color: style::label_for(theme),
        border: Border {
            radius: (style::NAV_BUTTON_H / 2.0).into(),
            ..Border::default()
        },
        shadow: iced::Shadow::default(),
        snap: false,
    }
}
