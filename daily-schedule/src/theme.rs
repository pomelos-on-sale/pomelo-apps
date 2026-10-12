use iced::theme::Palette;
use iced::{Color, Theme};

/// 创建纯黑主题：背景为绝对纯黑 Color::BLACK (#000000)，前景文本为纯白
pub fn pure_black() -> Theme {
    Theme::custom(
        "pure_black",
        Palette {
            background: Color::BLACK,
            text: Color::WHITE,
            ..Palette::DARK
        },
    )
}
