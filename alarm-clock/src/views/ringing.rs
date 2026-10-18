//! Active alarm ringing full-screen overlay (闹钟响铃界面).
//!
//! Designed for immediate recognition and easy dismissal/snoozing
//! even when waking up half-asleep:
//! - Prominent alarm icon
//! - Large time and label
//! - Distinct Snooze (稍后提醒) and Dismiss (停止) touch buttons

use iced::widget::{button, column, container, text, Space};
use iced::{Alignment, Border, Color, Element, Length, Shadow};
use pomelo_material_symbols::Icon;
use pomelo_widgets::{icon, Language, SystemPreferences};

use crate::style;
use crate::Message;

/// Renders the full-screen active ringing overlay.
pub fn view_ringing<'a>(
    time_str: &str,
    label: &str,
    preferences: &SystemPreferences,
) -> Element<'a, Message> {
    let lang = preferences.language;

    let snooze_str = match lang {
        Language::Chinese => "稍后提醒 (5分钟)",
        Language::English => "Snooze (5 min)",
    };

    let stop_str = match lang {
        Language::Chinese => "停止",
        Language::English => "Stop",
    };

    let alarm_icon = icon(Icon::ALARM)
        .size(64.0)
        .color(style::ORANGE);

    let time_text = text(time_str.to_string())
        .size(56.0)
        .color(style::TEXT_PRIMARY);

    let label_text = text(label.to_string())
        .size(style::LABEL_FONT_SIZE)
        .color(style::TEXT_MUTED);

    let snooze_btn = button(
        container(
            text(snooze_str)
                .size(style::HEADER_BTN_FONT_SIZE)
                .color(style::TEXT_PRIMARY),
        )
        .width(Length::Fill)
        .align_x(Alignment::Center),
    )
    .width(Length::Fixed(320.0))
    .padding([14.0, 24.0])
    .style(|_, status| {
        let bg = match status {
            button::Status::Pressed => Color::from_rgb8(58, 58, 60),
            _ => Color::from_rgb8(44, 44, 46),
        };
        button::Style {
            background: Some(bg.into()),
            text_color: style::TEXT_PRIMARY,
            border: Border {
                radius: 16.0.into(),
                ..Border::default()
            },
            shadow: Shadow::default(),
            snap: false,
        }
    })
    .on_press(Message::SnoozeRinging);

    let stop_btn = button(
        container(
            text(stop_str)
                .size(22.0)
                .color(Color::WHITE),
        )
        .width(Length::Fill)
        .align_x(Alignment::Center),
    )
    .width(Length::Fixed(320.0))
    .padding([16.0, 24.0])
    .style(|_, status| {
        let bg = match status {
            button::Status::Pressed => Color::from_rgb8(230, 80, 0),
            _ => style::ORANGE,
        };
        button::Style {
            background: Some(bg.into()),
            text_color: Color::WHITE,
            border: Border {
                radius: 16.0.into(),
                ..Border::default()
            },
            shadow: Shadow::default(),
            snap: false,
        }
    })
    .on_press(Message::DismissRinging);

    let col = column![
        Space::new().height(Length::Fixed(40.0)),
        alarm_icon,
        Space::new().height(Length::Fixed(16.0)),
        time_text,
        Space::new().height(Length::Fixed(6.0)),
        label_text,
        Space::new().height(Length::Fill),
        snooze_btn,
        Space::new().height(Length::Fixed(16.0)),
        stop_btn,
        Space::new().height(Length::Fixed(40.0)),
    ]
    .align_x(Alignment::Center)
    .width(Length::Fill);

    container(col)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .style(|_| container::Style {
            background: Some(style::BG_BLACK.into()),
            ..container::Style::default()
        })
        .into()
}
