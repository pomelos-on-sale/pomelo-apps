//! Schedule chime notification sounds.
//!
//! Loads audio notification sounds (e.g. `/internal/sounds/` or local assets),
//! and provides playback functions for event start and event end.

use std::fs;
use std::path::{Path, PathBuf};

use pomelo_hal::probe_audio_meta;
use pomelo_hal::Board;

pub const SOUNDS_SUBDIR: &str = "sounds";
pub const EVENT_START_FILE: &str = "creatorassets.com/sound-6.mp3";
pub const EVENT_END_FILE: &str = "creatorassets.com/sound-8.mp3";

pub const DEFAULT_START_SOUND: &[u8] =
    include_bytes!("../../../assets/internal/sounds/creatorassets.com/sound-6.mp3");
pub const DEFAULT_END_SOUND: &[u8] =
    include_bytes!("../../../assets/internal/sounds/creatorassets.com/sound-8.mp3");

#[derive(Debug, Clone)]
pub struct ScheduleSounds {
    pub start_path: PathBuf,
    pub end_path: PathBuf,
}

impl ScheduleSounds {
    /// Loads and ensures schedule sound files exist in /internal or equivalent storage.
    pub fn init_and_load() -> Self {
        let start_path = resolve_or_install(EVENT_START_FILE, DEFAULT_START_SOUND);
        let end_path = resolve_or_install(EVENT_END_FILE, DEFAULT_END_SOUND);
        println!(
            "[ScheduleSounds] init_and_load: start_path={:?} (exists={}), end_path={:?} (exists={})",
            start_path,
            start_path.exists(),
            end_path,
            end_path.exists()
        );
        Self {
            start_path,
            end_path,
        }
    }

    /// Plays the event start chime.
    pub fn play_start(&self, board: &Board) {
        if let Some(path_str) = self.start_path.to_str() {
            println!("[ScheduleSounds] play_start: requesting audio play for '{}'", path_str);
            match board.audio().play(path_str) {
                Ok(meta) => {
                    println!(
                        "[ScheduleSounds] play_start: chime playback started successfully (duration={:.2}s)",
                        meta.duration_secs
                    );
                }
                Err(e) => {
                    eprintln!("[ScheduleSounds] play_start: failed to play '{}': {}", path_str, e);
                }
            }
        } else {
            eprintln!("[ScheduleSounds] play_start: invalid non-UTF8 path {:?}", self.start_path);
        }
    }

    /// Plays the event end chime.
    pub fn play_end(&self, board: &Board) {
        if let Some(path_str) = self.end_path.to_str() {
            println!("[ScheduleSounds] play_end: requesting audio play for '{}'", path_str);
            match board.audio().play(path_str) {
                Ok(meta) => {
                    println!(
                        "[ScheduleSounds] play_end: chime playback started successfully (duration={:.2}s)",
                        meta.duration_secs
                    );
                }
                Err(e) => {
                    eprintln!("[ScheduleSounds] play_end: failed to play '{}': {}", path_str, e);
                }
            }
        } else {
            eprintln!("[ScheduleSounds] play_end: invalid non-UTF8 path {:?}", self.end_path);
        }
    }
}

/// Resolves the file path for `filename`, or creates it with `default_bytes` if not found.
fn resolve_or_install(filename: &str, default_bytes: &[u8]) -> PathBuf {
    let candidates = [
        format!("/internal/sounds/{filename}"),
        format!("./internal/sounds/{filename}"),
        format!("./assets/internal/sounds/{filename}"),
        format!("../assets/internal/sounds/{filename}"),
        format!("../../assets/internal/sounds/{filename}"),
        format!("../../../assets/internal/sounds/{filename}"),
    ];

    for c in &candidates {
        let p = PathBuf::from(c);
        if is_valid_sound_file(&p) {
            println!("[ScheduleSounds] found valid sound file at candidate path: {:?}", p);
            return p;
        }
    }

    // Try app data storage directory
    let app_data_sounds = pomelo_hal::storage::resolve_app_data_root()
        .join(SOUNDS_SUBDIR)
        .join(filename);
    if is_valid_sound_file(&app_data_sounds) {
        println!("[ScheduleSounds] found valid sound file in app_data: {:?}", app_data_sounds);
        return app_data_sounds;
    }

    // Attempt to install to /internal/sounds, ./internal/sounds, or app_data
    let install_targets = [
        PathBuf::from("/internal/sounds").join(filename),
        PathBuf::from("./internal/sounds").join(filename),
        app_data_sounds.clone(),
    ];

    for target in &install_targets {
        if let Some(parent) = target.parent() {
            if fs::create_dir_all(parent).is_ok() {
                if fs::write(target, default_bytes).is_ok() {
                    println!("[ScheduleSounds] installed default sound file to: {:?}", target);
                    return target.clone();
                }
            }
        }
    }

    eprintln!(
        "[ScheduleSounds] WARNING: failed to find or install sound file '{}', falling back to {:?}",
        filename, app_data_sounds
    );
    app_data_sounds
}

fn is_valid_sound_file(path: &Path) -> bool {
    if let Some(path_str) = path.to_str() {
        probe_audio_meta(path_str).is_ok()
    } else {
        false
    }
}
