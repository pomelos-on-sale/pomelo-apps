//! Metrics and colours.
//!
//! Two kinds of number live here, and the difference matters. It is the rule the original
//! calculator was laid out with, and this port had let it slip: a key's height was frozen at the
//! value the 480×480 panel was tuned with, so a taller window did not give taller keys — it gave
//! a band of empty page at the bottom.
//!
//! * **Shares of the page** (`*_SHARE`) — the *layout*. Every box takes its height from a flex
//!   weight whose value is the pixel value the design panel was tuned with, so the design is
//!   reproduced *exactly* at 480×480 and scales proportionally everywhere else: a taller window
//!   gives taller keys, a wider one wider keys. The page's weights are `20 : 130 : 10 : 300 : 20`,
//!   which is the panel — see `the_page_shares_are_the_design_panel`.
//! * **Absolute pixels** — the *type and the insets*: an 18 pt label, an 18 px card inset, a 16 px
//!   radius. Flutter's `fontSize` means the same thing on every window and so do these; resizing
//!   the window must not change them.

use iced::Color;

use crate::keys::{Kind, LAYOUT};

/// The panel the calculator is designed for.
pub const SCREEN: u32 = 480;

/// The band the design leaves above and below its content. Without it a shorter window would
/// stretch its content to the window and stop matching the design.
pub const BAND_SHARE: u16 = 20;

/// The display card.
///
/// 130 and not the 84 the keypad was tuned against: 20 + 130 + 10 + 300 + 20 is the 480 px panel
/// exactly, and a card that does not take its share is a page whose keys come out 59 px instead of
/// the 52 they were measured at.
pub const CARD_SHARE: u16 = 130;

/// The gap between the card and the keypad.
pub const CARD_GAP_SHARE: u16 = 10;

/// One key row.
pub const KEY_SHARE: u16 = 52;

/// The gap between two key rows.
pub const KEY_GAP_SHARE: u16 = 10;

/// The share of the page the whole keypad takes: one [`KEY_SHARE`] per row plus one
/// [`KEY_GAP_SHARE`] between each pair — 300 for the five rows of the design panel.
pub const KEYPAD_SHARE: u16 = keypad_share(LAYOUT.len());

/// [`KEYPAD_SHARE`] for a keypad of `rows` rows. A function so that the share cannot drift from the
/// table it describes.
const fn keypad_share(rows: usize) -> u16 {
    rows as u16 * KEY_SHARE + rows.saturating_sub(1) as u16 * KEY_GAP_SHARE
}

/// The side margin of the page. A key's *cell* carries half of [`COLUMN_GAP`] on each side, so the
/// content itself starts `PAGE_MARGIN + COLUMN_GAP / 2` from the edge and the outer gap matches the
/// inner ones.
pub const PAGE_MARGIN: f32 = 32.0;

/// The distance between two keys in a row. Carried by the cells rather than by spacers between
/// them: that is what keeps a row with fewer gaps — the `0` row — lined up with the rows above.
pub const COLUMN_GAP: f32 = 16.0;

/// The display card's insets: wide, but shallow. It holds a 34 pt line and a 14 pt one, which is
/// 64 px of content, so 18 px of padding on *every* side used to push the second line outside the
/// card's own height.
pub const CARD_PAD_H: f32 = 18.0;
pub const CARD_PAD_V: f32 = 10.0;
pub const CARD_RADIUS: f32 = 10.0;
pub const PRIMARY_FONT: f32 = 34.0;
pub const SECONDARY_FONT: f32 = 14.0;

/// The gap between the display's two lines.
pub const LINE_GAP: f32 = 6.0;

/// A key's corner radius and the size of its label.
///
/// 18 is one of the three sizes the platform bakes glyphs for; a key label was 22, and 22 is not
/// baked, so every keypad's first draw rasterised its own digits.
pub const KEY_RADIUS: f32 = 16.0;
pub const KEY_FONT: f32 = 18.0;

/// The three colours one key paints with.
pub struct KeyPalette {
    pub fill: Color,
    pub pressed: Color,
    pub text: Color,
}

/// The colours for each role.
pub fn palette(kind: Kind) -> KeyPalette {
    let (fill, pressed) = match kind {
        Kind::Function => ((44, 44, 46), (58, 58, 60)),
        Kind::Number => ((28, 28, 30), (44, 44, 46)),
        Kind::Operator => ((255, 159, 10), (255, 179, 64)),
        Kind::Equals => ((10, 132, 255), (64, 156, 255)),
    };

    KeyPalette {
        fill: rgb(fill),
        pressed: rgb(pressed),
        text: Color::WHITE,
    }
}

/// The page background.
pub fn background() -> Color {
    rgb((10, 10, 12))
}

/// The display card.
pub fn card() -> Color {
    rgb((18, 18, 20))
}

fn rgb((r, g, b): (u8, u8, u8)) -> Color {
    Color::from_rgb8(r, g, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shares are the design's pixel values, so they have to add up to the panel: that is what
    /// "exact at 480, proportional everywhere else" means, and it is the first thing to break when
    /// a number is edited.
    #[test]
    fn the_page_shares_are_the_design_panel() {
        let page = 2 * BAND_SHARE + CARD_SHARE + CARD_GAP_SHARE + KEYPAD_SHARE;

        assert_eq!(
            page, SCREEN as u16,
            "the page's shares no longer add up to the design panel ({page} vs {SCREEN})"
        );
    }

    /// The card's two lines have to fit between its insets, or the second one is drawn outside the
    /// card and clipped away — which is what happened while the padding was 18 px on every side.
    #[test]
    fn the_card_can_hold_both_of_its_lines() {
        // iced lays a text out at 1.2 of its size, which is where these two numbers come from.
        let primary = (PRIMARY_FONT * 1.2).ceil();
        let secondary = (SECONDARY_FONT * 1.2).ceil();
        let content = primary + LINE_GAP + secondary;
        let room = CARD_SHARE as f32 - 2.0 * CARD_PAD_V;

        assert!(
            content <= room,
            "the card's two lines need {content} px and the card gives them {room}"
        );
    }
}
