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
pub const ICON: f32 = 118.0;
pub const ICON_RADIUS: f32 = 33.0;
pub const GLYPH: f32 = 54.0;
pub const LABEL: f32 = 15.0;
pub const GLYPH_GAP: f32 = 10.0;

/// The pager.
///
/// How far a finger moves before a press becomes a drag: Flutter's standard `kTouchSlop` is 18.0 px.
/// This acts as a dead zone so that slight finger jitter or roll during a tap does not mistakenly
/// cancel the press or start dragging.
pub const SLOP: f32 = 18.0;

/// How far a finger has to travel without flicking to turn the page: about half the screen (45%).
///
/// Following Flutter's `PageScrollPhysics`, slow dragging requires moving past roughly half the
/// screen (216 px on this 480 px panel), while a quick flick (fling velocity > 450 px/s) turns
/// the page with only a brief swipe.
pub const SWIPE_COMMIT: f32 = 216.0;

/// The page dots: one per page, the page that is up lit.
///
/// Exported, like [`SCREEN`], because the panel tests find what is on screen by its colour.
pub const DOT: f32 = 8.0;
pub const DOT_GAP: f32 = 12.0;
pub const DOT_UP: (u8, u8, u8) = (233, 236, 244);
pub const DOT_REST: (u8, u8, u8) = (62, 68, 92);

/// The status bar. Taller than a bar of text: it is the top of a round display, and the row sits
/// inside it rather than against its edge.
pub const STATUS_HEIGHT: f32 = 48.0;

/// The bar's own background: pure white in light mode, pure black in dark mode.
///
/// Exported, like [`DOT_UP`], because the panel tests find what is on screen by its colour.
pub const STATUS_BG: (u8, u8, u8) = (255, 255, 255);
pub const STATUS_BG_DARK: (u8, u8, u8) = (0, 0, 0);
pub const STATUS_BG_LIGHT: (u8, u8, u8) = (255, 255, 255);

pub const STATUS_FONT: f32 = 20.0;

/// The status bar's icons -- the signal and the battery -- in the icon font.
///
/// Larger than the bar's text because a glyph's ink is smaller than its em: Material Symbols fills
/// about three quarters of the square it is given, so an icon at [`STATUS_FONT`] would be an 11 px
/// picture beside 15 px digits.
pub const STATUS_ICON: f32 = 24.0;

/// The battery icon's size, scaled up for visibility.
pub const STATUS_BATTERY_ICON: f32 = 36.0;

/// The battery percentage text size.
pub const STATUS_PERCENT_FONT: f32 = 18.0;

/// Gap between the percentage text and the battery icon.
pub const STATUS_BATTERY_GAP: f32 = 6.0;

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
pub const STATUS_INSET: f32 = GUTTER + 7.0;

/// The signal scale, in bars.
///
/// Four, because that is the scale the HAL answers on (`ApInfo::signal_bars()`) -- and it is what
/// the launcher accepts, not what it can draw: the icon font has three bars and a crossed-out one,
/// so the fourth bar shows the third's picture. See `status::wifi_icon`.
pub const WIFI_BARS: u8 = 4;
