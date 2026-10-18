//! Main Alarm Clock list view (闹钟列表主页面).
//!
//! Inspired by iOS Clock design language:
//! - Top bar with "编辑" / "完成" and "+" add button
//! - Large digital time typography with subtitles
//! - Responsive toggle switch or delete button in list editing mode
//! - AMOLED pitch black surface with Apple Clock orange accents

use iced::widget::{button, column, container, scrollable, text, Column, Row, Space};
use iced::{Alignment, Border, Color, Element, Length, Shadow};
use pomelo_material_symbols::Icon;
use pomelo_widgets::{icon, toggle_switch, Language, SystemPreferences};

use crate::model::{AlarmConfig, AlarmItem};
use crate::style;
use crate::Message;

/// Renders the main alarm list view.
pub fn view_list<'a>(
    config: &'a AlarmConfig,
    is_editing_list: bool,
    preferences: &SystemPreferences,
) -> Element<'a, Message> {
    let lang = preferences.language;

    // Top Bar
    let edit_label = if is_editing_list {
        match lang {
            Language::Chinese => "完成",
            Language::English => "Done",
        }
    } else {
        match lang {
            Language::Chinese => "编辑",
            Language::English => "Edit",
        }
    };

    let edit_btn = button(
        text(edit_label)
            .size(style::HEADER_BTN_FONT_SIZE)
            .color(style::ORANGE),
    )
    .padding([6.0, 10.0])
    .style(|_, status| style::orange_text_btn_style(status))
    .on_press(Message::ToggleListEdit);

    let title_text = match lang {
        Language::Chinese => "闹钟",
        Language::English => "Alarms",
    };

    let title = text(title_text)
        .size(style::TITLE_FONT_SIZE)
        .color(style::TEXT_PRIMARY);

    let add_btn = button(
        icon(Icon::ADD)
            .size(24.0)
            .color(style::ORANGE),
    )
    .padding([6.0, 10.0])
    .style(|_, status| style::orange_text_btn_style(status))
    .on_press(Message::OpenNew);

    let top_bar = Row::new()
        .width(Length::Fill)
        .align_y(Alignment::Center)
        .push(edit_btn)
        .push(Space::new().width(Length::Fill))
        .push(title)
        .push(Space::new().width(Length::Fill))
        .push(add_btn);

    let top_container = container(top_bar)
        .width(Length::Fill)
        .padding([10.0, 20.0]);

    // Alarms list
    let mut list_col = Column::new()
        .width(Length::Fill)
        .spacing(12.0)
        .align_x(Alignment::Center);

    if config.alarms.is_empty() {
        let empty_text = match lang {
            Language::Chinese => "暂无闹钟\n点击右上角 + 添加",
            Language::English => "No Alarms\nTap + to add one",
        };
        let empty_view = container(
            column![
                icon(Icon::ALARM).size(48.0).color(style::TEXT_MUTED),
                Space::new().height(Length::Fixed(12.0)),
                text(empty_text)
                    .size(style::LABEL_FONT_SIZE)
                    .color(style::TEXT_MUTED)
                    .align_x(Alignment::Center),
            ]
            .align_x(Alignment::Center),
        )
        .width(Length::Fill)
        .height(Length::Fixed(240.0))
        .center_x(Length::Fill)
        .center_y(Length::Fill);

        list_col = list_col.push(empty_view);
    } else {
        for item in &config.alarms {
            list_col = list_col.push(render_alarm_card(item, is_editing_list, preferences));
        }
    }

    // Bottom spacer to clear the round AMOLED bezel
    list_col = list_col.push(Space::new().height(Length::Fixed(40.0)));

    let scrollable_content = scrollable(list_col)
        .width(Length::Fill)
        .height(Length::Fill);

    column![top_container, scrollable_content]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Renders an individual alarm card row.
fn render_alarm_card<'a>(
    item: &'a AlarmItem,
    is_editing_list: bool,
    preferences: &SystemPreferences,
) -> Element<'a, Message> {
    let lang = preferences.language;
    let theme = preferences.theme;

    let time_color = if item.enabled {
        style::TEXT_PRIMARY
    } else {
        style::TEXT_MUTED
    };

    let time_widget = text(item.time_string())
        .size(style::TIME_FONT_SIZE)
        .color(time_color);

    let subtitle_widget = text(item.subtitle_string(lang))
        .size(style::SUBTITLE_FONT_SIZE)
        .color(style::TEXT_MUTED);

    let info_col = column![time_widget, subtitle_widget].spacing(2.0);

    let action_widget: Element<'a, Message> = if is_editing_list {
        let del_btn = button(
            icon(Icon::DELETE)
                .size(24.0)
                .color(style::DANGER_RED),
        )
        .padding([8.0, 12.0])
        .style(|_, status| {
            let bg = match status {
                button::Status::Pressed => style::CARD_PRESSED,
                _ => Color::TRANSPARENT,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: style::DANGER_RED,
                border: Border::default(),
                shadow: Shadow::default(),
                snap: false,
            }
        })
        .on_press(Message::DeleteAlarm(item.id));
        del_btn.into()
    } else {
        toggle_switch(item.enabled, Message::ToggleAlarm(item.id), theme)
    };

    // Main row
    let row_content = Row::new()
        .width(Length::Fill)
        .align_y(Alignment::Center)
        .push(info_col)
        .push(Space::new().width(Length::Fill))
        .push(action_widget);

    let card_inner = container(row_content)
        .width(Length::Fill)
        .padding([style::CARD_PAD_V, style::CARD_PAD_H]);

    button(card_inner)
        .width(Length::Fixed(440.0))
        .padding(0)
        .style(|_, status| style::alarm_row_btn_style(status))
        .on_press(Message::OpenEdit(item.id))
        .into()
}
