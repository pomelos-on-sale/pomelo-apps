//! Repeat schedule selection subpage (重复页面).
//!
//! Reuses [`Section`] and [`ItemGroup`] from `pomelo_widgets` to display the 7 days of the week
//! (Sunday through Saturday). Tapping any day toggles its selection state with
//! an iOS-style checkmark. Defaults to none selected ("永不" / Never).

use iced::widget::{column, container, scrollable, text, Row, Space};
use iced::{Alignment, Element, Length, Padding};
use pomelo_material_symbols::Icon;
use pomelo_widgets::{capsule_icon_button, ActionStyle, Item, ItemGroup, Language, Section, SystemPreferences};

use crate::model::{AlarmItem, DAY_NAMES};
use crate::Message;

/// Renders the Repeat schedule subpage.
pub fn view_repeat_page<'a>(
    draft: &'a AlarmItem,
    preferences: &SystemPreferences,
) -> Element<'a, Message> {
    let theme = preferences.theme;
    let lang = preferences.language;

    // Top Bar with back capsule button and centered title
    let back_btn = capsule_icon_button(
        Icon::ARROW_BACK,
        Some(Message::CloseSubSheet),
        ActionStyle::Secondary,
        true,
        theme,
    );

    let title_str = match lang {
        Language::Chinese => "重复",
        Language::English => "Repeat",
    };

    let title_widget = text(title_str)
        .size(24.0)
        .color(theme.text_primary());

    let top_bar = Row::new()
        .width(Length::Fill)
        .align_y(Alignment::Center)
        .push(back_btn)
        .push(
            container(title_widget)
                .width(Length::Fill)
                .center_x(Length::Fill),
        )
        // Symmetric 72px space to balance top-left capsule button
        .push(Space::new().width(Length::Fixed(72.0)));

    let top_container = container(top_bar)
        .width(Length::Fill)
        .padding(Padding {
            top: 14.0,
            bottom: 6.0,
            left: 16.0,
            right: 16.0,
        });

    // 7-day selection section
    let mut group = ItemGroup::new();
    for idx in 0..7 {
        let label = match lang {
            Language::Chinese => DAY_NAMES[idx].0,
            Language::English => DAY_NAMES[idx].1,
        };
        let is_selected = draft.repeat.is_day_selected(idx);
        group = group.push(Item::option(label, is_selected, Message::ToggleRepeatDay(idx)));
    }

    let content_col = column![
        Space::new().height(Length::Fixed(12.0)),
        Section::group(group, theme).view(),
        Space::new().height(Length::Fixed(40.0)),
    ]
    .width(Length::Fixed(440.0))
    .align_x(Alignment::Center);

    let body_scrollable = scrollable(
        container(content_col)
            .width(Length::Fill)
            .center_x(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill);

    column![top_container, body_scrollable]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
