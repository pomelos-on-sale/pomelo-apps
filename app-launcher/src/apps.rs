//! The catalogue the grid shows.
//!
//! Names, accents and an icon. The icon is a Material Symbols glyph rather than a baked bitmap:
//! bitmaps need iced's `image` feature, which pulls the `image` crate into the firmware, and our
//! icons are RGB565 blits `pomelo-gfx` would have to be handed through a door iced does not open
//! yet. A font is the road that is already built — see `pomelo_material_symbols`.

use iced::Color;

use pomelo_material_symbols::Icon;
use pomelo_widgets::Language;

/// One app in the grid.
pub struct Entry {
    /// Shown under the tile (canonical English name).
    pub name: &'static str,
    /// Simplified Chinese name.
    pub name_zh: &'static str,
    /// Drawn inside the tile.
    pub icon: Icon,
    /// The tile's accent, as bytes so this table stays a plain `const`.
    pub accent: (u8, u8, u8),
}

impl Entry {
    /// The localized name in `language`.
    pub fn localized_name(&self, language: Language) -> &'static str {
        match language {
            Language::Chinese => self.name_zh,
            Language::English => self.name,
        }
    }

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
        name_zh: "终端",
        icon: Icon::TERMINAL,
        accent: (38, 44, 62),
    },
    Entry {
        name: "Calculator",
        name_zh: "计算器",
        icon: Icon::CALCULATE,
        accent: (46, 62, 46),
    },
    Entry {
        name: "Counter",
        name_zh: "计数器",
        icon: Icon::COUNTER_0,
        accent: (62, 48, 36),
    },
    Entry {
        name: "Hello",
        name_zh: "你好",
        icon: Icon::WAVING_HAND,
        accent: (58, 38, 58),
    },
    Entry {
        name: "Settings",
        name_zh: "设置",
        icon: Icon::SETTINGS,
        accent: (40, 52, 60),
    },
    Entry {
        name: "Music",
        name_zh: "音乐",
        icon: Icon::MUSIC_NOTE,
        accent: (60, 40, 44),
    },
];
