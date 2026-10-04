//! The catalogue the grid shows.
//!
//! The launcher decides the order and list of apps displayed on the home screen.
//! Each sub-app defines its own identity (bilingual names, icon glyph, and accent color)
//! via [`pomelo_widgets::AppMeta`].

pub use pomelo_widgets::AppMeta as Entry;

/// Six apps, in the order the grid shows them.
pub const CATALOGUE: &[Entry] = &[
    terminal::META,
    calculator::META,
    demo_counter::META,
    hello::META,
    settings::META,
    music_player::META,
];
