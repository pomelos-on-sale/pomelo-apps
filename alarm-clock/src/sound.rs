//! Alarm ringtone audio playback and management.

use std::path::{Path, PathBuf};

use pomelo_hal::Board;

pub const SOUNDS_SUBDIR: &str = "sounds";
pub const SOUND_0_FILE: &str = "creatorassets.com/sound-6.mp3";
pub const SOUND_1_FILE: &str = "creatorassets.com/sound-8.mp3";
pub const SOUND_2_FILE: &str = "creatorassets.com/sound-1.mp3";

pub const DEFAULT_SOUND_0: &[u8] =
    include_bytes!("../../../assets/sounds/creatorassets.com/sound-6.mp3");
pub const DEFAULT_SOUND_1: &[u8] =
    include_bytes!("../../../assets/sounds/creatorassets.com/sound-8.mp3");
pub const DEFAULT_SOUND_2: &[u8] =
    include_bytes!("../../../assets/sounds/creatorassets.com/sound-1.mp3");

#[derive(Debug, Clone)]
pub struct AlarmSounds {
    pub paths: [PathBuf; 3],
}

impl AlarmSounds {
    /// Resolves and ensures the 3 ringtone sound files exist for streaming playback.
    pub fn init_and_load() -> Self {
        let p0 = resolve_or_install(SOUND_0_FILE, DEFAULT_SOUND_0);
        let p1 = resolve_or_install(SOUND_1_FILE, DEFAULT_SOUND_1);
        let p2 = resolve_or_install(SOUND_2_FILE, DEFAULT_SOUND_2);

        Self {
            paths: [p0, p1, p2],
        }
    }

    /// Plays the requested alarm melody via streaming audio once (for preview).
    pub fn play(&self, board: &Board, sound_id: u8) {
        let bytes = match sound_id % 3 {
            0 => DEFAULT_SOUND_0,
            1 => DEFAULT_SOUND_1,
            _ => DEFAULT_SOUND_2,
        };
        if board.audio().play_bytes(bytes, Some("mp3")).is_err() {
            let idx = (sound_id as usize) % 3;
            if let Some(path_str) = self.paths[idx].to_str() {
                let _ = board.audio().play(path_str);
            }
        }
    }

    /// Plays the requested alarm melody in a continuous loop until stopped.
    pub fn play_loop(&self, board: &Board, sound_id: u8) {
        let bytes = match sound_id % 3 {
            0 => DEFAULT_SOUND_0,
            1 => DEFAULT_SOUND_1,
            _ => DEFAULT_SOUND_2,
        };
        if board.audio().play_bytes_loop(bytes, Some("mp3")).is_err() {
            let idx = (sound_id as usize) % 3;
            if let Some(path_str) = self.paths[idx].to_str() {
                let _ = board.audio().play_loop(path_str);
            }
        }
    }

    /// Stops any active audio playback.
    pub fn stop(&self, board: &Board) {
        board.audio().stop();
    }
}

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
