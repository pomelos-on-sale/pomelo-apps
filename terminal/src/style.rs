//! Metrics and colours.
//!
//! The original terminal's numbers, so the two apps laid out alike: absolute type,
//! one 24 px cell per line, and the iOS dark keyboard palette by name. The prompt colours are
//! [`shell`](crate::shell)'s [`Rgb`](crate::shell::Rgb) roles, converted to iced here — the model
//! has no colour
//! type of its own.

use crate::shell;
use crate::shell::Rgb;
use iced::Color;

/// The panel the terminal is designed for.
pub const SCREEN: u32 = 480;

/// The transcript's horizontal padding on the 480 px panel (the round AMOLED's safe inset).
pub const PAGE_PADDING: f32 = 22.0;

/// The keyboard band: about 48% of the panel's height, clamped. The formula lives with the
/// keyboard, because the settings app sizes its own band the same way.
pub use pomelo_widgets::touch_keyboard::band_height as keyboard_height;

/// The terminal's type and line cell, from the shell module that owns them.
pub const FONT_SIZE: f32 = shell::TERM_FONT_SIZE;
pub const LINE_HEIGHT: f32 = shell::TERM_LINE_HEIGHT;

/// The active input line's cursor. The baked font has `█`, but iced's font subset does not, so the
/// cursor is a painted block rather than a glyph.
pub const CURSOR_WIDTH: f32 = 9.0;
pub const CURSOR_HEIGHT: f32 = 16.0;

/// Measures a string, in logical pixels.
///
/// This is deliberately an estimate: iced's text engine is cosmic-text (shaping, per-glyph
/// advances) and is not reachable from `view`, while the original measured exactly
/// with its baked font. The constant is the same 0.55 × font size the original fallback used,
/// which is close for the Latin subset at one size. It only decides where a transcript line wraps
/// and how much of the input line's tail is shown — never the layout of a key.
pub fn measure(text: &str) -> f32 {
    text.chars().count() as f32 * (FONT_SIZE * 0.55)
}

/// The model's [`Rgb`] as an iced colour.
pub fn color(rgb: Rgb) -> Color {
    Color::from_rgb8(rgb.r, rgb.g, rgb.b)
}

/// The page background: pure black, the VS Code terminal.
pub fn background() -> Color {
    rgb((0, 0, 0))
}

/// The three colours one key paints with, and the palette that picks them — the shared widgets',
/// so the terminal and the settings app paint a key the same way.
pub use pomelo_widgets::touch_keyboard::style::{palette, KeyPalette};

fn rgb((r, g, b): (u8, u8, u8)) -> Color {
    Color::from_rgb8(r, g, b)
}
