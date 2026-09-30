//! Metrics and colors.
//!
//! Absolute, like the rest of this repository's UI code: the panel is 480×480, type must not
//! scale with it, and a box that should adapt says so with `Length::Fill`. The one place that
//! would change on another panel is [`SCREEN`], and even that is only used by the host and the
//! tests.

/// The panel the launcher is designed for.
pub const SCREEN: u32 = 480;

/// The app grid: two columns of two rows, so a page holds four apps, one to a quadrant.
pub const COLUMNS: usize = 2;
pub const ROWS: usize = 2;

/// The apps one page of the grid holds.
pub const PER_PAGE: usize = COLUMNS * ROWS;

/// The gap between the cells, and around the page.
pub const GUTTER: f32 = 20.0;
/// The padding of a tile's *box*: what the pressed wash covers around the icon and its label.
///
/// The box is the app's own size and not the cell's — the cell is the whole quadrant, and a wash the
/// size of a quadrant would be a white square rather than an app under a finger.
pub const TILE_PADDING: f32 = 18.0;
pub const TILE_RADIUS: f32 = 18.0;
pub const ICON: f32 = 84.0;
pub const ICON_RADIUS: f32 = 24.0;
pub const GLYPH: f32 = 38.0;
pub const LABEL: f32 = 15.0;
pub const GLYPH_GAP: f32 = 12.0;

/// The pager.
///
/// How far a finger moves before a press becomes a drag: a tap that twitched by a pixel must still
/// open the app it is on, and the platform's own movement threshold is a pixel — this is the rest of
/// the noise a finger makes while it is deciding. It decides *whether* this is a drag; whether the
/// drag turns the page is [`SWIPE_COMMIT`].
pub const SLOP: f32 = 8.0;

/// How far a finger has to travel to turn the page: about an eighth of the panel.
///
/// A fixed distance, and not a share of the screen — which is what it used to be (`width / 2`, once
/// the slop was taken off: 248 px on this panel and 520 in a desktop window). A finger travels the
/// same distance whatever the screen is, and a page that has to be dragged halfway across it is a
/// page nobody drags. The turn still happens when the finger leaves — see `Pager` — so this is the
/// whole threshold: reach it and the release turns the page, fall short of it and the release does
/// nothing.
pub const SWIPE_COMMIT: f32 = 64.0;

/// The page dots: one per page, the page that is up lit.
///
/// Exported, like [`SCREEN`], because the panel tests find what is on screen by its colour.
pub const DOT: f32 = 8.0;
pub const DOT_GAP: f32 = 12.0;
pub const DOT_UP: (u8, u8, u8) = (233, 236, 244);
pub const DOT_REST: (u8, u8, u8) = (62, 68, 92);

/// The status bar. Taller than a bar of text: it is the top of a round display, and the row sits
/// inside it rather than against its edge.
pub const STATUS_HEIGHT: f32 = 56.0;

/// The bar's own background: pure black.
///
/// Not the page's colour — the bar is the top of the display and reads as its own band — and black
/// rather than a near-black because these are AMOLED pixels: an unlit one costs nothing.
///
/// Exported, like [`DOT_UP`], because the panel tests find what is on screen by its colour.
pub const STATUS_BG: (u8, u8, u8) = (0, 0, 0);

pub const STATUS_FONT: f32 = 15.0;

/// The status bar's icons -- the signal and the battery -- in the icon font.
///
/// Larger than the bar's text because a glyph's ink is smaller than its em: Material Symbols fills
/// about three quarters of the square it is given, so an icon at [`STATUS_FONT`] would be an 11 px
/// picture beside 15 px digits.
pub const STATUS_ICON: f32 = 20.0;

pub const STATUS_GAP: f32 = 14.0;

/// How far the status bar's contents stay away from the screen's left and right edges.
///
/// The panel is a square with rounded corners and the bar is the band that meets them. The cut is
/// only a few pixels deep over the rows the text occupies, but a clock wedged into the corner of a
/// round display reads as a mistake whether or not it is *inside* the cut -- and these are the
/// panel's outermost pixels, the first the bezel takes.
///
/// It is [`GUTTER`], the page's own margin, so the clock sits over the left column of the grid
/// rather than at the edge of the glass: the bar is the only row that spans the full width, and one
/// number is enough to say where its contents start.
pub const STATUS_INSET: f32 = GUTTER;

/// The signal scale, in bars.
///
/// Four, because that is the scale the HAL answers on (`ApInfo::signal_bars()`) -- and it is what
/// the launcher accepts, not what it can draw: the icon font has three bars and a crossed-out one,
/// so the fourth bar shows the third's picture. See `status::wifi_icon`.
pub const WIFI_BARS: u8 = 4;
