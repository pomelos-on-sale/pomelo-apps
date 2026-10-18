//! iOS-inspired styling, color palette, and layout metrics for Alarm Clock.

use iced::widget::{button, container};
use iced::{Border, Color, Shadow};
use pomelo_widgets::ThemeMode;

// iOS Palette Colors
pub const BG_BLACK: Color = Color::from_rgb(0.0, 0.0, 0.0);
pub const CARD_BG: Color = Color::from_rgb(28.0 / 255.0, 28.0 / 255.0, 30.0 / 255.0); // #1C1C1E
pub const CARD_PRESSED: Color = Color::from_rgb(44.0 / 255.0, 44.0 / 255.0, 46.0 / 255.0); // #2C2C2E
pub const BORDER_COLOR: Color = Color::from_rgb(44.0 / 255.0, 44.0 / 255.0, 46.0 / 255.0);
pub const SEPARATOR_COLOR: Color = Color::from_rgb(58.0 / 255.0, 58.0 / 255.0, 60.0 / 255.0); // #3A3A3C

pub const ORANGE: Color = Color::from_rgb(255.0 / 255.0, 159.0 / 255.0, 10.0 / 255.0); // #FF9F0A (Apple Clock Orange)
pub const ORANGE_PRESSED: Color = Color::from_rgb(212.0 / 255.0, 130.0 / 255.0, 8.0 / 255.0);

pub const GREEN_ON: Color = Color::from_rgb(52.0 / 255.0, 199.0 / 255.0, 89.0 / 255.0); // #34C759
pub const GREEN_PRESSED: Color = Color::from_rgb(45.0 / 255.0, 170.0 / 255.0, 76.0 / 255.0);

pub const GRAY_OFF: Color = Color::from_rgb(57.0 / 255.0, 57.0 / 255.0, 61.0 / 255.0); // #39393D
pub const GRAY_OFF_PRESSED: Color = Color::from_rgb(44.0 / 255.0, 44.0 / 255.0, 46.0 / 255.0);

pub const TEXT_PRIMARY: Color = Color::WHITE;
pub const TEXT_MUTED: Color = Color::from_rgb(142.0 / 255.0, 142.0 / 255.0, 147.0 / 255.0); // #8E8E93
pub const TEXT_SUBTITLE: Color = Color::from_rgb(161.0 / 255.0, 161.0 / 255.0, 166.0 / 255.0);

pub const DANGER_RED: Color = Color::from_rgb(255.0 / 255.0, 69.0 / 255.0, 58.0 / 255.0); // #FF453A
pub const DANGER_RED_PRESSED: Color = Color::from_rgb(211.0 / 255.0, 47.0 / 255.0, 47.0 / 255.0);

// Layout & Typography
pub const SCREEN_WIDTH: f32 = 480.0;
pub const SCREEN_HEIGHT: f32 = 480.0;

pub const TIME_FONT_SIZE: f32 = 42.0;
pub const PICKER_FONT_SIZE: f32 = 44.0;
pub const RINGING_FONT_SIZE: f32 = 54.0;
pub const TITLE_FONT_SIZE: f32 = 22.0;
pub const HEADER_BTN_FONT_SIZE: f32 = 18.0;
pub const LABEL_FONT_SIZE: f32 = 16.0;
pub const SUBTITLE_FONT_SIZE: f32 = 14.0;

pub const CARD_RADIUS: f32 = 16.0;
pub const CARD_PAD_H: f32 = 18.0;
pub const CARD_PAD_V: f32 = 14.0;

// iOS Switch dimensions
pub const TOGGLE_W: f32 = 52.0;
pub const TOGGLE_H: f32 = 28.0;
pub const TOGGLE_RADIUS: f32 = 14.0;
pub const TOGGLE_INSET: f32 = 2.0;
pub const KNOB_SIZE: f32 = 24.0;
pub const KNOB_RADIUS: f32 = 12.0;

/// Top bar container styling.
pub fn top_bar_style(_theme_mode: ThemeMode) -> container::Style {
    container::Style {
        background: Some(BG_BLACK.into()),
        ..container::Style::default()
    }
}

/// Alarm card item container styling.
pub fn alarm_card_style(_theme_mode: ThemeMode) -> container::Style {
    container::Style {
        background: Some(CARD_BG.into()),
        border: Border {
            color: BORDER_COLOR,
            width: 1.0,
            radius: CARD_RADIUS.into(),
        },
        ..container::Style::default()
    }
}

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

/// Orange text button style (e.g. "编辑", "取消", "存储").
pub fn orange_text_btn_style(status: button::Status) -> button::Style {
    let color = match status {
        button::Status::Pressed => ORANGE_PRESSED,
        _ => ORANGE,
    };
    button::Style {
        background: None,
        text_color: color,
        border: Border::default(),
        shadow: Shadow::default(),
        snap: false,
    }
}

/// iOS Switch toggle button style.
pub fn toggle_btn_style(on: bool, status: button::Status) -> button::Style {
    let fill = match (on, status) {
        (true, button::Status::Pressed) => GREEN_PRESSED,
        (true, _) => GREEN_ON,
        (false, button::Status::Pressed) => GRAY_OFF_PRESSED,
        (false, _) => GRAY_OFF,
    };
    button::Style {
        background: Some(fill.into()),
        text_color: Color::WHITE,
        border: Border {
            radius: TOGGLE_RADIUS.into(),
            ..Border::default()
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

/// Stepper button style (+ / -) in time picker.
pub fn stepper_btn_style(status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Pressed => Color::from_rgb(60.0 / 255.0, 60.0 / 255.0, 65.0 / 255.0),
        _ => Color::from_rgb(44.0 / 255.0, 44.0 / 255.0, 46.0 / 255.0),
    };
    button::Style {
        background: Some(bg.into()),
        text_color: ORANGE,
        border: Border {
            radius: 8.0.into(),
            ..Border::default()
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

/// Tag/pill button style in edit mode.
pub fn pill_btn_style(selected: bool, status: button::Status) -> button::Style {
    let (bg, text_color) = if selected {
        match status {
            button::Status::Pressed => (ORANGE_PRESSED, Color::BLACK),
            _ => (ORANGE, Color::BLACK),
        }
    } else {
        match status {
            button::Status::Pressed => (
                Color::from_rgb(50.0 / 255.0, 50.0 / 255.0, 54.0 / 255.0),
                TEXT_PRIMARY,
            ),
            _ => (CARD_BG, TEXT_MUTED),
        }
    };

    button::Style {
        background: Some(bg.into()),
        text_color,
        border: Border {
            color: if selected { ORANGE } else { BORDER_COLOR },
            width: 1.0,
            radius: 12.0.into(),
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

/// Red delete button style in edit view.
pub fn delete_btn_style(status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Pressed => Color::from_rgba(255.0 / 255.0, 69.0 / 255.0, 58.0 / 255.0, 0.25),
        _ => Color::from_rgba(255.0 / 255.0, 69.0 / 255.0, 58.0 / 255.0, 0.12),
    };
    button::Style {
        background: Some(bg.into()),
        text_color: DANGER_RED,
        border: Border {
            color: DANGER_RED,
            width: 1.0,
            radius: CARD_RADIUS.into(),
        },
        shadow: Shadow::default(),
        snap: false,
    }
}
