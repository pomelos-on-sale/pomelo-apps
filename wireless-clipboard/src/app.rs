use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

use iced::border::Radius;
use iced::widget::{column, container, qr_code, rich_text, row, scrollable, span, text};
use iced::{never, Alignment, Border, Color, Element, Fill, Length, Subscription, Task, Theme};
use pomelo_hal::types::WifiState;
use pomelo_hal::Board;
use pomelo_material_symbols::Icon;
use pomelo_widgets::preferences::{FontSizeTier, Language, SystemPreferences, ThemeMode};

/// The font size tiers obtained from SystemPreferences: [18.0, 20.0, 24.0, 30.0].
pub const FONT_EXTRA_SMALL: f32 = FontSizeTier::ExtraSmall.base_size(); // 18.0 px (Compact tier)
pub const FONT_SMALL: f32 = FontSizeTier::Small.base_size();            // 20.0 px (Small tier)
pub const FONT_STANDARD: f32 = FontSizeTier::Standard.base_size();      // 24.0 px (Standard tier)
pub const FONT_LARGE: f32 = FontSizeTier::Large.base_size();            // 30.0 px (Large tier)

use crate::server::HttpServer;

pub struct WirelessClipboard {
    board: Arc<Board>,
    preferences: SystemPreferences,
    #[allow(dead_code)]
    server: Option<HttpServer>,
    notify_rx: Receiver<String>,
    #[allow(dead_code)]
    notify_tx: Sender<String>,
    port: u16,
    server_url: String,
    last_ip: String,
    qr_data: Option<qr_code::Data>,
    clipboard_text: String,
    last_synced_at: Option<Instant>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    Tick,
    Clear,
    ClipboardPolled(Option<String>),
}

fn tick_stream() -> impl iced::futures::Stream<Item = Message> {
    let (mut tx, rx) = iced::futures::channel::mpsc::channel(4);
    std::thread::Builder::new()
        .name("clipboard-timer".into())
        .spawn(move || {
            loop {
                std::thread::sleep(Duration::from_millis(500));
                if tx.try_send(Message::Tick).is_err() {
                    break;
                }
            }
        })
        .ok();
    rx
}

impl WirelessClipboard {
    pub fn new(board: Arc<Board>) -> Self {
        let (tx, rx) = channel();
        let (server, port) = match HttpServer::start(8080, tx.clone()) {
            Ok((s, p)) => (Some(s), p),
            Err(_) => (None, 8080),
        };

        let initial_text = server.as_ref().map(|s| s.get_text()).unwrap_or_default();

        let mut app = Self {
            board,
            preferences: SystemPreferences::default(),
            server,
            notify_rx: rx,
            notify_tx: tx,
            port,
            server_url: String::new(),
            last_ip: String::new(),
            qr_data: None,
            clipboard_text: initial_text,
            last_synced_at: None,
        };
        app.update_network_status();
        app
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn clipboard_text(&self) -> &str {
        &self.clipboard_text
    }

    pub fn boot(board: Arc<Board>) -> (Self, Task<Message>) {
        (Self::new(board), iced::clipboard::read().map(Message::ClipboardPolled))
    }

    pub fn preferences(&self) -> SystemPreferences {
        self.preferences
    }

    pub fn set_preferences(&mut self, preferences: SystemPreferences) {
        self.preferences = preferences;
    }

    pub fn theme_mode(&self) -> ThemeMode {
        self.preferences.theme
    }

    pub fn set_theme_mode(&mut self, theme: ThemeMode) {
        self.preferences.theme = theme;
    }

    pub fn language(&self) -> Language {
        self.preferences.language
    }

    pub fn set_language(&mut self, language: Language) {
        self.preferences.language = language;
    }

    pub fn theme(&self) -> Theme {
        if self.preferences.theme.is_light() {
            Theme::custom(
                "PomeloLight",
                iced::theme::Palette {
                    background: Color::from_rgb8(242, 242, 247),
                    ..iced::theme::Palette::LIGHT
                },
            )
        } else {
            Theme::custom(
                "PomeloDark",
                iced::theme::Palette {
                    background: Color::BLACK,
                    ..iced::theme::Palette::DARK
                },
            )
        }
    }

    fn update_network_status(&mut self) {
        let wifi = self.board.wifi();
        let status = wifi.status();

        if status.state == WifiState::Connected && !status.ip.is_empty() {
            if self.last_ip != status.ip {
                self.last_ip = status.ip.clone();
                let url = format!("http://{}:{}", status.ip, self.port);
                self.server_url = url.clone();
                self.qr_data = qr_code::Data::new(&url).ok();
            }
        } else {
            self.last_ip.clear();
            self.server_url.clear();
            self.qr_data = None;
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick => {
                self.update_network_status();

                let mut latest = None;
                while let Ok(content) = self.notify_rx.try_recv() {
                    latest = Some(content);
                }

                if let Some(content) = latest {
                    self.clipboard_text = content.clone();
                    if let Some(srv) = &self.server {
                        srv.set_text(&content);
                    }
                    self.last_synced_at = Some(Instant::now());
                    iced::clipboard::write(content)
                } else {
                    iced::clipboard::read().map(Message::ClipboardPolled)
                }
            }
            Message::ClipboardPolled(content) => {
                let current = content.unwrap_or_default();
                if self.clipboard_text != current {
                    self.clipboard_text = current.clone();
                    if let Some(srv) = &self.server {
                        srv.set_text(&current);
                    }
                }
                Task::none()
            }
            Message::Clear => {
                self.clipboard_text.clear();
                if let Some(srv) = &self.server {
                    srv.set_text("");
                }
                self.last_synced_at = None;
                iced::clipboard::write(String::new())
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let is_light = self.theme_mode().is_light();
        let is_en = self.language() == Language::English;

        let bg_color = if is_light {
            Color::from_rgb8(242, 242, 247)
        } else {
            Color::BLACK
        };
        let text_color = if is_light {
            Color::from_rgb8(20, 22, 38)
        } else {
            Color::WHITE
        };
        let dim_text = if is_light {
            Color::from_rgb8(120, 122, 136)
        } else {
            Color::from_rgb8(150, 155, 175)
        };
        let border_color = if is_light {
            Color::from_rgba8(0, 0, 0, 0.15)
        } else {
            Color::from_rgba8(255, 255, 255, 0.18)
        };
        let accent = Color::from_rgb8(59, 130, 246);

        // Header: Centered Title (no symbol, no wifi/ip)
        let title_text = if is_en {
            "Wireless Clipboard"
        } else {
            "无线剪贴板"
        };

        let header = container(
            text(title_text)
                .size(FONT_LARGE)
                .style(move |_| iced::widget::text::Style {
                    color: Some(text_color),
                }),
        )
        .width(Fill)
        .center_x(Fill);

        // Left Column: QR Code + Tutorial (no container background)
        let left_content: Element<'_, Message> = if let Some(ref qr_data) = self.qr_data {
            let qr_widget = qr_code(qr_data)
                .cell_size(5.0)
                .style(|_| qr_code::Style {
                    cell: Color::BLACK,
                    background: Color::WHITE,
                });
            let qr_box = container(qr_widget)
                .padding(2)
                .style(|_| container::Style {
                    background: Some(Color::WHITE.into()),
                    border: Border {
                        radius: Radius::from(6.0),
                        ..Default::default()
                    },
                    ..Default::default()
                });

            let url_text = text(&self.server_url)
                .size(FONT_SMALL)
                .width(Fill)
                .wrapping(iced::widget::text::Wrapping::WordOrGlyph)
                .style(move |_| iced::widget::text::Style { color: Some(accent) });

            let tutorial_title = if is_en { "Usage Steps:" } else { "使用步骤：" };
            let step1 = if is_en { "1. Same Wi-Fi" } else { "1. 连同一 Wi-Fi" };
            let step2 = if is_en { "2. Scan QR code" } else { "2. 相机扫码打开" };
            let step3 = if is_en { "3. Send to sync" } else { "3. 发送实时同步" };
            let step4_prefix = if is_en { "4. Tap " } else { "4. 键盘点击 " };
            let step4_suffix = if is_en { " to paste" } else { " 粘贴" };

            let step4 = rich_text![
                span(step4_prefix),
                span(Icon::CONTENT_PASTE.glyph()).font(pomelo_material_symbols::font()),
                span(step4_suffix),
            ]
            .size(FONT_SMALL)
            .color(dim_text)
            .width(Fill)
            .wrapping(iced::widget::text::Wrapping::WordOrGlyph)
            .on_link_click(never);

            let tutorial_box = column![
                text(tutorial_title)
                    .size(FONT_SMALL)
                    .width(Fill)
                    .wrapping(iced::widget::text::Wrapping::WordOrGlyph)
                    .style(move |_| iced::widget::text::Style { color: Some(dim_text) }),
                text(step1)
                    .size(FONT_SMALL)
                    .width(Fill)
                    .wrapping(iced::widget::text::Wrapping::WordOrGlyph)
                    .style(move |_| iced::widget::text::Style { color: Some(dim_text) }),
                text(step2)
                    .size(FONT_SMALL)
                    .width(Fill)
                    .wrapping(iced::widget::text::Wrapping::WordOrGlyph)
                    .style(move |_| iced::widget::text::Style { color: Some(dim_text) }),
                text(step3)
                    .size(FONT_SMALL)
                    .width(Fill)
                    .wrapping(iced::widget::text::Wrapping::WordOrGlyph)
                    .style(move |_| iced::widget::text::Style { color: Some(dim_text) }),
                step4,
            ]
            .spacing(3)
            .width(Fill);

            let text_section = column![
                url_text,
                tutorial_box,
            ]
            .spacing(4)
            .width(Fill)
            .align_x(Alignment::Start);

            column![
                container(qr_box).width(Fill).center_x(Fill),
                text_section,
            ]
            .spacing(4)
            .width(Fill)
            .into()
        } else {
            let no_wifi_title = if is_en { "Wi-Fi Off" } else { "未连接 Wi-Fi" };

            container(
                column![
                    pomelo_widgets::icon(Icon::WIFI_OFF)
                        .size(44)
                        .style(move |_| iced::widget::text::Style { color: Some(dim_text) }),
                    text(no_wifi_title)
                        .size(FONT_STANDARD)
                        .style(move |_| iced::widget::text::Style { color: Some(dim_text) }),
                ]
                .spacing(8)
                .align_x(Alignment::Center),
            )
            .width(Fill)
            .height(Fill)
            .center_x(Fill)
            .center_y(Fill)
            .into()
        };

        let left_column = container(left_content)
            .width(Length::FillPortion(1))
            .height(Fill);

        // Right Column: Clipboard Content (border-only, always fills max area when showing content) / Empty state
        let right_content: Element<'_, Message> = if self.clipboard_text.is_empty() {
            let empty_hint = if is_en { "Empty" } else { "无内容" };

            container(
                column![
                    pomelo_widgets::icon(Icon::CONTENT_PASTE)
                        .size(44)
                        .style(move |_| iced::widget::text::Style { color: Some(dim_text) }),
                    text(empty_hint)
                        .size(FONT_STANDARD)
                        .style(move |_| iced::widget::text::Style { color: Some(dim_text) }),
                ]
                .spacing(8)
                .align_x(Alignment::Center),
            )
            .width(Fill)
            .height(Fill)
            .center_x(Fill)
            .center_y(Fill)
            .into()
        } else {
            let content_box = container(
                scrollable(
                    text(&self.clipboard_text)
                        .size(FONT_SMALL)
                        .width(Fill)
                        .wrapping(iced::widget::text::Wrapping::WordOrGlyph)
                        .style(move |_| iced::widget::text::Style { color: Some(text_color) }),
                )
                .width(Fill)
                .height(Fill),
            )
            .width(Fill)
            .height(Fill)
            .padding(10)
            .style(move |_| container::Style {
                border: Border {
                    color: border_color,
                    width: 1.0,
                    radius: Radius::from(10.0),
                },
                ..Default::default()
            });

            container(content_box)
                .width(Fill)
                .height(Fill)
                .padding([16, 0])
                .into()
        };

        let right_column = container(right_content)
            .width(Length::FillPortion(1))
            .height(Fill);

        // Combined Split View
        let content_row = row![left_column, right_column]
            .spacing(10)
            .width(Fill)
            .height(Fill);

        container(
            column![header, content_row]
                .spacing(8)
                .width(Fill)
                .height(Fill),
        )
        .padding([10, 14])
        .width(Fill)
        .height(Fill)
        .style(move |_| container::Style {
            background: Some(bg_color.into()),
            text_color: Some(text_color),
            ..Default::default()
        })
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::run(tick_stream)
    }
}
