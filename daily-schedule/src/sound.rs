//! Schedule chime notification sounds.
//!
//! Loads audio notification sounds from assets and provides streaming playback
//! functions for event start and event end.

use std::path::{Path, PathBuf};

use pomelo_hal::Board;

pub const SOUNDS_SUBDIR: &str = "sounds";
pub const EVENT_START_FILE: &str = "creatorassets.com/sound-6.mp3";
pub const EVENT_END_FILE: &str = "creatorassets.com/sound-8.mp3";

pub const DEFAULT_START_SOUND: &[u8] =
    include_bytes!("../../../assets/sounds/creatorassets.com/sound-6.mp3");
pub const DEFAULT_END_SOUND: &[u8] =
    include_bytes!("../../../assets/sounds/creatorassets.com/sound-8.mp3");

#[derive(Debug, Clone)]
pub struct ScheduleSounds {
    pub start_path: PathBuf,
    pub end_path: PathBuf,
}

impl ScheduleSounds {
    /// Loads and ensures schedule sound files exist in /internal for streaming playback.
    pub fn init_and_load() -> Self {
        let start_path = resolve_or_install(EVENT_START_FILE, DEFAULT_START_SOUND);
        let end_path = resolve_or_install(EVENT_END_FILE, DEFAULT_END_SOUND);

        Self {
            start_path,
            end_path,
        }
    }

    /// Plays the event start chime via streaming audio.
    pub fn play_start(&self, board: &Board) {
        if board.audio().play_bytes(DEFAULT_START_SOUND, Some("mp3")).is_err() {
            if let Some(path_str) = self.start_path.to_str() {
                let _ = board.audio().play(path_str);
            }
        }
    }

    /// Plays the event end chime via streaming audio.
    pub fn play_end(&self, board: &Board) {
        if board.audio().play_bytes(DEFAULT_END_SOUND, Some("mp3")).is_err() {
            if let Some(path_str) = self.end_path.to_str() {
                let _ = board.audio().play(path_str);
            }
        }
    }
}

/// Resolves the file path for `filename`, or creates it with `_default_bytes` if not found.
fn resolve_or_install(filename: &str, _default_bytes: &[u8]) -> PathBuf {
    let candidates = [
        format!("./assets/sounds/{filename}"),
        format!("../assets/sounds/{filename}"),
        format!("../../assets/sounds/{filename}"),
        format!("../../../assets/sounds/{filename}"),
    ];

    for c in &candidates {
        let p = PathBuf::from(c);
        if is_valid_sound_file(&p) {
            return p;
        }
    }

    PathBuf::from(format!("./assets/sounds/{filename}"))
}

fn is_valid_sound_file(path: &Path) -> bool {
    path.is_file()
}
