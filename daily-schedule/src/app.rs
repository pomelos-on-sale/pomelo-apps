use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Local, NaiveDate, Timelike};
use iced::widget::{column, container, row, text};
use iced::{Color, Element, Fill, Subscription, Task, Theme};
use pomelo_hal::Board;
use pomelo_material_symbols::Icon;
use pomelo_widgets::preferences::{Language, SystemPreferences, ThemeMode};

use crate::ical;
use crate::model::{self, ScheduleConfig, ScheduleEvent};
use crate::time_display;
use crate::timeline::Timeline;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncStatus {
    Idle,
    Fetching,
    Error,
}

pub struct DailySchedule {
    now: DateTime<Local>,
    user_scrolled_at: Option<Instant>,
    last_minute: u32,
    initial_focused: bool,
    preferences: SystemPreferences,
    board: Arc<Board>,

    config: ScheduleConfig,
    events: Vec<ScheduleEvent>,
    sync_status: SyncStatus,
    current_date: NaiveDate,
    toast_message: Option<(String, Instant)>,
    is_programmatic_focus: bool,
}

pub type App = DailySchedule;

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    Tick(DateTime<Local>),
    TimelineScrolled,
    ImportFromClipboard,
    Refresh,
    ClearSubscription,
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

fn sanitize_calendar_url(raw: &str) -> Option<String> {
    let s = raw.trim();
    let s = s.trim_matches(|c| c == '"' || c == '\'' || c == '<' || c == '>');
    let s = s.trim();
    if s.starts_with("webcal://") {
        Some(format!("https://{}", &s[9..]))
    } else if s.starts_with("http://") || s.starts_with("https://") {
        Some(s.to_string())
    } else {
        None
    }
}

impl DailySchedule {
    #[cfg(not(target_os = "espidf"))]
    pub fn new() -> Self {
        Self::with_board(Arc::new(Board::simulated()))
    }

    pub fn with_board(board: Arc<Board>) -> Self {
        let now = Local::now();
        let today = now.date_naive();
        let config = model::load_config();

        // 检查是否有订阅链接
        let (events, sync_status) = if config.subscription_url.is_some() {
            // 链接存在，检查缓存是否存在且是当天的
            if let Some(cached_events) = model::load_today_cache(today) {
                // 缓存存在且是当天的，直接加载进入 UI 绘制
                (cached_events, SyncStatus::Idle)
            } else {
                // 缓存不存在或不是当天的，准备拉取
                (Vec::new(), SyncStatus::Idle)
            }
        } else {
            (Vec::new(), SyncStatus::Idle)
        };

        let mut app = Self {
            now,
            user_scrolled_at: None,
            last_minute: now.minute(),
            initial_focused: false,
            preferences: SystemPreferences::default(),
            board,
            config,
            events,
            sync_status,
            current_date: today,
            toast_message: None,
            is_programmatic_focus: false,
        };

        // 如果有链接但没有当天的缓存，启动时自动拉取
        if app.config.subscription_url.is_some() && model::load_today_cache(today).is_none() {
            app.trigger_fetch();
        }

        app
    }

    #[cfg(not(target_os = "espidf"))]
    pub fn boot() -> (Self, Task<Message>) {
        let app = Self::new();
        let task = Timeline::focus(app.now);
        (app, task)
    }

    pub fn focus(&mut self) -> Task<Message> {
        self.now = Local::now();
        self.is_programmatic_focus = true;
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

    pub fn subscription_url(&self) -> Option<&str> {
        self.config.subscription_url.as_deref()
    }

    pub fn events(&self) -> &[ScheduleEvent] {
        &self.events
    }

    pub fn sync_status(&self) -> SyncStatus {
        self.sync_status
    }

    /// 发起异步拉取日程数据
    pub fn trigger_fetch(&mut self) {
        if let Some(ref url) = self.config.subscription_url {
            let mut clean_url = url.trim().to_string();
            if clean_url.starts_with("webcal://") {
                clean_url = format!("https://{}", &clean_url[9..]);
            }
            if clean_url.starts_with("http://") || clean_url.starts_with("https://") {
                self.sync_status = SyncStatus::Fetching;
                if let Err(e) = self.board.http().start(&clean_url) {
                    self.sync_status = SyncStatus::Error;
                    self.toast_message = Some((
                        format!("无法发起同步: {}", e),
                        Instant::now() + Duration::from_secs(4),
                    ));
                }
            }
        }
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

    fn apply_subscription_link(&mut self, url: String) {
        self.config.subscription_url = Some(url);
        let _ = model::save_config(&self.config);

        let today = self.now.date_naive();
        if let Some(cached_events) = model::load_today_cache(today) {
            self.events = cached_events;
            self.sync_status = SyncStatus::Idle;
            self.toast_message = Some((
                if self.language() == Language::English {
                    "Link imported, loaded today's cache".to_string()
                } else {
                    "已导入链接，已载入今日缓存".to_string()
                },
                Instant::now() + Duration::from_secs(3),
            ));
        } else {
            self.toast_message = Some((
                if self.language() == Language::English {
                    "Link imported, fetching today's schedule...".to_string()
                } else {
                    "已导入链接，正在拉取今日日程...".to_string()
                },
                Instant::now() + Duration::from_secs(3),
            ));
            self.trigger_fetch();
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::TimelineScrolled => {
                if self.is_programmatic_focus {
                    self.is_programmatic_focus = false;
                    return Task::none();
                }
                // 用户手动滚动了：记录操作时刻，暂停自动跟随
                self.user_scrolled_at = Some(Instant::now());
                Task::none()
            }
            Message::ImportFromClipboard => {
                let clipboard_text = pomelo_widgets::clipboard::read();
                if let Some(url) = sanitize_calendar_url(&clipboard_text) {
                    self.apply_subscription_link(url);
                } else {
                    self.toast_message = Some((
                        if self.language() == Language::English {
                            "Clipboard has no valid calendar link".to_string()
                        } else {
                            "剪贴板中未发现有效日历订阅链接".to_string()
                        },
                        Instant::now() + Duration::from_secs(3),
                    ));
                }
                Task::none()
            }
            Message::Refresh => {
                if self.config.subscription_url.is_some() {
                    self.toast_message = Some((
                        if self.language() == Language::English {
                            "Refreshing today's schedule...".to_string()
                        } else {
                            "正在刷新今日日程...".to_string()
                        },
                        Instant::now() + Duration::from_secs(3),
                    ));
                    self.trigger_fetch();
                }
                Task::none()
            }
            Message::ClearSubscription => {
                self.config.subscription_url = None;
                let _ = model::save_config(&self.config);
                self.events.clear();
                self.sync_status = SyncStatus::Idle;
                self.toast_message = Some((
                    if self.language() == Language::English {
                        "Calendar subscription cleared".to_string()
                    } else {
                        "已清除日历订阅配置".to_string()
                    },
                    Instant::now() + Duration::from_secs(3),
                ));
                Task::none()
            }
            Message::Tick(now) => {
                let minute_changed = now.minute() != self.last_minute;
                self.last_minute = now.minute();
                self.now = now;

                // 检查跨日：若到第二天且有订阅，自动拉取新一天的日程
                let today = now.date_naive();
                if today != self.current_date {
                    self.current_date = today;
                    if self.config.subscription_url.is_some() {
                        if let Some(cached_events) = model::load_today_cache(today) {
                            self.events = cached_events;
                        } else {
                            self.events.clear();
                            self.trigger_fetch();
                        }
                    }
                }

                // 检查异步 HTTP 拉取进度
                if self.sync_status == SyncStatus::Fetching {
                    let http_state = self.board.http().state();
                    match http_state {
                        pomelo_hal::HttpState::Done => {
                            if let Ok(payload) = self.board.http().take_result() {
                                let ics_str = String::from_utf8_lossy(&payload);
                                let events = ical::parse_today_events(&ics_str, today);
                                let _ = model::save_today_cache(today, &events);
                                self.events = events;
                                self.sync_status = SyncStatus::Idle;
                                self.toast_message = Some((
                                    if self.language() == Language::English {
                                        format!("Synced {} event(s)", self.events.len())
                                    } else {
                                        format!("已同步 {} 条今日日程", self.events.len())
                                    },
                                    Instant::now() + Duration::from_secs(3),
                                ));
                                self.is_programmatic_focus = true;
                                return Timeline::focus(now);
                            } else {
                                self.sync_status = SyncStatus::Error;
                            }
                        }
                        pomelo_hal::HttpState::Error => {
                            self.sync_status = SyncStatus::Error;
                            self.toast_message = Some((
                                if self.language() == Language::English {
                                    "Failed to sync schedule, check network".to_string()
                                } else {
                                    "日程拉取失败，请检查网络".to_string()
                                },
                                Instant::now() + Duration::from_secs(4),
                            ));
                        }
                        _ => {}
                    }
                }

                // 清理过期的 toast
                if let Some((_, expiry)) = self.toast_message {
                    if Instant::now() >= expiry {
                        self.toast_message = None;
                    }
                }

                // 1. 初次启动第一帧对焦
                if !self.initial_focused {
                    self.initial_focused = true;
                    self.is_programmatic_focus = true;
                    return Timeline::focus(now);
                }

                // 2. 状态判断：用户是否手动干预了滑动
                if let Some(scrolled_at) = self.user_scrolled_at {
                    if scrolled_at.elapsed() >= Duration::from_secs(30) {
                        // 已经满 30 秒无动作：解除用户锁定，自动 focus 回当前时间点
                        self.user_scrolled_at = None;
                        self.is_programmatic_focus = true;
                        Timeline::focus(now)
                    } else {
                        // 还在 30 秒等待期内：保持用户自由浏览位置，不强行复位
                        Task::none()
                    }
                } else if minute_changed {
                    // 3. 无用户干预，且分钟增加：微调跟随滚动，保持当前时间居中
                    self.is_programmatic_focus = true;
                    Timeline::focus(now)
                } else {
                    Task::none()
                }
            }
        }
    }

    /// 右侧未配置订阅链接时的视图
    fn no_subscription_view(&self) -> Element<'_, Message> {
        let is_light = self.theme_mode().is_light();
        let dim_color = if is_light {
            Color::from_rgb8(120, 120, 130)
        } else {
            Color::from_rgb8(160, 160, 175)
        };
        let text_color = if is_light {
            Color::from_rgb8(20, 22, 38)
        } else {
            Color::WHITE
        };

        let icon = pomelo_widgets::icon(Icon::CALENDAR_MONTH)
            .size(44)
            .style(move |_| iced::widget::text::Style { color: Some(dim_color) });

        let title = text(if self.language() == Language::English {
            "No Calendar Link"
        } else {
            "未配置日历订阅"
        })
        .size(17)
        .style(move |_| iced::widget::text::Style { color: Some(text_color) });

        let import_btn = iced::widget::button(
            text(if self.language() == Language::English {
                "Import from Clipboard"
            } else {
                "从剪切板导入"
            })
            .size(14)
            .align_x(iced::alignment::Horizontal::Center),
        )
        .on_press(Message::ImportFromClipboard)
        .padding([9, 16])
        .style(move |theme: &Theme, status| {
            let palette = theme.extended_palette();
            let bg = match status {
                iced::widget::button::Status::Pressed => palette.primary.strong.color,
                iced::widget::button::Status::Hovered => palette.primary.base.color,
                _ => palette.primary.base.color,
            };
            iced::widget::button::Style {
                background: Some(bg.into()),
                text_color: Color::WHITE,
                border: iced::Border {
                    radius: iced::border::Radius::from(8.0),
                    ..Default::default()
                },
                ..Default::default()
            }
        });

        let tip = text(if self.language() == Language::English {
            "Configure in Settings > Apps > Schedule, or import from clipboard below"
        } else {
            "可在「设置 -> 应用 -> 日程」中配置管理，或直接从剪切板导入"
        })
        .size(11)
        .align_x(iced::alignment::Horizontal::Center)
        .style(move |_| iced::widget::text::Style { color: Some(dim_color) });

        let mut content_col = column![icon, title, tip, import_btn]
            .spacing(12)
            .align_x(iced::Alignment::Center);

        if let Some((msg, _)) = &self.toast_message {
            content_col = content_col.push(
                text(msg.as_str())
                    .size(12)
                    .align_x(iced::alignment::Horizontal::Center)
                    .style(move |_| iced::widget::text::Style {
                        color: Some(Color::from_rgb8(240, 140, 40)),
                    }),
            );
        }

        container(content_col)
            .width(Fill)
            .height(Fill)
            .center(Fill)
            .padding(16)
            .into()
    }

    /// 右侧首次拉取中的加载状态
    fn fetching_view(&self) -> Element<'_, Message> {
        let is_light = self.theme_mode().is_light();
        let dim_color = if is_light {
            Color::from_rgb8(120, 120, 130)
        } else {
            Color::from_rgb8(160, 160, 175)
        };
        let text_color = if is_light {
            Color::from_rgb8(20, 22, 38)
        } else {
            Color::WHITE
        };

        let icon = pomelo_widgets::icon(Icon::SYNC)
            .size(44)
            .style(move |_| iced::widget::text::Style { color: Some(dim_color) });

        let title = text(if self.language() == Language::English {
            "Syncing Schedule..."
        } else {
            "正在拉取今日日程..."
        })
        .size(16)
        .style(move |_| iced::widget::text::Style { color: Some(text_color) });

        let hint = text(if self.language() == Language::English {
            "Connecting to calendar service"
        } else {
            "连接日历服务中"
        })
        .size(13)
        .style(move |_| iced::widget::text::Style { color: Some(dim_color) });

        container(
            column![icon, title, hint]
                .spacing(12)
                .align_x(iced::Alignment::Center),
        )
        .width(Fill)
        .height(Fill)
        .center(Fill)
        .into()
    }

    /// 右侧同步失败且尚无缓存事件时的错误视图
    fn error_view(&self) -> Element<'_, Message> {
        let is_light = self.theme_mode().is_light();
        let dim_color = if is_light {
            Color::from_rgb8(120, 120, 130)
        } else {
            Color::from_rgb8(160, 160, 175)
        };
        let text_color = if is_light {
            Color::from_rgb8(20, 22, 38)
        } else {
            Color::WHITE
        };
        let error_color = Color::from_rgb8(235, 87, 87);

        let icon = pomelo_widgets::icon(Icon::SYNC_PROBLEM)
            .size(44)
            .style(move |_| iced::widget::text::Style { color: Some(error_color) });

        let title = text(if self.language() == Language::English {
            "Sync Failed"
        } else {
            "日程同步失败"
        })
        .size(17)
        .style(move |_| iced::widget::text::Style { color: Some(text_color) });

        let hint = text(if self.language() == Language::English {
            "Please check network connection\nor calendar subscription URL"
        } else {
            "请检查网络连接\n或订阅链接是否有效"
        })
        .size(13)
        .align_x(iced::alignment::Horizontal::Center)
        .style(move |_| iced::widget::text::Style { color: Some(dim_color) });

        let retry_btn = iced::widget::button(
            text(if self.language() == Language::English {
                "Retry Sync"
            } else {
                "重试同步"
            })
            .size(14)
            .align_x(iced::alignment::Horizontal::Center),
        )
        .on_press(Message::Refresh)
        .padding([8, 16])
        .style(move |theme: &Theme, status| {
            let palette = theme.extended_palette();
            let bg = match status {
                iced::widget::button::Status::Pressed => palette.primary.strong.color,
                iced::widget::button::Status::Hovered => palette.primary.base.color,
                _ => palette.primary.base.color,
            };
            iced::widget::button::Style {
                background: Some(bg.into()),
                text_color: Color::WHITE,
                border: iced::Border {
                    radius: iced::border::Radius::from(8.0),
                    ..Default::default()
                },
                ..Default::default()
            }
        });

        let settings_tip = text(if self.language() == Language::English {
            "Check network or verify link in Settings > Apps"
        } else {
            "请检查网络或在「设置 -> 应用」中核对链接"
        })
        .size(11)
        .align_x(iced::alignment::Horizontal::Center)
        .style(move |_| iced::widget::text::Style {
            color: Some(Color::from_rgb8(140, 140, 160)),
        });

        container(
            column![icon, title, hint, retry_btn, settings_tip]
                .spacing(12)
                .align_x(iced::Alignment::Center),
        )
        .width(Fill)
        .height(Fill)
        .center(Fill)
        .padding(16)
        .into()
    }

    /// 正常时间轴界面（顶部带有微型快捷操作条）
    fn timeline_view(&self) -> Element<'_, Message> {
        let is_en = self.language() == Language::English;
        let is_light = self.theme_mode().is_light();
        let dim_color = if is_light {
            Color::from_rgb8(120, 120, 130)
        } else {
            Color::from_rgb8(150, 150, 165)
        };

        let status_text = if let Some((msg, _)) = &self.toast_message {
            text(msg.as_str())
                .size(11)
                .style(move |_| iced::widget::text::Style {
                    color: Some(Color::from_rgb8(240, 140, 40)),
                })
        } else if self.events.is_empty() {
            text(if is_en { "No events today" } else { "今日暂无日程" })
                .size(11)
                .style(move |_| iced::widget::text::Style { color: Some(dim_color) })
        } else {
            text("").size(11)
        };

        let top_bar = row![status_text]
            .width(Fill)
            .padding([2, 8])
            .align_y(iced::Alignment::Center);

        let timeline = Timeline::with_events(self.now, self.events.clone()).view();

        column![top_bar, timeline]
            .width(Fill)
            .height(Fill)
            .into()
    }

    pub fn view(&self) -> Element<'_, Message> {
        let left_panel = time_display::view(self.now, self.language());

        let right_panel: Element<'_, Message> = if self.config.subscription_url.is_none() {
            // 没有订阅链接：显示提示并提供“从剪切板导入”按钮
            self.no_subscription_view()
        } else if self.sync_status == SyncStatus::Fetching && self.events.is_empty() {
            // 正在拉取中且尚无事件：显示加载状态
            self.fetching_view()
        } else if self.sync_status == SyncStatus::Error && self.events.is_empty() {
            // 同步失败且无事件：显示错误状态
            self.error_view()
        } else {
            // 缓存存在或已有数据：进入UI绘制（时间轴）
            self.timeline_view()
        };

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

        // 布局辅助分割线（位于正中，方便分析左侧时间与右侧时间轴的 UI 布局）
        let separator_color = if is_light {
            Color::from_rgba8(0, 0, 0, 0.25)
        } else {
            Color::from_rgba8(255, 255, 255, 0.35)
        };
        let separator: Element<'_, Message> = container(iced::widget::space())
            .width(1.0)
            .height(Fill)
            .style(move |_theme| container::Style {
                background: Some(separator_color.into()),
                ..Default::default()
            })
            .into();

        container(
            row![left_panel, separator, right_panel]
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

#[cfg(not(target_os = "espidf"))]
impl Default for DailySchedule {
    fn default() -> Self {
        Self::new()
    }
}
