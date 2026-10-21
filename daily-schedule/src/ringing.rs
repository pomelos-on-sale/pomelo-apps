//! Active calendar event ringing full-screen view (事件开始闹钟响铃界面).
//!
//! Fully self-contained within daily-schedule to allow tailored calendar customizations:
//! - Prominent calendar event icon (Icon::EVENT_NOTE)
//! - Clear reminder header and large event title
//! - Event time duration span (e.g. "14:30 - 15:30")
//! - Easy-to-touch full-width "Confirm / Stop" dismiss button

use iced::widget::{button, column, container, text, Space};
use iced::{Alignment, Border, Color, Element, Length, Shadow};
use pomelo_material_symbols::Icon;
use pomelo_widgets::{icon, FontSizeTier, Language, SystemPreferences};

use crate::app::{Message, RingingState};

const BG_BLACK: Color = Color::from_rgb(0.0, 0.0, 0.0);
const COLOR_ACCENT: Color = Color::from_rgb8(255, 149, 0); // Warm alert amber/orange
const TEXT_PRIMARY: Color = Color::from_rgb8(242, 242, 247);
const TEXT_MUTED: Color = Color::from_rgb8(152, 152, 157);

/// Renders the full-screen calendar event ringing overlay.
pub fn view_ringing<'a>(
    state: &RingingState,
    preferences: &SystemPreferences,
) -> Element<'a, Message> {
    let lang = preferences.language;

    let reminder_badge = match lang {
        Language::Chinese => "日程闹钟",
        Language::English => "SCHEDULE ALARM",
    };

    let stop_str = match lang {
        Language::Chinese => "确认停止",
        Language::English => "Dismiss",
    };

    let event_icon = icon(Icon::EVENT_NOTE)
        .size(64.0)
        .color(COLOR_ACCENT);

    let badge_text = text(reminder_badge)
        .size(FontSizeTier::Small.base_size())
        .color(COLOR_ACCENT);

    let title_text = text(state.title.clone())
        .size(FontSizeTier::Large.base_size() + 4.0)
        .color(TEXT_PRIMARY);

    let time_text = text(state.time_range_str.clone())
        .size(FontSizeTier::Standard.base_size())
        .color(TEXT_MUTED);

    let stop_btn = button(
        container(
            text(stop_str)
                .size(FontSizeTier::Standard.base_size() + 2.0)
                .color(Color::WHITE),
        )
        .width(Length::Fill)
        .align_x(Alignment::Center),
    )
    .width(Length::Fixed(320.0))
    .padding([16.0, 24.0])
    .style(|_, status| {
        let bg = match status {
            button::Status::Pressed => Color::from_rgb8(230, 100, 0),
            _ => COLOR_ACCENT,
        };
        button::Style {
            background: Some(bg.into()),
            text_color: Color::WHITE,
            border: Border {
                radius: 18.0.into(),
                ..Border::default()
            },
            shadow: Shadow::default(),
            snap: false,
        }
    })
    .on_press(Message::DismissRinging);

    let col = column![
        Space::new().height(Length::Fixed(44.0)),
        event_icon,
        Space::new().height(Length::Fixed(12.0)),
        badge_text,
        Space::new().height(Length::Fixed(16.0)),
        title_text,
        Space::new().height(Length::Fixed(8.0)),
        time_text,
        Space::new().height(Length::Fill),
        stop_btn,
        Space::new().height(Length::Fixed(44.0)),
    ]
    .align_x(Alignment::Center)
    .width(Length::Fill);

    container(col)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .style(|_| container::Style {
            background: Some(BG_BLACK.into()),
            ..container::Style::default()
        })
        .into()
}
