//! Alarm ringtone audio playback and caching.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use pomelo_hal::{probe_audio_meta, AudioDecoder, AudioMeta, Board};

pub const SOUNDS_SUBDIR: &str = "sounds";
pub const SOUND_0_FILE: &str = "creatorassets.com/sound-6.mp3";
pub const SOUND_1_FILE: &str = "creatorassets.com/sound-8.mp3";
pub const SOUND_2_FILE: &str = "creatorassets.com/sound-1.mp3";

pub const DEFAULT_SOUND_0: &[u8] =
    include_bytes!("../../../assets/internal/sounds/creatorassets.com/sound-6.mp3");
pub const DEFAULT_SOUND_1: &[u8] =
    include_bytes!("../../../assets/internal/sounds/creatorassets.com/sound-8.mp3");
pub const DEFAULT_SOUND_2: &[u8] =
    include_bytes!("../../../assets/internal/sounds/creatorassets.com/sound-1.mp3");

#[derive(Debug, Clone)]
pub struct SoundPcm {
    pub pcm: Arc<Vec<u8>>,
    pub meta: AudioMeta,
}

#[derive(Debug, Clone)]
pub struct AlarmSounds {
    pub paths: [PathBuf; 3],
    pub pcm_cache: [Option<SoundPcm>; 3],
}

impl AlarmSounds {
    /// Initializes and pre-decodes the 3 ringtone sounds for instant, zero-delay playback.
    pub fn init_and_load() -> Self {
        let p0 = resolve_or_install(SOUND_0_FILE, DEFAULT_SOUND_0);
        let p1 = resolve_or_install(SOUND_1_FILE, DEFAULT_SOUND_1);
        let p2 = resolve_or_install(SOUND_2_FILE, DEFAULT_SOUND_2);

        let pcm0 = load_sound_pcm(&p0, DEFAULT_SOUND_0);
        let pcm1 = load_sound_pcm(&p1, DEFAULT_SOUND_1);
        let pcm2 = load_sound_pcm(&p2, DEFAULT_SOUND_2);

        Self {
            paths: [p0, p1, p2],
            pcm_cache: [pcm0, pcm1, pcm2],
        }
    }

    /// Plays the requested alarm melody.
    pub fn play(&self, board: &Board, sound_id: u8) {
        let idx = (sound_id as usize) % 3;
        if let Some(ref sound) = self.pcm_cache[idx] {
            let res = board.audio().play_pcm(
                Arc::clone(&sound.pcm),
                sound.meta.sample_rate,
                sound.meta.channels,
                sound.meta.bits_per_sample,
            );
            if res.is_ok() {
                return;
            }
        }

        if let Some(path_str) = self.paths[idx].to_str() {
            let _ = board.audio().play(path_str);
        }
    }

    /// Stops any active audio playback.
    pub fn stop(&self, board: &Board) {
        board.audio().stop();
    }
}

fn load_sound_pcm(path: &Path, default_bytes: &[u8]) -> Option<SoundPcm> {
    if let Some(path_str) = path.to_str() {
        if let Ok(decoder) = AudioDecoder::open(path_str) {
            if let Ok((meta, pcm)) = decoder.decode_all() {
                return Some(SoundPcm {
                    pcm: Arc::new(pcm),
                    meta,
                });
            }
        }
    }

    if let Ok(decoder) = AudioDecoder::from_bytes(default_bytes.to_vec(), Some("mp3")) {
        if let Ok((meta, pcm)) = decoder.decode_all() {
            return Some(SoundPcm {
                pcm: Arc::new(pcm),
                meta,
            });
        }
    }

    None
}

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
            return p;
        }
    }

    let app_data_sounds = pomelo_hal::storage::resolve_app_data_root()
        .join(SOUNDS_SUBDIR)
        .join(filename);
    if is_valid_sound_file(&app_data_sounds) {
        return app_data_sounds;
    }

    let install_targets = [
        PathBuf::from("/internal/sounds").join(filename),
        PathBuf::from("./internal/sounds").join(filename),
        app_data_sounds.clone(),
    ];

    for target in &install_targets {
        if let Some(parent) = target.parent() {
            if fs::create_dir_all(parent).is_ok() {
                if fs::write(target, default_bytes).is_ok() {
                    return target.clone();
                }
            }
        }
    }

    app_data_sounds
}

fn is_valid_sound_file(path: &Path) -> bool {
    if let Some(path_str) = path.to_str() {
        probe_audio_meta(path_str).is_ok()
    } else {
        false
    }
}
