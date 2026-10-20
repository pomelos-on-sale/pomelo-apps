//! Sub-page navigation top bar, section header banner, and badge primitives.

use iced::widget::{container, text, Row, Space};
use iced::{Alignment, Border, Color, Length, Padding};
use pomelo_material_symbols::Icon;
use pomelo_widgets::{Button, Section, ThemeMode};

use crate::pages::common::UI;
use crate::style;
use crate::{Message, SettingsSection};

/// The navigation back bar at the top of a sub-page.
///
/// Contains the capsule back button and standard top/bottom spacing.
pub(crate) fn back_bar<'a>(theme: ThemeMode) -> UI<'a> {
    let nav = Row::with_children(vec![
        Button::capsule(Icon::ARROW_BACK)
            .theme(theme)
            .on_press(Message::Back)
            .view(),
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


