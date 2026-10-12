use std::time::{Duration, Instant};

use chrono::{DateTime, Local, Timelike};
use iced::widget::{container, row};
use iced::{Color, Element, Fill, Subscription, Task, Theme};
use pomelo_widgets::preferences::{Language, SystemPreferences, ThemeMode};

use crate::time_display;
use crate::timeline::Timeline;

pub struct DailySchedule {
    now: DateTime<Local>,
    /// 用户最后一次手动滚动的时刻
    user_scrolled_at: Option<Instant>,
    /// 记录上一帧的分钟数，用于判断是否跨分钟
    last_minute: u32,
    /// 是否已执行初次对焦
    initial_focused: bool,
    preferences: SystemPreferences,
}

pub type App = DailySchedule;

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    Tick(DateTime<Local>),
    TimelineScrolled,
}

fn tick_stream() -> impl iced::futures::Stream<Item = Message> {
    let (mut tx, rx) = iced::futures::channel::mpsc::channel(4);
    std::thread::Builder::new()
        .name("daily-schedule-timer".into())
        .spawn(move || {
            loop {
                std::thread::sleep(Duration::from_millis(1000));
                if tx.try_send(Message::Tick(Local::now())).is_err() {
                    break;
                }
            }
        })
        .ok();
    rx
}

impl DailySchedule {
    pub fn new() -> Self {
        let now = Local::now();
        Self {
            now,
            user_scrolled_at: None,
            last_minute: now.minute(),
            initial_focused: false,
            preferences: SystemPreferences::default(),
        }
    }

    pub fn boot() -> (Self, Task<Message>) {
        let app = Self::new();
        let task = Timeline::focus(app.now);
        (app, task)
    }

    pub fn focus(&mut self) -> Task<Message> {
        self.now = Local::now();
        Timeline::focus(self.now)
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
            crate::theme::pure_black()
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::TimelineScrolled => {
                // 用户手动滚动了：记录操作时刻，暂停自动跟随
                self.user_scrolled_at = Some(Instant::now());
                Task::none()
            }
            Message::Tick(now) => {
                let minute_changed = now.minute() != self.last_minute;
                self.last_minute = now.minute();
                self.now = now;

                // 1. 初次启动第一帧对焦
                if !self.initial_focused {
                    self.initial_focused = true;
                    return Timeline::focus(now);
                }

                // 2. 状态判断：用户是否手动干预了滑动
                if let Some(scrolled_at) = self.user_scrolled_at {
                    if scrolled_at.elapsed() >= Duration::from_secs(5) {
                        // 已经满 5 秒无动作：解除用户锁定，自动 focus 回当前时间点
                        self.user_scrolled_at = None;
                        Timeline::focus(now)
                    } else {
                        // 还在 5 秒等待期内：保持用户自由浏览位置，不强行复位
                        Task::none()
                    }
                } else if minute_changed {
                    // 3. 无用户干预，且分钟增加：微调跟随滚动，保持当前时间居中
                    Timeline::focus(now)
                } else {
                    Task::none()
                }
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let left_panel = time_display::view(self.now, self.language());
        let right_timeline = Timeline::new(self.now).view();

        let is_light = self.theme_mode().is_light();
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

        container(
            row![left_panel, right_timeline]
                .width(Fill)
                .height(Fill),
        )
        .width(Fill)
        .height(Fill)
        .style(move |_theme| container::Style {
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

impl Default for DailySchedule {
    fn default() -> Self {
        Self::new()
    }
}
