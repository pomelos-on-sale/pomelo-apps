//! The settings application for Pomelo OS running on WebAssembly.

extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};
use pomelo_sdk::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Section {
    Main,
    Wifi,
    Theme,
    Language,
    Battery,
    Storage,
    About,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Message {
    OpenSection(Section),
    Back,
    ToggleTheme,
    ToggleLanguage,
}

pub struct SettingsApp {
    section: Section,
    is_dark: bool,
    is_chinese: bool,
    battery: u32,
    wifi_connected: bool,
    wifi_ssid: String,
}

impl SettingsApp {
    pub fn new() -> Self {
        let mut battery = 100;
        let mut is_dark = true;
        let mut is_chinese = true;

        if let Some(sys) = pomelo_sdk::services::get_system_info() {
            battery = sys.battery;
            if sys.theme == "light" {
                is_dark = false;
            }
            if sys.lang == "en" {
                is_chinese = false;
            }
        }

        Self {
            section: Section::Main,
            is_dark,
            is_chinese,
            battery,
            wifi_connected: true,
            wifi_ssid: "Pomelo-WiFi".to_string(),
        }
    }

    fn render_header(&self, title: &str, _is_subpage: bool) -> Element<Message> {
        let back_btn = button(
            container(
                text(if self.is_chinese { "< 返回" } else { "< Back" })
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

        let title_txt = text(title)
            .size(18.0)
            .color(Color::WHITE)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center);

        let row_header = row!()
            .width(Length::Fill)
            .height(Length::Fixed(52.0))
            .padding([0.0, 16.0])
            .align_y(Alignment::Center)
            .push(back_btn)
            .push(container(title_txt).width(Length::Fill).align_x(Alignment::Center))
            .push(space().width(Length::Fixed(80.0)));

        container(row_header)
            .width(Length::Fill)
            .height(Length::Fixed(52.0))
            .background(Color::from_rgba8(255, 255, 255, 0.03))
            .into()
    }

    fn render_item(&self, icon_str: &str, title: &str, value: &str, target: Option<Section>) -> Element<Message> {
        let left_part = row!()
            .spacing(12.0)
            .align_y(Alignment::Center)
            .push(
                container(
                    text(icon_str)
                        .size(18.0)
                        .color(Color::WHITE)
                        .align_x(Alignment::Center)
                        .align_y(Alignment::Center),
                )
                .width(Length::Fixed(32.0))
                .height(Length::Fixed(32.0))
                .background(Color::from_rgb8(45, 52, 65))
                .border_radius(8.0)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center),
            )
            .push(text(title).size(16.0).color(Color::WHITE).align_y(Alignment::Center));

        let right_part = row!()
            .spacing(8.0)
            .align_y(Alignment::Center)
            .push(text(value).size(14.0).color(Color::from_rgb8(150, 155, 170)).align_y(Alignment::Center))
            .push(text(">").size(16.0).color(Color::from_rgb8(100, 105, 120)).align_y(Alignment::Center));

        let content = row!()
            .width(Length::Fill)
            .height(Length::Fixed(54.0))
            .padding([0.0, 16.0])
            .align_y(Alignment::Center)
            .push(left_part)
            .push(space().width(Length::Fill))
            .push(right_part);

        if let Some(sec) = target {
            button(content)
                .width(Length::Fill)
                .height(Length::Fixed(54.0))
                .padding(0.0)
                .background(Color::from_rgba8(255, 255, 255, 0.04))
                .pressed_background(Color::from_rgba8(255, 255, 255, 0.08))
                .border_radius(12.0)
                .on_press(Message::OpenSection(sec))
                .into()
        } else {
            container(content)
                .width(Length::Fill)
                .height(Length::Fixed(54.0))
                .background(Color::from_rgba8(255, 255, 255, 0.04))
                .border_radius(12.0)
                .into()
        }
    }

    fn render_main(&self) -> Element<Message> {
        let (wifi_t, theme_t, lang_t, batt_t, store_t, about_t) = if self.is_chinese {
            ("无线网络", "外观主题", "界面语言", "电池电量", "存储空间", "关于本机")
        } else {
            ("Wi-Fi", "Theme", "Language", "Battery", "Storage", "About")
        };

        let theme_val = if self.is_dark {
            if self.is_chinese { "深色" } else { "Dark" }
        } else {
            if self.is_chinese { "浅色" } else { "Light" }
        };

        let lang_val = if self.is_chinese { "简体中文" } else { "English" };
        let wifi_val = if self.wifi_connected { &self.wifi_ssid } else { "未连接" };
        let batt_val = format!("{}%", self.battery);

        let list = col![
            self.render_item("W", wifi_t, wifi_val, Some(Section::Wifi)),
            self.render_item("T", theme_t, theme_val, Some(Section::Theme)),
            self.render_item("L", lang_t, lang_val, Some(Section::Language)),
            self.render_item("B", batt_t, &batt_val, Some(Section::Battery)),
            self.render_item("S", store_t, "16 MB", Some(Section::Storage)),
            self.render_item("i", about_t, "Pomelo OS", Some(Section::About)),
        ]
        .spacing(10.0)
        .padding(16.0);

        col![
            self.render_header(if self.is_chinese { "设置" } else { "Settings" }, false),
            container(list).width(Length::Fill).height(Length::Fill),
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn render_subpage(&self) -> Element<Message> {
        match self.section {
            Section::Wifi => {
                let title = if self.is_chinese { "无线网络" } else { "Wi-Fi" };
                let body = col![
                    self.render_item("W", "SSID", &self.wifi_ssid, None),
                    self.render_item("I", "IP", "192.168.1.108", None),
                    self.render_item("S", "Signal", "-58 dBm", None),
                ]
                .spacing(10.0)
                .padding(16.0);

                col![self.render_header(title, true), body].into()
            }
            Section::Theme => {
                let title = if self.is_chinese { "外观主题" } else { "Theme" };
                let toggle_txt = if self.is_dark { "切换为浅色主题" } else { "切换为深色主题" };
                let btn = button(
                    container(
                        text(toggle_txt)
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
                .width(Length::Fill)
                .height(Length::Fixed(48.0))
                .background(Color::from_rgb8(59, 130, 246))
                .border_radius(12.0)
                .on_press(Message::ToggleTheme);

                let body = col![
                    self.render_item("T", "Current", if self.is_dark { "Dark Mode" } else { "Light Mode" }, None),
                    space().height(Length::Fixed(16.0)),
                    btn,
                ]
                .spacing(10.0)
                .padding(16.0);

                col![self.render_header(title, true), body].into()
            }
            Section::Language => {
                let title = if self.is_chinese { "界面语言" } else { "Language" };
                let toggle_txt = if self.is_chinese { "Switch to English" } else { "切换为中文" };
                let btn = button(
                    container(
                        text(toggle_txt)
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
                .width(Length::Fill)
                .height(Length::Fixed(48.0))
                .background(Color::from_rgb8(16, 185, 129))
                .border_radius(12.0)
                .on_press(Message::ToggleLanguage);

                let body = col![
                    self.render_item("L", "Current", if self.is_chinese { "简体中文" } else { "English" }, None),
                    space().height(Length::Fixed(16.0)),
                    btn,
                ]
                .spacing(10.0)
                .padding(16.0);

                col![self.render_header(title, true), body].into()
            }
            Section::Battery => {
                let title = if self.is_chinese { "电池电量" } else { "Battery" };
                let body = col![
                    self.render_item("B", "Level", &format!("{}%", self.battery), None),
                    self.render_item("H", "Health", "Good (100%)", None),
                    self.render_item("V", "Voltage", "4.12 V", None),
                ]
                .spacing(10.0)
                .padding(16.0);

                col![self.render_header(title, true), body].into()
            }
            Section::Storage => {
                let title = if self.is_chinese { "存储空间" } else { "Storage" };
                let body = col![
                    self.render_item("F", "Flash Total", "16 MB", None),
                    self.render_item("U", "Apps & Data", "3.2 MB Used", None),
                    self.render_item("A", "Available", "12.8 MB Free", None),
                ]
                .spacing(10.0)
                .padding(16.0);

                col![self.render_header(title, true), body].into()
            }
            Section::About => {
                let title = if self.is_chinese { "关于本机" } else { "About" };
                let body = col![
                    self.render_item("P", "Device", "Pomelo S3", None),
                    self.render_item("O", "OS", "Pomelo OS (Wasm AOT)", None),
                    self.render_item("V", "Version", "v0.2.0", None),
                    self.render_item("C", "Chip", "ESP32-S3 (Xtensa Dual)", None),
                    self.render_item("A", "Author", "Pomelo Team", None),
                ]
                .spacing(10.0)
                .padding(16.0);

                col![self.render_header(title, true), body].into()
            }
            Section::Main => self.render_main(),
        }
    }
}

impl Default for SettingsApp {
    fn default() -> Self {
        Self::new()
    }
}

impl Application for SettingsApp {
    type Message = Message;

    fn update(&mut self, message: Self::Message) {
        match message {
            Message::OpenSection(s) => {
                self.section = s;
            }
            Message::Back => {
                if self.section == Section::Main {
                    pomelo_sdk::services::back();
                } else {
                    self.section = Section::Main;
                }
            }
            Message::ToggleTheme => {
                self.is_dark = !self.is_dark;
                let val: &[u8] = if self.is_dark { b"dark" } else { b"light" };
                pomelo_sdk::services::kv_set("theme", val);
            }
            Message::ToggleLanguage => {
                self.is_chinese = !self.is_chinese;
                let val: &[u8] = if self.is_chinese { b"zh" } else { b"en" };
                pomelo_sdk::services::kv_set("lang", val);
            }
        }
    }

    fn view(&self) -> Element<Self::Message> {
        let content = if self.section == Section::Main {
            self.render_main()
        } else {
            self.render_subpage()
        };

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .background(Color::from_rgb8(15, 18, 25))
            .into()
    }
}

export_app!(SettingsApp);
