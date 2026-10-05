//! The catalogue the grid shows.
//!
//! The launcher decides the order and list of apps displayed on the home screen.
//! Each sub-app defines its own identity (bilingual names, icon glyph, and accent color)
//! via [`pomelo_widgets::AppMeta`].

pub use pomelo_widgets::{AppIcon, AppMeta as Entry};

include!(concat!(env!("OUT_DIR"), "/baked_icons.rs"));

/// Six apps, in the order the grid shows them.
pub const CATALOGUE: &[Entry] = &[
    terminal::META,
    calculator::META,
    demo_counter::META,
    Entry {
        icon: AppIcon::Bitmap(baked::HELLO_ICON),
        ..hello::META
    },
    settings::META,
    music_player::META,
];
