//! Touch-scrollable digital number wheel roller (数字滚轮) for hours and minutes.

use iced::widget::{button, column, container, text, Row};
use iced::{Alignment, Border, Color, Element, Length, Shadow};
use pomelo_widgets::gesture_detector;

use crate::style;
use crate::Message;

/// Height of the top and bottom item in the roller.
const ITEM_HEIGHT: f32 = 42.0;
/// Height of the middle selected lens container.
const LENS_HEIGHT: f32 = 50.0;
/// Width of one roller column.
const ROLLER_WIDTH: f32 = 110.0;

/// Renders the hour and minute number rollers side-by-side with a ":" separator.
pub fn render_wheel_pickers<'a>(hour: u8, minute: u8) -> Element<'a, Message> {
    let hour_roller = render_roller(
        hour,
        24,
        Message::HourChange,
        Message::HourDrag,
        Message::HourDragEnd,
    );

    let colon = text(":")
        .size(36.0)
        .color(style::TEXT_MUTED);

    let minute_roller = render_roller(
        minute,
        60,
        Message::MinuteChange,
        Message::MinuteDrag,
        Message::MinuteDragEnd,
    );

    let content = Row::new()
        .align_y(Alignment::Center)
        .spacing(20.0)
        .push(hour_roller)
        .push(colon)
        .push(minute_roller);

    container(content)
        .width(Length::Fill)
        .center_x(Length::Fill)
        .padding([2.0, 16.0])
        .into()
}

/// Renders a single touch-scrollable number wheel.
fn render_roller<'a>(
    current: u8,
    limit: u8,
    on_step: fn(i8) -> Message,
    on_drag: fn(f32) -> Message,
    on_drag_end: Message,
) -> Element<'a, Message> {
    let prev = (current as i16 - 1).rem_euclid(limit as i16) as u8;
    let next = (current as i16 + 1).rem_euclid(limit as i16) as u8;

    // Top item (prev) - tap to step -1
    let prev_btn = button(
        container(
            text(format!("{:02}", prev))
                .size(24.0)
                .color(Color::from_rgb8(142, 142, 147)),
        )
        .center_x(Length::Fixed(ROLLER_WIDTH))
        .center_y(Length::Fixed(ITEM_HEIGHT)),
    )
    .width(Length::Fixed(ROLLER_WIDTH))
    .height(Length::Fixed(ITEM_HEIGHT))
    .padding(0)
    .style(|_, status| {
        let bg = match status {
            button::Status::Pressed => Color::from_rgb8(38, 38, 40),
            _ => Color::TRANSPARENT,
        };
        button::Style {
            background: Some(bg.into()),
            text_color: Color::WHITE,
            border: Border::default(),
            shadow: Shadow::default(),
            snap: false,
        }
    })
    .on_press(on_step(-1));

    // Middle selected item with rounded highlight lens
    let selected_lens = container(
        text(format!("{:02}", current))
            .size(40.0)
            .color(style::TEXT_PRIMARY),
    )
    .center_x(Length::Fixed(ROLLER_WIDTH))
    .center_y(Length::Fixed(LENS_HEIGHT))
    .style(|_| container::Style {
        background: Some(Color::from_rgb8(44, 44, 46).into()),
        border: Border {
            radius: 12.0.into(),
            ..Border::default()
        },
        ..container::Style::default()
    });

    // Bottom item (next) - tap to step +1
    let next_btn = button(
        container(
            text(format!("{:02}", next))
                .size(24.0)
                .color(Color::from_rgb8(142, 142, 147)),
        )
        .center_x(Length::Fixed(ROLLER_WIDTH))
        .center_y(Length::Fixed(ITEM_HEIGHT)),
    )
    .width(Length::Fixed(ROLLER_WIDTH))
    .height(Length::Fixed(ITEM_HEIGHT))
    .padding(0)
    .style(|_, status| {
        let bg = match status {
            button::Status::Pressed => Color::from_rgb8(38, 38, 40),
            _ => Color::TRANSPARENT,
        };
        button::Style {
            background: Some(bg.into()),
            text_color: Color::WHITE,
            border: Border::default(),
            shadow: Shadow::default(),
            snap: false,
        }
    })
    .on_press(on_step(1));

    let roller_col = column![prev_btn, selected_lens, next_btn]
        .width(Length::Fixed(ROLLER_WIDTH))
        .align_x(Alignment::Center)
        .spacing(2.0);

    // Wrap in gesture detector for touch scrolling and flicks
    gesture_detector(roller_col)
        .on_pan_update(move |details| on_drag(details.delta.y))
        .on_pan_end(move |_| on_drag_end.clone())
        .on_swipe_up(on_step(1))
        .on_swipe_down(on_step(-1))
        .into()
}
