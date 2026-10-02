//! The Hello signature and greeting application for Pomelo OS running on WebAssembly.

extern crate alloc;

use alloc::format;
use alloc::string::ToString;
use pomelo_sdk::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Message {
    Wave,
    NextColor,
    LaunchCalc,
    LaunchTerm,
    Back,
}

pub struct HelloApp {
    wave_count: u32,
    color_index: usize,
}

impl HelloApp {
    pub fn new() -> Self {
        Self {
            wave_count: 0,
            color_index: 0,
        }
    }

    fn current_accent(&self) -> Color {
        let colors = [
            Color::from_rgb8(236, 72, 153), // Pink
            Color::from_rgb8(168, 85, 247), // Purple
            Color::from_rgb8(59, 130, 246), // Blue
            Color::from_rgb8(16, 185, 129), // Emerald
            Color::from_rgb8(245, 158, 11), // Amber
        ];
        colors[self.color_index % colors.len()]
    }
}

impl Default for HelloApp {
    fn default() -> Self {
        Self::new()
    }
}

impl Application for HelloApp {
    type Message = Message;

    fn update(&mut self, message: Self::Message) {
        match message {
            Message::Wave => {
                self.wave_count += 1;
                self.color_index = (self.color_index + 1) % 5;
                log_info(format!("Hello app waved! Count is now {}", self.wave_count));
            }
            Message::NextColor => {
                self.color_index = (self.color_index + 1) % 5;
            }
            Message::LaunchCalc => {
                pomelo_sdk::services::launch_app("calculator");
            }
            Message::LaunchTerm => {
                pomelo_sdk::services::launch_app("terminal");
            }
            Message::Back => {
                pomelo_sdk::services::back();
            }
        }
    }

    fn view(&self) -> Element<Self::Message> {
        let accent = self.current_accent();

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

        let top_bar = row!()
            .width(Length::Fill)
            .height(Length::Fixed(48.0))
            .padding([0.0, 16.0])
            .align_y(Alignment::Center)
            .push(back_btn)
            .push(space().width(Length::Fill));

        // 2. Hero Section
        let hello_title = text("hello.")
            .size(72.0)
            .color(accent)
            .align_x(Alignment::Center);

        let subtitle = text("欢迎来到 Pomelo OS")
            .size(20.0)
            .color(Color::WHITE)
            .align_x(Alignment::Center);

        let desc = text("下一代轻量级 WebAssembly 嵌入式微系统")
            .size(14.0)
            .color(Color::from_rgb8(150, 155, 175))
            .align_x(Alignment::Center);

        let waves_txt = if self.wave_count == 0 {
            "点击下方按钮打个招呼吧 👋".to_string()
        } else {
            format!("已打招呼 {} 次！✨", self.wave_count)
        };

        let wave_badge = container(
            text(waves_txt)
                .size(14.0)
                .color(Color::from_rgb8(220, 225, 245))
                .align_x(Alignment::Center)
                .align_y(Alignment::Center),
        )
        .padding([8.0, 16.0])
        .background(Color::from_rgba8(255, 255, 255, 0.06))
        .border_radius(16.0);

        let hero = col![
            hello_title,
            subtitle,
            desc,
            space().height(Length::Fixed(12.0)),
            wave_badge,
        ]
        .spacing(8.0)
        .align_x(Alignment::Center);

        // 3. Action Buttons
        let wave_btn = button(
            container(
                text("👋 打招呼 (Wave)")
                    .size(16.0)
                    .color(Color::WHITE)
                    .align_x(Alignment::Center)
                    .align_y(Alignment::Center),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center),
        )
        .width(Length::Fixed(220.0))
        .height(Length::Fixed(52.0))
        .padding(0.0)
        .background(accent)
        .pressed_background(Color::from_rgba8(0, 0, 0, 0.2))
        .border_radius(16.0)
        .on_press(Message::Wave);

        let launch_calc_btn = button(
            container(
                text("打开计算器")
                    .size(14.0)
                    .color(Color::WHITE)
                    .align_x(Alignment::Center)
                    .align_y(Alignment::Center),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center),
        )
        .width(Length::Fixed(120.0))
        .height(Length::Fixed(44.0))
        .padding(0.0)
        .background(Color::from_rgb8(45, 52, 65))
        .border_radius(12.0)
        .on_press(Message::LaunchCalc);

        let launch_term_btn = button(
            container(
                text("打开终端")
                    .size(14.0)
                    .color(Color::WHITE)
                    .align_x(Alignment::Center)
                    .align_y(Alignment::Center),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center),
        )
        .width(Length::Fixed(120.0))
        .height(Length::Fixed(44.0))
        .padding(0.0)
        .background(Color::from_rgb8(45, 52, 65))
        .border_radius(12.0)
        .on_press(Message::LaunchTerm);

        let sub_actions = row!()
            .spacing(12.0)
            .align_y(Alignment::Center)
            .push(launch_calc_btn)
            .push(launch_term_btn);

        let actions = col![
            wave_btn,
            sub_actions,
        ]
        .spacing(14.0)
        .align_x(Alignment::Center);

        let page = col![
            top_bar,
            space().height(Length::Fill),
            hero,
            space().height(Length::Fill),
            actions,
            space().height(Length::Fixed(24.0)),
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center);

        container(page)
            .width(Length::Fill)
            .height(Length::Fill)
            .background(Color::from_rgb8(15, 17, 24))
            .into()
    }
}

export_app!(HelloApp);
