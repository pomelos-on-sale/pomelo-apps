use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Local, NaiveDate, Timelike};
use iced::widget::scrollable::AbsoluteOffset;
use iced::widget::{container, row, Column, Space, text};
use iced::{Color, Element, Fill, Length, Subscription, Task, Theme};
use pomelo_hal::Board;
use pomelo_widgets::{
    dialog_button, dialog_card, modal_dialog, AnimationController, AppPreferences, Curve,
    DialogButtonStyle, FontSizeTier, Language, SystemPreferences, ThemeMode,
};

use crate::ical;
use crate::model::{self, ScheduleConfig, ScheduleEvent};
use crate::sound::ScheduleSounds;
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
    current_scroll_y: f32,
    scroll_animation: Option<AnimationController>,
    last_minute: Option<u32>,
    initial_focused: bool,
    preferences: SystemPreferences,
    board: Arc<Board>,

    config: ScheduleConfig,
    events: Vec<ScheduleEvent>,
    tomorrow_events: Vec<ScheduleEvent>,
    sync_status: SyncStatus,
    current_date: NaiveDate,
    toast_message: Option<(String, Instant)>,
    is_programmatic_focus: bool,

    dismissed_no_subscription: bool,
    dismissed_fetching: bool,
    dismissed_error: bool,
    last_sync_at: Option<Instant>,
    is_silent_fetch: bool,

    pub sounds: ScheduleSounds,
    played_start_alerts: HashSet<String>,
    played_end_alerts: HashSet<String>,
}

pub type App = DailySchedule;

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    Tick(DateTime<Local>),
    ScrollFrame(Instant),
    TimelineScrolled(AbsoluteOffset),
    ImportFromClipboard,
    ClipboardImported(Option<String>),
    Refresh,
    ClearSubscription,
    DismissDialog,
}


fn tick_stream() -> impl iced::futures::Stream<Item = Message> {
    pomelo_widgets::interval_stream("daily-schedule-timer", Duration::from_millis(1000), || {
        Message::Tick(Local::now())
    })
}


fn sanitize_calendar_url(raw: &str) -> Option<String> {
    let s = raw.trim();
    let s = s.trim_matches(|c| c == '"' || c == '\'' || c == '<' || c == '>');
    let s = s.trim();
    if let Some(rest) = s.strip_prefix("webcal://") {
        Some(format!("https://{}", rest))
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
        let (events, tomorrow_events, sync_status, last_sync_at) = if config.subscription_url.is_some() {
            // 链接存在，检查缓存是否存在且包含今日（或昨日保存的今日）日程
            if let Some((mut cached_today, mut cached_tomorrow)) = model::load_schedule_cache(today) {
                // 缓存存在且有效，根据当前颜色规则重新计算颜色并直接加载进入 UI 绘制
                for evt in &mut cached_today {
                    evt.color_index = ical::compute_color_index_with_rules(&evt.title, &config.color_rules);
                }
                for evt in &mut cached_tomorrow {
                    evt.color_index = ical::compute_color_index_with_rules(&evt.title, &config.color_rules);
                }
                (cached_today, cached_tomorrow, SyncStatus::Idle, Some(Instant::now()))
            } else {
                // 缓存不存在或已失效，准备拉取
                (Vec::new(), Vec::new(), SyncStatus::Idle, None)
            }
        } else {
            (Vec::new(), Vec::new(), SyncStatus::Idle, None)
        };

        let mut app = Self {
            now,
            user_scrolled_at: None,
            current_scroll_y: Timeline::target_y(now),
            scroll_animation: None,
            last_minute: None,
            initial_focused: false,
            preferences: SystemPreferences::default(),
            board,
            config,
            events,
            tomorrow_events,
            sync_status,
            current_date: today,
            toast_message: None,
            is_programmatic_focus: false,
            dismissed_no_subscription: false,
            dismissed_fetching: false,
            dismissed_error: false,
            last_sync_at,
            is_silent_fetch: false,
            sounds: ScheduleSounds::init_and_load(),
            played_start_alerts: HashSet::new(),
            played_end_alerts: HashSet::new(),
        };

        // 如果配置了日历订阅：
        // 1. 若没有当天缓存，启动时自动显式拉取（带居中加载弹窗）；
        // 2. 若已有缓存，UI 瞬间展示缓存的同时，立即在后台发起一次静默拉取保鲜。
        if app.config.subscription_url.is_some() {
            if model::load_schedule_cache(today).is_none() {
                app.trigger_fetch();
            } else {
                app.trigger_silent_fetch();
            }
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
        self.config = model::load_config();
        if self.config.subscription_url.is_none() {
            self.dismissed_no_subscription = false;
            self.dismissed_fetching = false;
            self.dismissed_error = false;
        }
        self.now = Local::now();
        self.last_minute = None;
        self.scroll_animation = None;
        self.user_scrolled_at = None;
        self.initial_focused = false;
        self.is_programmatic_focus = true;
        self.current_scroll_y = Timeline::target_y(self.now);

        // 检查跨日：若在后台期间跨过零点，直接将预存的明天日程升格为今日日程
        let today = self.now.date_naive();
        if today != self.current_date {
            self.current_date = today;
            self.played_start_alerts.clear();
            self.played_end_alerts.clear();
            if !self.tomorrow_events.is_empty() {
                self.events = std::mem::take(&mut self.tomorrow_events);
                let _ = model::save_schedule_cache(today, &self.events, &[]);
            } else if let Some((cached_today, cached_tomorrow)) = model::load_schedule_cache(today) {
                self.events = cached_today;
                self.tomorrow_events = cached_tomorrow;
            } else {
                self.events.clear();
            }
        }

        // 切回前台保鲜检查：若已配置日历、当前空闲且距离上次成功拉取超过 10 分钟，自动静默拉取
        if self.config.subscription_url.is_some()
            && self.sync_status == SyncStatus::Idle
            && self.last_sync_at.is_none_or(|t| t.elapsed() >= Duration::from_secs(600))
        {
            self.trigger_silent_fetch();
        }

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

    pub fn config(&self) -> &ScheduleConfig {
        &self.config
    }

    pub fn events(&self) -> &[ScheduleEvent] {
        &self.events
    }

    pub fn sync_status(&self) -> SyncStatus {
        self.sync_status
    }

    /// 发起显式拉取（显示同步中/失败弹窗）
    pub fn trigger_fetch(&mut self) {
        self.is_silent_fetch = false;
        self.dismissed_fetching = false;
        self.dismissed_error = false;
        self.start_http_fetch();
    }

    /// 发起后台静默拉取（不弹出全屏对话框，失败时静默重试不干扰用户）
    pub fn trigger_silent_fetch(&mut self) {
        self.is_silent_fetch = true;
        self.dismissed_fetching = true;
        self.dismissed_error = true;
        self.start_http_fetch();
    }

    fn start_http_fetch(&mut self) {
        self.config = model::load_config();
        if let Some(ref url) = self.config.subscription_url {
            if let Some(clean_url) = sanitize_calendar_url(url) {
                self.sync_status = SyncStatus::Fetching;
                if let Err(e) = self.board.http().start(&clean_url) {
                    if !self.is_silent_fetch {
                        self.sync_status = SyncStatus::Error;
                        let err_msg = if self.language() == Language::English {
                            format!("Failed to start sync: {}", e)
                        } else {
                            format!("无法发起同步: {}", e)
                        };
                        self.toast_message = Some((
                            err_msg,
                            Instant::now() + Duration::from_secs(4),
                        ));
                    } else {
                        self.sync_status = SyncStatus::Idle;
                        self.last_sync_at = Some(Instant::now());
                    }
                }
            } else if !self.is_silent_fetch {
                self.sync_status = SyncStatus::Error;
                let err_msg = if self.language() == Language::English {
                    "Invalid calendar subscription URL".to_string()
                } else {
                    "日历订阅链接格式无效".to_string()
                };
                self.toast_message = Some((
                    err_msg,
                    Instant::now() + Duration::from_secs(4),
                ));
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
        self.dismissed_no_subscription = false;
        self.dismissed_fetching = false;
        self.dismissed_error = false;

        let today = self.now.date_naive();
        if let Some((cached_today, cached_tomorrow)) = model::load_schedule_cache(today) {
            self.events = cached_today;
            self.tomorrow_events = cached_tomorrow;
            self.sync_status = SyncStatus::Idle;
            self.last_sync_at = Some(Instant::now());
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
                    "Link imported, fetching schedule...".to_string()
                } else {
                    "已导入链接，正在拉取日程...".to_string()
                },
                Instant::now() + Duration::from_secs(3),
            ));
            self.trigger_fetch();
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::TimelineScrolled(offset) => {
                self.current_scroll_y = offset.y;
                if self.is_programmatic_focus || !self.initial_focused {
                    self.is_programmatic_focus = false;
                    return Task::none();
                }
                // 若 offset 极度贴近当前目标居中点，说明属于程序重置或对焦到达，而非用户脱离居中位置
                let target_y = Timeline::target_y(self.now);
                if (offset.y - target_y).abs() <= 2.0 {
                    return Task::none();
                }
                // 若正在执行滚动动画，且当前 offset 贴近动画上一帧计算值，说明属于动画帧滚动
                if let Some(anim) = &self.scroll_animation {
                    if (offset.y - anim.value()).abs() <= 2.0 {
                        return Task::none();
                    }
                }
                // 用户手动滚动：打断平滑动画，记录操作时刻，进入 10 秒防打扰期
                self.scroll_animation = None;
                self.user_scrolled_at = Some(Instant::now());
                Task::none()
            }
            Message::ScrollFrame(instant) => {
                if !self.initial_focused {
                    self.initial_focused = true;
                    self.is_programmatic_focus = true;
                    self.user_scrolled_at = None;
                    self.current_scroll_y = Timeline::target_y(self.now);
                    return Timeline::focus(self.now);
                }
                if let Some(mut anim) = self.scroll_animation.take() {
                    let is_running = anim.update(instant);
                    let next_y = anim.value();
                    self.current_scroll_y = next_y;
                    if is_running {
                        self.scroll_animation = Some(anim);
                    }
                    self.is_programmatic_focus = true;
                    Timeline::scroll_to_y(next_y)
                } else {
                    Task::none()
                }
            }
            Message::ImportFromClipboard => {
                if self.sync_status == SyncStatus::Fetching {
                    Task::none()
                } else {
                    iced::clipboard::read().map(Message::ClipboardImported)
                }
            }
            Message::ClipboardImported(content) => {
                let clipboard_text = content.unwrap_or_default();
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
                self.dismissed_no_subscription = false;
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
            Message::DismissDialog => {
                if self.config.subscription_url.is_none() {
                    self.dismissed_no_subscription = true;
                } else if self.sync_status == SyncStatus::Fetching {
                    self.dismissed_fetching = true;
                } else if self.sync_status == SyncStatus::Error {
                    self.dismissed_error = true;
                    self.sync_status = SyncStatus::Idle;
                    if self.events.is_empty() {
                        self.config.subscription_url = None;
                        let _ = model::save_config(&self.config);
                    }
                }
                self.is_programmatic_focus = true;
                self.user_scrolled_at = None;
                self.current_scroll_y = Timeline::target_y(self.now);
                Timeline::focus(self.now)
            }

            Message::Tick(now) => {
                let minute_changed = self.last_minute.is_none_or(|m| m != now.minute());
                self.last_minute = Some(now.minute());
                self.now = now;

                // 检查跨日：到零点时，直接加载已缓存的明天日程，无需等待网络！
                let today = now.date_naive();
                if today != self.current_date {
                    self.current_date = today;
                    self.played_start_alerts.clear();
                    self.played_end_alerts.clear();
                    if !self.tomorrow_events.is_empty() {
                        self.events = std::mem::take(&mut self.tomorrow_events);
                        let _ = model::save_schedule_cache(today, &self.events, &[]);
                    } else if let Some((cached_today, cached_tomorrow)) = model::load_schedule_cache(today) {
                        self.events = cached_today;
                        self.tomorrow_events = cached_tomorrow;
                    } else {
                        self.events.clear();
                    }
                    if self.config.subscription_url.is_some() {
                        self.trigger_silent_fetch();
                    }
                }

                // 检查日程开始与结束提示音（开启且新的一分钟到达时检查）
                if minute_changed && self.config.reminder_sound {
                    let mut should_play_start = false;
                    let mut should_play_end = false;

                    for event in &self.events {
                        if event.is_all_day {
                            continue;
                        }
                        if event.start.date_naive() == today
                            && event.start.hour() == now.hour()
                            && event.start.minute() == now.minute()
                        {
                            let key = format!("{}_start_{}", event.id, event.start.timestamp());
                            if !self.played_start_alerts.contains(&key) {
                                self.played_start_alerts.insert(key);
                                should_play_start = true;
                            }
                        }
                        if event.end.date_naive() == today
                            && event.end.hour() == now.hour()
                            && event.end.minute() == now.minute()
                        {
                            let key = format!("{}_end_{}", event.id, event.end.timestamp());
                            if !self.played_end_alerts.contains(&key) {
                                self.played_end_alerts.insert(key);
                                should_play_end = true;
                            }
                        }
                    }

                    if should_play_start {
                        let _ = self.board.set_display_power(true);
                        self.sounds.play_start(&self.board);
                    } else if should_play_end {
                        let _ = self.board.set_display_power(true);
                        self.sounds.play_end(&self.board);
                    }
                }

                // 检查异步 HTTP 拉取进度
                if self.sync_status == SyncStatus::Fetching {
                    let http_state = self.board.http().state();
                    match http_state {
                        pomelo_hal::HttpState::Done => {
                            if let Ok(payload) = self.board.http().take_result() {
                                let ics_str = String::from_utf8_lossy(&payload);
                                let (today_events, tomorrow_events) =
                                    ical::parse_schedule_events_with_rules(
                                        &ics_str,
                                        today,
                                        &self.config.color_rules,
                                    );
                                let _ = model::save_schedule_cache(
                                    today,
                                    &today_events,
                                    &tomorrow_events,
                                );
                                self.events = today_events;
                                self.tomorrow_events = tomorrow_events;
                                self.sync_status = SyncStatus::Idle;
                                self.last_sync_at = Some(Instant::now());

                                if !self.is_silent_fetch {
                                    self.is_programmatic_focus = true;
                                    self.current_scroll_y = Timeline::target_y(now);
                                    return Timeline::focus(now);
                                }
                            } else {
                                if !self.is_silent_fetch {
                                    self.sync_status = SyncStatus::Error;
                                } else {
                                    self.sync_status = SyncStatus::Idle;
                                    self.last_sync_at = Some(Instant::now());
                                }
                            }
                        }
                        pomelo_hal::HttpState::Error => {
                            if !self.is_silent_fetch {
                                self.sync_status = SyncStatus::Error;
                                self.toast_message = Some((
                                    if self.language() == Language::English {
                                        "Failed to sync schedule, check network".to_string()
                                    } else {
                                        "日程拉取失败，请检查网络".to_string()
                                    },
                                    Instant::now() + Duration::from_secs(4),
                                ));
                            } else {
                                self.sync_status = SyncStatus::Idle;
                                self.last_sync_at = Some(Instant::now());
                            }
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

                // 1. 初次启动第一帧对焦（容错后备）
                let focus_task = if !self.initial_focused {
                    self.initial_focused = true;
                    self.is_programmatic_focus = true;
                    self.user_scrolled_at = None;
                    self.current_scroll_y = Timeline::target_y(now);
                    Timeline::focus(now)
                } else if let Some(scrolled_at) = self.user_scrolled_at {
                    // 2. 状态判断：用户是否手动干预了滑动
                    if scrolled_at.elapsed() >= Duration::from_secs(10) {
                        // 已经满 10 秒无动作：解除用户锁定，启动平滑滚动动画回到当前时间点
                        self.user_scrolled_at = None;
                        let target_y = Timeline::target_y(now);
                        if (self.current_scroll_y - target_y).abs() >= 1.0 {
                            let mut anim = AnimationController::new(self.current_scroll_y);
                            anim.animate_to(
                                self.current_scroll_y,
                                target_y,
                                Duration::from_millis(500),
                                Curve::EaseOutCubic,
                                Instant::now(),
                            );
                            self.scroll_animation = Some(anim);
                        } else {
                            self.scroll_animation = None;
                        }
                        Task::none()
                    } else {
                        // 还在 10 秒等待期内：保持用户自由浏览位置，不强行复位
                        Task::none()
                    }
                } else if self.scroll_animation.is_none() && minute_changed {
                    // 3. 无用户干预，不在动画中，且分钟增加：微调跟随滚动，保持当前时间居中
                    self.is_programmatic_focus = true;
                    self.current_scroll_y = Timeline::target_y(now);
                    Timeline::focus(now)
                } else {
                    Task::none()
                };

                // 4. 定时自动刷新（前台常驻时每 10 分钟自动静默刷新今日与明天日程）
                if self.config.subscription_url.is_some()
                    && self.sync_status == SyncStatus::Idle
                    && self.last_sync_at.is_none_or(|t| t.elapsed() >= Duration::from_secs(600))
                {
                    self.trigger_silent_fetch();
                }

                focus_task
            }
        }
    }

    /// 当前激活的模态弹窗视图
    pub fn active_dialog(&self) -> Option<Element<'_, Message>> {
        let is_en = self.language() == Language::English;
        let theme = self.theme_mode();

        let (body_text, error_text, is_fetching) = if self.config.subscription_url.is_none() && !self.dismissed_no_subscription {
            let base = if is_en {
                "Configure your calendar subscription in Settings > Apps > Schedule, or import directly from the clipboard."
            } else {
                "可在「设置 -> 应用 -> 日程」中配置管理日历订阅链接，或直接从剪切板导入。"
            };
            let err = self.toast_message.as_ref().map(|(msg, _)| msg.as_str());
            (base, err, false)
        } else if self.sync_status == SyncStatus::Fetching && !self.dismissed_fetching && !self.is_silent_fetch {
            let msg = if is_en {
                "Connecting to calendar service and downloading today's events, please wait..."
            } else {
                "正在连接日历服务并下载今日日程，请稍候..."
            };
            (msg, None, true)
        } else if self.sync_status == SyncStatus::Error && !self.dismissed_error && !self.is_silent_fetch {
            let msg = if is_en {
                "Unable to sync schedule. Please verify your Wi-Fi network connection and calendar subscription URL."
            } else {
                "未能成功拉取日程。请检查 Wi-Fi 网络连接，或在设置中确认订阅链接是否有效。"
            };
            let err = self.toast_message.as_ref().map(|(msg, _)| msg.as_str());
            (msg, err, false)
        } else {
            return None;
        };

        let title = if is_en {
            "Import Calendar Link"
        } else {
            "导入订阅链接"
        };

        let top_button_label = if is_fetching {
            if is_en { "Syncing Schedule..." } else { "正在同步日程..." }
        } else {
            if is_en { "Import from Clipboard" } else { "从剪切板导入" }
        };

        let cancel_label = if is_en { "Cancel" } else { "取消" };

        let mut content = Column::new()
            .width(Fill)
            .spacing(12.0)
            .push(
                text(title)
                    .size(FontSizeTier::Standard.base_size())
                    .color(theme.text_primary()),
            )
            .push(
                text(body_text)
                    .size(FontSizeTier::Small.base_size())
                    .color(if self.sync_status == SyncStatus::Error {
                        Color::from_rgb8(255, 69, 58)
                    } else {
                        theme.text_secondary()
                    }),
            );

        if let Some(err) = error_text {
            content = content.push(
                text(err)
                    .size(FontSizeTier::Small.base_size())
                    .color(Color::from_rgb8(255, 69, 58)),
            );
        }

        let buttons = Column::new()
            .width(Fill)
            .spacing(10.0)
            .push(dialog_button(
                top_button_label,
                Message::ImportFromClipboard,
                DialogButtonStyle::Primary,
                theme,
            ))
            .push(dialog_button(
                cancel_label,
                Message::DismissDialog,
                DialogButtonStyle::Cancel,
                theme,
            ));

        content = content
            .push(Space::new().height(Length::Fixed(8.0)))
            .push(buttons);

        Some(modal_dialog(dialog_card(content, theme)))
    }

    /// 是否当前有模态弹窗正在显示
    pub fn has_active_dialog(&self) -> bool {
        self.active_dialog().is_some()
    }

    /// 正常时间轴界面
    fn timeline_view(&self) -> Element<'_, Message> {
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
        Timeline::with_events(self.now, self.events.clone(), bg_color, text_color).view()
    }

    pub fn view(&self) -> Element<'_, Message> {
        let left_panel = time_display::view(self.now, self.language());
        let right_panel = self.timeline_view();

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

        let base: Element<'_, Message> = container(
            row![left_panel, right_panel]
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
        .into();

        if let Some(dialog) = self.active_dialog() {
            iced::widget::stack![base, dialog].into()
        } else {
            base
        }
    }


    pub fn subscription(&self) -> Subscription<Message> {
        let timer = Subscription::run(tick_stream);
        if self.scroll_animation.is_some() || !self.initial_focused {
            Subscription::batch(vec![
                timer,
                iced::window::frames().map(Message::ScrollFrame),
            ])
        } else {
            timer
        }
    }

    /// 当前时间轴是否正在执行平滑回滚动画
    pub fn is_animating_scroll(&self) -> bool {
        self.scroll_animation.is_some()
    }

    /// 当前记录的时间轴滚动 Y 偏移量
    pub fn current_scroll_y(&self) -> f32 {
        self.current_scroll_y
    }

    pub fn set_user_scrolled_at(&mut self, at: Option<Instant>) {
        self.user_scrolled_at = at;
    }

    /// 明日的预存日程列表
    pub fn tomorrow_events(&self) -> &[ScheduleEvent] {
        &self.tomorrow_events
    }

    /// 上次同步完成或发起的时刻
    pub fn last_sync_at(&self) -> Option<Instant> {
        self.last_sync_at
    }

    /// 当前是否处于后台静默拉取状态
    pub fn is_silent_fetch(&self) -> bool {
        self.is_silent_fetch
    }

    pub fn set_last_sync_at(&mut self, at: Option<Instant>) {
        self.last_sync_at = at;
    }
}

impl AppPreferences for DailySchedule {
    fn preferences(&self) -> SystemPreferences {
        self.preferences
    }

    fn set_preferences(&mut self, preferences: SystemPreferences) {
        self.preferences = preferences;
    }
}


#[cfg(not(target_os = "espidf"))]
impl Default for DailySchedule {
    fn default() -> Self {
        Self::new()
    }
}
