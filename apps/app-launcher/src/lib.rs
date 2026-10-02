//! The application launcher for Pomelo OS running on WebAssembly.

extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use pomelo_sdk::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Message {
    Launch(String),
    NextPage,
    PrevPage,
    SetPage(usize),
}

#[derive(Clone)]
pub struct AppEntry {
    pub id: String,
    pub name: String,
    pub name_zh: String,
    pub icon: String,
    pub accent: Color,
}

pub struct LauncherApp {
    apps: Vec<AppEntry>,
    current_page: usize,
    clock: String,
    battery: u32,
    wifi: u32,
}

impl LauncherApp {
    pub fn new() -> Self {
        let apps = Self::load_apps_from_system();
        Self {
            apps,
            current_page: 0,
            clock: "12:00".to_string(),
            battery: 100,
            wifi: 4,
        }
    }

    fn load_apps_from_system() -> Vec<AppEntry> {
        let mut apps = Vec::new();
        if let Some(sys) = pomelo_sdk::services::get_system_info() {
            for app in sys.apps {
                if app.id == "app-launcher" || app.id == "launcher" || app.id.ends_with(".launcher") {
                    continue;
                }
                let accent = match app.id.as_str() {
                    "counter" => Color::from_rgb8(62, 48, 36),
                    "calculator" => Color::from_rgb8(46, 62, 46),
                    "terminal" => Color::from_rgb8(38, 44, 62),
                    "hello" => Color::from_rgb8(58, 38, 58),
                    "settings" => Color::from_rgb8(40, 52, 60),
                    "music-player" => Color::from_rgb8(60, 40, 44),
                    _ => Color::from_rgb8(45, 52, 65),
                };
                apps.push(AppEntry {
                    id: app.id,
                    name: app.name,
                    name_zh: app.name_zh,
                    icon: app.icon,
                    accent,
                });
            }
        }
        apps
    }

    fn render_status_bar(&self) -> Element<Message> {
        let (clock, battery, wifi) = if let Some(sys) = pomelo_sdk::services::get_system_info() {
            (sys.clock, sys.battery, sys.wifi)
        } else {
            (self.clock.clone(), self.battery, self.wifi)
        };

        let clock_txt = text(&clock)
            .size(16.0)
            .color(Color::WHITE)
            .align_y(Alignment::Center);

        let wifi_glyph = match wifi {
            0 => "wifi_off",
            1 => "wifi_1",
            2 => "wifi_2",
            _ => "wifi",
        };
        let wifi_widget = icon(wifi_glyph)
            .size(18.0)
            .color(Color::from_rgb8(180, 185, 200))
            .align_y(Alignment::Center);

        let batt_glyph = match battery {
            0..=15 => "battery_0",
            16..=30 => "battery_1",
            31..=45 => "battery_2",
            46..=60 => "battery_3",
            61..=75 => "battery_4",
            76..=90 => "battery_5",
            91..=95 => "battery_6",
            _ => "battery_full",
        };
        let batt_widget = icon(batt_glyph)
            .size(18.0)
            .color(Color::from_rgb8(180, 185, 200))
            .align_y(Alignment::Center);

        let batt_txt = text(format!("{battery}%"))
            .size(14.0)
            .color(Color::from_rgb8(180, 185, 200))
            .align_y(Alignment::Center);

        let row_bar = row!()
            .width(Length::Fill)
            .height(Length::Fixed(44.0))
            .padding([0.0, 20.0])
            .align_y(Alignment::Center)
            .push(clock_txt)
            .push(space().width(Length::Fill))
            .push(wifi_widget)
            .push(space().width(Length::Fixed(12.0)))
            .push(batt_widget)
            .push(space().width(Length::Fixed(4.0)))
            .push(batt_txt);

        container(row_bar)
            .width(Length::Fill)
            .height(Length::Fixed(44.0))
            .into()
    }

    fn render_app_tile(&self, app: &AppEntry) -> Element<Message> {
        let icon_box = container(
            icon(&app.icon)
                .size(36.0)
                .color(Color::WHITE)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center),
        )
        .width(Length::Fixed(96.0))
        .height(Length::Fixed(96.0))
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .background(app.accent)
        .border_radius(26.0);

        let label = text(&app.name)
            .size(15.0)
            .color(Color::WHITE)
            .align_x(Alignment::Center);

        let tile_content = col![icon_box, label]
            .spacing(10.0)
            .align_x(Alignment::Center);

        let tile_btn = button(
            container(tile_content)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(6.0)
        .background(Color::TRANSPARENT)
        .pressed_background(Color::from_rgba8(255, 255, 255, 0.1))
        .border_radius(18.0)
        .on_press(Message::Launch(app.id.clone()));

        container(tile_btn)
            .width(Length::FillPortion(1))
            .height(Length::FillPortion(1))
            .padding(10.0)
            .into()
    }

    fn render_dots(&self, total_pages: usize) -> Element<Message> {
        let mut dots_row = row!().spacing(12.0).align_y(Alignment::Center);

        for p in 0..total_pages {
            let is_active = p == self.current_page;
            let dot_color = if is_active {
                Color::from_rgb8(233, 236, 244)
            } else {
                Color::from_rgb8(62, 68, 92)
            };

            let dot = button(
                container(space().width(Length::Fixed(8.0)).height(Length::Fixed(8.0)))
                    .width(Length::Fixed(8.0))
                    .height(Length::Fixed(8.0))
                    .background(dot_color)
                    .border_radius(4.0),
            )
            .width(Length::Fixed(24.0))
            .height(Length::Fixed(24.0))
            .padding(0.0)
            .background(Color::TRANSPARENT)
            .on_press(Message::SetPage(p));

            dots_row = dots_row.push(dot);
        }

        container(dots_row)
            .width(Length::Fill)
            .height(Length::Fixed(32.0))
            .align_x(Alignment::Center)
            .into()
    }
}

impl Default for LauncherApp {
    fn default() -> Self {
        Self::new()
    }
}

impl Application for LauncherApp {
    type Message = Message;

    fn update(&mut self, message: Self::Message) {
        match message {
            Message::Launch(app_id) => {
                log_info(format!("Launching application: {app_id}"));
                launch_app(&app_id);
            }
            Message::NextPage => {
                let dynamic_apps = Self::load_apps_from_system();
                let count = if !dynamic_apps.is_empty() { dynamic_apps.len() } else { self.apps.len() };
                let total_pages = (count + 3) / 4;
                if self.current_page + 1 < total_pages {
                    self.current_page += 1;
                }
            }
            Message::PrevPage => {
                if self.current_page > 0 {
                    self.current_page -= 1;
                }
            }
            Message::SetPage(p) => {
                self.current_page = p;
            }
        }
    }

    fn view(&self) -> Element<Self::Message> {
        let dynamic_apps = Self::load_apps_from_system();
        let apps = if !dynamic_apps.is_empty() {
            &dynamic_apps
        } else {
            &self.apps
        };

        let per_page = 4;
        let total_pages = if apps.is_empty() { 1 } else { (apps.len() + per_page - 1) / per_page };
        let page_start = self.current_page * per_page;
        let page_apps = &apps[page_start.min(apps.len())..(page_start + per_page).min(apps.len())];

        // 2x2 grid rows
        let mut row1 = row!().width(Length::Fill).height(Length::FillPortion(1));
        let mut row2 = row!().width(Length::Fill).height(Length::FillPortion(1));

        for (i, app) in page_apps.iter().enumerate() {
            let tile = self.render_app_tile(app);
            if i < 2 {
                row1 = row1.push(tile);
            } else {
                row2 = row2.push(tile);
            }
        }

        // Fill empty quadrants if less than 4 apps on page
        if page_apps.len() == 1 {
            row1 = row1.push(space().width(Length::FillPortion(1)));
        }
        if page_apps.len() <= 2 {
            row2 = row2.push(space().width(Length::FillPortion(1))).push(space().width(Length::FillPortion(1)));
        } else if page_apps.len() == 3 {
            row2 = row2.push(space().width(Length::FillPortion(1)));
        }

        let grid = col![row1, row2]
            .width(Length::Fill)
            .height(Length::Fill)
            .padding([0.0, 16.0]);

        let page = col![
            self.render_status_bar(),
            grid,
            self.render_dots(total_pages),
        ]
        .width(Length::Fill)
        .height(Length::Fill);

        container(page)
            .width(Length::Fill)
            .height(Length::Fill)
            .background(Color::from_rgb8(15, 18, 25))
            .into()
    }
}

export_app!(LauncherApp);
