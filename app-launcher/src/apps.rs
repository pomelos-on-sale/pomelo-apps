//! The catalogue the grid shows.
//!
//! Names, accents and an icon. The icon is a Material Symbols glyph rather than a baked bitmap:
//! bitmaps need iced's `image` feature, which pulls the `image` crate into the firmware, and our
//! icons are RGB565 blits `pomelo-gfx` would have to be handed through a door iced does not open
//! yet. A font is the road that is already built — see `pomelo_material_symbols`.

use iced::Color;

use pomelo_material_symbols::Icon;

/// One app in the grid.
pub struct Entry {
    /// Shown under the tile.
    pub name: &'static str,
    /// Drawn inside the tile.
    pub icon: Icon,
    /// The tile's accent, as bytes so this table stays a plain `const`.
    pub accent: (u8, u8, u8),
}

impl Entry {
    /// The accent, as a color.
    pub fn color(&self) -> Color {
        let (r, g, b) = self.accent;

        Color::from_rgb8(r, g, b)
    }
}

/// Six apps, in the order the grid shows them.
pub const CATALOGUE: &[Entry] = &[
    Entry {
        name: "Terminal",
        icon: Icon::TERMINAL,
        accent: (38, 44, 62),
    },
    Entry {
        name: "Calculator",
        icon: Icon::CALCULATE,
        accent: (46, 62, 46),
    },
    Entry {
        name: "Counter",
        icon: Icon::COUNTER_0,
        accent: (62, 48, 36),
    },
    Entry {
        name: "Hello",
        icon: Icon::WAVING_HAND,
        accent: (58, 38, 58),
    },
    Entry {
        name: "Settings",
        icon: Icon::SETTINGS,
        accent: (40, 52, 60),
    },
    Entry {
        name: "Music",
        icon: Icon::MUSIC_NOTE,
        accent: (60, 40, 44),
    },
];
