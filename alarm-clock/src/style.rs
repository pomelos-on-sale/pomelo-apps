//! iOS-inspired styling, color palette, and layout metrics for Alarm Clock.

use iced::widget::button;
use iced::{Border, Color, Shadow};

// iOS Palette Colors
pub const BG_BLACK: Color = Color::from_rgb(0.0, 0.0, 0.0);
pub const CARD_BG: Color = Color::from_rgb(28.0 / 255.0, 28.0 / 255.0, 30.0 / 255.0); // #1C1C1E
pub const CARD_PRESSED: Color = Color::from_rgb(44.0 / 255.0, 44.0 / 255.0, 46.0 / 255.0); // #2C2C2E
pub const BORDER_COLOR: Color = Color::from_rgb(44.0 / 255.0, 44.0 / 255.0, 46.0 / 255.0);

pub const ORANGE: Color = Color::from_rgb(1.0, 159.0 / 255.0, 10.0 / 255.0); // #FF9F0A (Apple Clock Orange)

pub const TEXT_PRIMARY: Color = Color::WHITE;
pub const TEXT_MUTED: Color = Color::from_rgb(142.0 / 255.0, 142.0 / 255.0, 147.0 / 255.0); // #8E8E93

pub const DANGER_RED: Color = Color::from_rgb(1.0, 69.0 / 255.0, 58.0 / 255.0); // #FF453A

// Typography & Metrics
pub const TIME_FONT_SIZE: f32 = 42.0;
pub const TITLE_FONT_SIZE: f32 = 22.0;
pub const HEADER_BTN_FONT_SIZE: f32 = 18.0;
pub const LABEL_FONT_SIZE: f32 = 16.0;
pub const SUBTITLE_FONT_SIZE: f32 = 14.0;

pub const CARD_RADIUS: f32 = 16.0;
pub const CARD_PAD_H: f32 = 18.0;
pub const CARD_PAD_V: f32 = 14.0;

/// Alarm row button style when tapping to edit.
pub fn alarm_row_btn_style(status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Pressed => CARD_PRESSED,
        _ => CARD_BG,
    };
    button::Style {
        background: Some(bg.into()),
        text_color: TEXT_PRIMARY,
        border: Border {
            color: BORDER_COLOR,
            width: 1.0,
            radius: CARD_RADIUS.into(),
        },
        shadow: Shadow::default(),
        snap: false,
    }
}
