//! The music player application for Pomelo OS running on WebAssembly.

extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use pomelo_sdk::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Message {
    PlayPause,
    Next,
    Previous,
    VolumeUp,
    VolumeDown,
    Back,
}

pub struct Track {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_secs: u32,
}

pub struct MusicPlayerApp {
    tracks: Vec<Track>,
    current_index: usize,
    is_playing: bool,
    elapsed_secs: u32,
    volume: u32,
}

impl MusicPlayerApp {
    pub fn new() -> Self {
        let tracks = vec![
            Track {
                title: "Pomelo Sunset".to_string(),
                artist: "Lofi Dreamer".to_string(),
                album: "Chilled Citrus Vol. 1".to_string(),
                duration_secs: 184,
            },
            Track {
                title: "Cyberpunk Pulse".to_string(),
                artist: "Synthwave Unit".to_string(),
                album: "Neon Highways".to_string(),
                duration_secs: 215,
            },
            Track {
                title: "Midnight Drive".to_string(),
                artist: "Retro Waves".to_string(),
                album: "Outrun 1986".to_string(),
                duration_secs: 160,
            },
        ];

        Self {
            tracks,
            current_index: 0,
            is_playing: false,
            elapsed_secs: 42,
            volume: 80,
        }
    }

    fn current_track(&self) -> &Track {
        &self.tracks[self.current_index % self.tracks.len()]
    }

    fn format_time(secs: u32) -> String {
        let m = secs / 60;
        let s = secs % 60;
        format!("{:02}:{:02}", m, s)
    }
}

impl Default for MusicPlayerApp {
    fn default() -> Self {
        Self::new()
    }
}

impl Application for MusicPlayerApp {
    type Message = Message;

    fn update(&mut self, message: Self::Message) {
        match message {
            Message::PlayPause => {
                self.is_playing = !self.is_playing;
                log_info(format!("Music playback changed: playing = {}", self.is_playing));
            }
            Message::Next => {
                self.current_index = (self.current_index + 1) % self.tracks.len();
                self.elapsed_secs = 0;
            }
            Message::Previous => {
                if self.current_index > 0 {
                    self.current_index -= 1;
                } else {
                    self.current_index = self.tracks.len() - 1;
                }
                self.elapsed_secs = 0;
            }
            Message::VolumeUp => {
                if self.volume < 100 {
                    self.volume += 10;
                }
            }
            Message::VolumeDown => {
                if self.volume >= 10 {
                    self.volume -= 10;
                }
            }
            Message::Back => {
                pomelo_sdk::services::back();
            }
        }
    }

    fn view(&self) -> Element<Self::Message> {
        let track = self.current_track();

        // 1. Top Bar
        let back_btn = button(
            container(
                text("< 返回")
                    .size(16.0)
                    .color(Color::from_rgb8(100, 160, 255))
                    .align_y(Alignment::Center),
            )
            .height(Length::Fill)
            .align_y(Alignment::Center),
        )
        .width(Length::Fixed(80.0))
        .height(Length::Fixed(44.0))
        .padding(0.0)
        .background(Color::TRANSPARENT)
        .pressed_background(Color::from_rgba8(255, 255, 255, 0.05))
        .border_radius(8.0)
        .on_press(Message::Back);

        let vol_txt = text(format!("Vol: {}%", self.volume))
            .size(14.0)
            .color(Color::from_rgb8(180, 185, 200))
            .align_y(Alignment::Center);

        let top_bar = row!()
            .width(Length::Fill)
            .height(Length::Fixed(48.0))
            .padding([0.0, 16.0])
            .align_y(Alignment::Center)
            .push(back_btn)
            .push(space().width(Length::Fill))
            .push(vol_txt);

        // 2. Vinyl Record Disk Artwork
        let center_badge = container(
            text("♫")
                .size(36.0)
                .color(Color::WHITE)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center),
        )
        .width(Length::Fixed(64.0))
        .height(Length::Fixed(64.0))
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .background(Color::from_rgb8(220, 38, 38))
        .border_radius(32.0);

        let vinyl_disk = container(center_badge)
            .width(Length::Fixed(180.0))
            .height(Length::Fixed(180.0))
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .background(Color::from_rgb8(24, 24, 30))
            .border_radius(90.0)
            .border_width(8.0)
            .border_color(Color::from_rgb8(40, 40, 50));

        let disk_container = container(vinyl_disk)
            .width(Length::Fill)
            .align_x(Alignment::Center)
            .padding(10.0);

        // 3. Track Titles
        let title_txt = text(&track.title)
            .size(24.0)
            .color(Color::WHITE)
            .align_x(Alignment::Center);

        let artist_txt = text(format!("{} • {}", track.artist, track.album))
            .size(14.0)
            .color(Color::from_rgb8(160, 165, 185))
            .align_x(Alignment::Center);

        let titles_col = col![title_txt, artist_txt]
            .spacing(6.0)
            .align_x(Alignment::Center);

        // 4. Progress bar
        let elapsed_str = Self::format_time(self.elapsed_secs);
        let total_str = Self::format_time(track.duration_secs);

        let elapsed_label = text(elapsed_str)
            .size(12.0)
            .color(Color::from_rgb8(140, 145, 160));

        let total_label = text(total_str)
            .size(12.0)
            .color(Color::from_rgb8(140, 145, 160));

        let pct = (self.elapsed_secs as f32 / track.duration_secs.max(1) as f32).clamp(0.0, 1.0);
        let filled_portion = (pct * 100.0) as u16;
        let empty_portion = 100u16.saturating_sub(filled_portion);

        let bar_filled = container(space())
            .width(Length::FillPortion(filled_portion.max(1)))
            .height(Length::Fixed(6.0))
            .background(Color::from_rgb8(59, 130, 246))
            .border_radius(3.0);

        let bar_empty = container(space())
            .width(Length::FillPortion(empty_portion.max(1)))
            .height(Length::Fixed(6.0))
            .background(Color::from_rgb8(45, 50, 65))
            .border_radius(3.0);

        let track_bar = row!()
            .width(Length::Fill)
            .height(Length::Fixed(6.0))
            .push(bar_filled)
            .push(bar_empty);

        let progress_row = row!()
            .width(Length::Fill)
            .align_y(Alignment::Center)
            .push(elapsed_label)
            .push(space().width(Length::Fill))
            .push(total_label);

        let progress_section = col![track_bar, progress_row]
            .spacing(6.0)
            .padding([0.0, 24.0]);

        // 5. Controls
        let prev_btn = button(
            container(text("◀◀").size(18.0).color(Color::WHITE).align_x(Alignment::Center).align_y(Alignment::Center))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center),
        )
        .width(Length::Fixed(60.0))
        .height(Length::Fixed(60.0))
        .padding(0.0)
        .background(Color::from_rgb8(35, 40, 52))
        .pressed_background(Color::from_rgb8(55, 62, 80))
        .border_radius(30.0)
        .on_press(Message::Previous);

        let play_label = if self.is_playing { "❚❚" } else { "▶" };
        let play_btn = button(
            container(text(play_label).size(24.0).color(Color::WHITE).align_x(Alignment::Center).align_y(Alignment::Center))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center),
        )
        .width(Length::Fixed(72.0))
        .height(Length::Fixed(72.0))
        .padding(0.0)
        .background(Color::from_rgb8(37, 99, 235))
        .pressed_background(Color::from_rgb8(29, 78, 216))
        .border_radius(36.0)
        .on_press(Message::PlayPause);

        let next_btn = button(
            container(text("▶▶").size(18.0).color(Color::WHITE).align_x(Alignment::Center).align_y(Alignment::Center))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center),
        )
        .width(Length::Fixed(60.0))
        .height(Length::Fixed(60.0))
        .padding(0.0)
        .background(Color::from_rgb8(35, 40, 52))
        .pressed_background(Color::from_rgb8(55, 62, 80))
        .border_radius(30.0)
        .on_press(Message::Next);

        let vol_down_btn = button(
            container(text("－").size(16.0).color(Color::WHITE).align_x(Alignment::Center).align_y(Alignment::Center))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center),
        )
        .width(Length::Fixed(44.0))
        .height(Length::Fixed(44.0))
        .padding(0.0)
        .background(Color::from_rgb8(30, 34, 45))
        .border_radius(22.0)
        .on_press(Message::VolumeDown);

        let vol_up_btn = button(
            container(text("＋").size(16.0).color(Color::WHITE).align_x(Alignment::Center).align_y(Alignment::Center))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center),
        )
        .width(Length::Fixed(44.0))
        .height(Length::Fixed(44.0))
        .padding(0.0)
        .background(Color::from_rgb8(30, 34, 45))
        .border_radius(22.0)
        .on_press(Message::VolumeUp);

        let controls_row = row!()
            .spacing(16.0)
            .align_y(Alignment::Center)
            .push(vol_down_btn)
            .push(prev_btn)
            .push(play_btn)
            .push(next_btn)
            .push(vol_up_btn);

        let controls_container = container(controls_row)
            .width(Length::Fill)
            .align_x(Alignment::Center)
            .padding(10.0);

        let page = col![
            top_bar,
            disk_container,
            titles_col,
            space().height(Length::Fixed(12.0)),
            progress_section,
            space().height(Length::Fixed(8.0)),
            controls_container,
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center);

        container(page)
            .width(Length::Fill)
            .height(Length::Fill)
            .background(Color::from_rgb8(15, 16, 22))
            .into()
    }
}

export_app!(MusicPlayerApp);
