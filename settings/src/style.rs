//! Metrics and colours.
//!
//! The original settings app's numbers, so that the two apps laid out alike:
//! absolute type, fluid boxes. Only [`SCREEN`] is about the panel; everything else is a
//! measurement of the design, and a wider screen gets wider boxes, never bigger text.

use iced::Color;

use crate::SettingsSection;

/// The panel the app is designed for.
pub const SCREEN: u32 = 480;

/// The page: its margins, and the gaps between the blocks of a list.
pub const PAGE_MARGIN: f32 = 20.0;
pub const BLOCK_GAP: f32 = 16.0;
pub const FOOTNOTE_GAP: f32 = 20.0;
pub const BOTTOM_GAP: f32 = 24.0;
pub const MAIN_BOTTOM_GAP: f32 = 30.0;

use pomelo_widgets::{FontSizeTier, SystemPreferences};

/// The font size tiers obtained from SystemPreferences: [18.0, 20.0, 24.0, 30.0].
pub const FONT_SIZES: [f32; 4] = SystemPreferences::font_sizes();
pub const FONT_EXTRA_SMALL: f32 = FontSizeTier::ExtraSmall.base_size(); // 18.0 px (Compact tier)
pub const FONT_SMALL: f32 = FontSizeTier::Small.base_size();            // 20.0 px (Small tier)
pub const FONT_STANDARD: f32 = FontSizeTier::Standard.base_size();      // 24.0 px (Standard tier)
pub const FONT_LARGE: f32 = FontSizeTier::Large.base_size();            // 30.0 px (Large tier)

/// The navigation bar: the page's one fixed measurement.
pub const NAV_HEIGHT: f32 = 44.0;
pub const NAV_PADDING: f32 = 20.0;
pub const NAV_FONT: f32 = FONT_STANDARD;
pub const BACK_FONT: f32 = FONT_SMALL;
pub const BACK_PADDING_V: f32 = 6.0;
pub const BACK_PADDING_H: f32 = 10.0;
pub const BACK_RADIUS: f32 = 8.0;

/// A grouped card.
pub const CARD_RADIUS: f32 = 14.0;
pub const CARD_BORDER_WIDTH: f32 = 1.0;

/// A row of the main list.
pub const ROW_PADDING_V: f32 = 12.0;
pub const ROW_PADDING_H: f32 = 14.0;
pub const ROW_SEPARATOR: f32 = 1.0;

/// A row of a detail card.
pub const DETAIL_PADDING_V: f32 = 14.0;
pub const DETAIL_PADDING_H: f32 = 16.0;
pub const DETAIL_FONT: f32 = FONT_SMALL;

/// A switch row.
pub const SWITCH_PADDING_V: f32 = 14.0;
pub const SWITCH_PADDING_H: f32 = 16.0;

/// The badge square at the left of a main-list row.
pub const BADGE: f32 = 28.0;
pub const BADGE_RADIUS: f32 = 6.0;
pub const BADGE_FONT: f32 = FONT_SMALL;
pub const BADGE_GAP: f32 = 12.0;

/// The main list's typography sourced from SystemPreferences tiers.
pub const LABEL_FONT: f32 = FONT_STANDARD;
pub const VALUE_FONT: f32 = FONT_SMALL;
pub const VALUE_GAP: f32 = 8.0;
pub const CHEVRON_FONT: f32 = FONT_SMALL;
pub const FOOTNOTE_FONT: f32 = FONT_SMALL;

/// The switch: a 52x28 track with a 24px knob, inset 2px from the edge it rests against.
pub const TOGGLE_W: f32 = 52.0;
pub const TOGGLE_H: f32 = 28.0;
pub const TOGGLE_RADIUS: f32 = 14.0;
pub const TOGGLE_INSET: f32 = 2.0;
pub const KNOB: f32 = 24.0;
pub const KNOB_RADIUS: f32 = 12.0;

/// A usage bar.
pub const USAGE_PADDING: f32 = 16.0;
pub const BAR_GAP: f32 = 10.0;
pub const BAR_HEIGHT: f32 = 10.0;
pub const BAR_RADIUS: f32 = 5.0;

/// The palette chips of the theme page.
pub const CHIP_W: f32 = 60.0;
pub const CHIP_H: f32 = 40.0;
pub const CHIP_RADIUS: f32 = 10.0;
pub const CHIP_GAP: f32 = 10.0;
pub const PALETTE_PADDING: f32 = 16.0;

/// A network's row in the Wi-Fi list, and the four bars that stand for its signal.
///
/// Four, not three: `ApInfo::signal_bars()` in the HAL answers on a `0..=4` scale, and a scale
/// drawn with one bar fewer than it is measured in is a bar that can never light.
pub const SIGNAL_BARS: u8 = 4;
pub const SIGNAL_BAR_W: f32 = 3.0;
pub const SIGNAL_BAR_GAP: f32 = 2.0;
pub const SIGNAL_BAR_H: f32 = 14.0;
pub const SIGNAL_BAR_MIN_H: f32 = 4.0;
pub const SIGNAL_GAP: f32 = 10.0;

/// The password prompt: a dimmed page, a card on it, and the keyboard inside the card.
pub const PROMPT_MARGIN: f32 = 16.0;
pub const PROMPT_PADDING: f32 = 16.0;
pub const PROMPT_GAP: f32 = 12.0;
pub const PROMPT_RADIUS: f32 = 16.0;

/// The prompt's keyboard is this tall.
///
/// The keyboard itself is flex — it fills whatever box it is given, in both axes — so the height of
/// the box is stated here, on the layout, the way every other measurement of this page is. There is
/// no matching width: the card is the screen less its margins, and the keyboard fills the card.
pub const PROMPT_BAND_H: f32 = 200.0;

/// The typed password: one dot per character, and a caret after them. The font is a Chinese and
/// Latin subset with no `•` in it, so the dots are painted rather than typed — the same reason the
/// terminal's cursor is a block.
pub const PASSWORD_DOT: f32 = 9.0;
pub const PASSWORD_DOT_GAP: f32 = 7.0;
pub const CARET_W: f32 = 2.0;
pub const CARET_H: f32 = FONT_SMALL;

use pomelo_widgets::ThemeMode;

/// The page background: pure AMOLED black, so an unlit pixel costs nothing.
pub fn black() -> Color {
    rgb((0, 0, 0))
}

pub fn background_for(theme: ThemeMode) -> Color {
    theme.background()
}

/// The navigation bar, a shade above the black page.
pub fn nav() -> Color {
    nav_for(ThemeMode::Dark)
}

pub fn nav_for(theme: ThemeMode) -> Color {
    theme.nav_bar()
}

/// A grouped card.
pub fn card() -> Color {
    card_for(ThemeMode::Dark)
}

pub fn card_for(theme: ThemeMode) -> Color {
    theme.card()
}

/// The hairline around a card.
pub fn card_border() -> Color {
    card_border_for(ThemeMode::Dark)
}

pub fn card_border_for(theme: ThemeMode) -> Color {
    theme.border()
}

/// The hairline between two rows of a card.
pub fn separator() -> Color {
    separator_for(ThemeMode::Dark)
}

pub fn separator_for(theme: ThemeMode) -> Color {
    theme.separator()
}

/// The accent, used by the back button -- and by a test to find it.
pub fn accent() -> Color {
    rgb((10, 132, 255))
}

/// The accent under a finger.
pub fn accent_pressed() -> Color {
    rgb((64, 156, 255))
}

/// Primary text label colour.
pub fn label_for(theme: ThemeMode) -> Color {
    theme.text_primary()
}

/// A value on the right of a row.
pub fn muted() -> Color {
    muted_for(ThemeMode::Dark)
}

pub fn muted_for(theme: ThemeMode) -> Color {
    theme.text_secondary()
}

/// The chevron that says a row opens.
pub fn chevron() -> Color {
    chevron_for(ThemeMode::Dark)
}

pub fn chevron_for(theme: ThemeMode) -> Color {
    match theme {
        ThemeMode::Dark => rgb((100, 100, 105)),
        ThemeMode::Light => rgb((160, 160, 165)),
    }
}

/// The key of a detail row.
pub fn detail_key() -> Color {
    detail_key_for(ThemeMode::Dark)
}

pub fn detail_key_for(theme: ThemeMode) -> Color {
    match theme {
        ThemeMode::Dark => rgb((180, 180, 185)),
        ThemeMode::Light => rgb((100, 100, 105)),
    }
}

/// The main list's footnote.
pub fn footnote() -> Color {
    footnote_for(ThemeMode::Dark)
}

pub fn footnote_for(theme: ThemeMode) -> Color {
    match theme {
        ThemeMode::Dark => rgb((120, 120, 128)),
        ThemeMode::Light => rgb((142, 142, 147)),
    }
}

/// iOS green: the `on` track of a switch, and the battery badge.
pub fn green() -> Color {
    rgb((52, 199, 89))
}

/// The `on` track under a finger.
pub fn green_pressed() -> Color {
    rgb((92, 214, 125))
}

/// The `off` track of a switch.
pub fn track_off() -> Color {
    track_off_for(ThemeMode::Dark)
}

pub fn track_off_for(theme: ThemeMode) -> Color {
    match theme {
        ThemeMode::Dark => rgb((60, 60, 65)),
        ThemeMode::Light => rgb((229, 229, 234)),
    }
}

/// The `off` track under a finger.
pub fn track_off_pressed() -> Color {
    track_off_pressed_for(ThemeMode::Dark)
}

pub fn track_off_pressed_for(theme: ThemeMode) -> Color {
    match theme {
        ThemeMode::Dark => rgb((74, 74, 80)),
        ThemeMode::Light => rgb((209, 209, 214)),
    }
}

/// The empty part of a usage bar.
pub fn bar_track() -> Color {
    bar_track_for(ThemeMode::Dark)
}

pub fn bar_track_for(theme: ThemeMode) -> Color {
    match theme {
        ThemeMode::Dark => rgb((50, 50, 55)),
        ThemeMode::Light => rgb((229, 229, 234)),
    }
}

/// The memory bar.
pub fn memory_bar() -> Color {
    rgb((88, 86, 214))
}

/// The storage bar, and its badge.
pub fn storage_bar() -> Color {
    rgb((255, 149, 0))
}

/// The hairline around a palette chip.
pub fn chip_border() -> Color {
    chip_border_for(ThemeMode::Dark)
}

pub fn chip_border_for(theme: ThemeMode) -> Color {
    match theme {
        ThemeMode::Dark => rgba((255, 255, 255), 60.0 / 255.0),
        ThemeMode::Light => rgba((0, 0, 0), 40.0 / 255.0),
    }
}

/// A lit signal bar of a network's row.
pub fn signal_on() -> Color {
    signal_on_for(ThemeMode::Dark)
}

pub fn signal_on_for(theme: ThemeMode) -> Color {
    match theme {
        ThemeMode::Dark => rgba((255, 255, 255), 0.9),
        ThemeMode::Light => rgba((0, 0, 0), 0.85),
    }
}

/// The bars a network's signal does not reach.
pub fn signal_off() -> Color {
    signal_off_for(ThemeMode::Dark)
}

pub fn signal_off_for(theme: ThemeMode) -> Color {
    match theme {
        ThemeMode::Dark => rgba((255, 255, 255), 0.22),
        ThemeMode::Light => rgba((0, 0, 0), 0.18),
    }
}

/// The page behind the password prompt, dimmed: the prompt has the finger, not the list.
pub fn backdrop() -> Color {
    rgba((0, 0, 0), 0.6)
}

/// A row that names a state instead of offering an action -- "scanning…", "no networks found".
pub fn notice() -> Color {
    notice_for(ThemeMode::Dark)
}

pub fn notice_for(theme: ThemeMode) -> Color {
    match theme {
        ThemeMode::Dark => rgb((120, 120, 128)),
        ThemeMode::Light => rgb((142, 142, 147)),
    }
}

/// A failed connection attempt.
pub fn error() -> Color {
    rgb((255, 69, 58))
}

/// The badge colour of a main-list row.
///
/// Each row's badge is a solid, and therefore a colour a test can scan the panel for. `Main`
/// never draws one; it takes the neutral grey so the match stays total.
pub fn badge(section: SettingsSection) -> Color {
    match section {
        SettingsSection::Wifi => rgb((0, 122, 255)),
        SettingsSection::Memory => memory_bar(),
        SettingsSection::Storage => storage_bar(),
        SettingsSection::Battery => green(),
        SettingsSection::SystemInfo | SettingsSection::Main => rgb((142, 142, 147)),
        SettingsSection::Theme => rgb((48, 176, 199)),
        SettingsSection::Time => rgb((255, 45, 85)),
    }
}

/// The eight presets the theme page shows, in two rows of four.
pub fn palette() -> [Color; 8] {
    [
        rgb((0, 0, 0)),
        rgb((30, 58, 138)),
        rgb((6, 182, 212)),
        rgb((16, 185, 129)),
        rgb((245, 158, 11)),
        rgb((244, 63, 94)),
        rgb((168, 85, 247)),
        Color::WHITE,
    ]
}

fn rgb(color: (u8, u8, u8)) -> Color {
    Color::from_rgb8(color.0, color.1, color.2)
}

fn rgba(color: (u8, u8, u8), alpha: f32) -> Color {
    Color::from_rgba8(color.0, color.1, color.2, alpha)
}
