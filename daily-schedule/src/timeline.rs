use chrono::{DateTime, Local, Timelike};
use iced::alignment;
use iced::mouse;
use iced::widget::canvas::{Geometry, Path};
use iced::widget::operation;
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{canvas, container, scrollable, text};
use iced::{Color, Element, Fill, Point, Rectangle, Renderer, Task, Theme};
use pomelo_material_symbols::Icon;

use crate::app::Message;
use crate::model::ScheduleEvent;

/// 竖直时间轴组件（可上下滚动，隐藏滚动条）
pub struct Timeline {
    pub now: DateTime<Local>,
    pub events: Vec<ScheduleEvent>,
}

impl Timeline {
    /// 唯一标识该可滚动时间轴
    pub const ID: &str = "timeline_scrollable";

    /// Canvas 内部上下端点缓冲各 16px，轴线净长 1440px（每小时 60px，每分钟 1px）
    pub const PAD_Y: f32 = 16.0;
    pub const AXIS_HEIGHT: f32 = 1440.0;
    pub const TOTAL_HEIGHT: f32 = Self::AXIS_HEIGHT + Self::PAD_Y * 2.0;

    pub fn new(now: DateTime<Local>) -> Self {
        Self {
            now,
            events: Vec::new(),
        }
    }

    pub fn with_events(now: DateTime<Local>, events: Vec<ScheduleEvent>) -> Self {
        Self { now, events }
    }

    /// 产生一个让时间轴对焦到当前时间（居中显示）的操作任务
    pub fn focus(now: DateTime<Local>) -> Task<Message> {
        Self::scroll_to_y(Self::target_y(now))
    }

    /// 计算当前时间点在时间轴上居中所需的滚动偏移 Y
    pub fn target_y(now: DateTime<Local>) -> f32 {
        let total_minutes =
            (now.hour() * 60 + now.minute()) as f32 + (now.second() as f32 / 60.0);
        let current_y = Self::PAD_Y + total_minutes;

        // 视口净高 416 (480 - 64 padding)，中心为 208
        let viewport_height = 416.0;
        let max_scroll = (Self::TOTAL_HEIGHT - viewport_height).max(0.0);
        (current_y - viewport_height / 2.0).clamp(0.0, max_scroll)
    }

    /// 滚动到指定的绝对 Y 偏移量
    pub fn scroll_to_y(y: f32) -> Task<Message> {
        operation::scroll_to(
            Self::ID,
            scrollable::AbsoluteOffset {
                x: 0.0,
                y,
            },
        )
    }

    pub fn view<'a>(self) -> Element<'a, Message> {
        let content = canvas(self)
            .width(Fill)
            .height(Self::TOTAL_HEIGHT);

        // 隐藏 scrollbar，用户滑动或程序滚动时记录 offset
        let scroll = scrollable(content)
            .id(Self::ID)
            .direction(Direction::Vertical(Scrollbar::hidden()))
            .on_scroll(|viewport| Message::TimelineScrolled(viewport.absolute_offset()))
            .width(Fill)
            .height(Fill);

        // 外层 Container 静止不动，提供固定的上下与左右可视边距（上下 32px，左右 16px）
        // 内部的 canvas 在此安全视口内滚动，上下边距永远保持存在
        container(scroll)
            .width(Fill)
            .height(Fill)
            .padding([32.0, 16.0])
            .into()
    }
}

impl<Message> canvas::Program<Message> for Timeline {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let palette = theme.extended_palette();

        // 填充背景以确保滚动时 dirty-rect / damage 区域完整覆盖整个画布
        let bg_color = palette.background.base.color;
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), bg_color);

        let base_color = palette.background.strong.text;
        let axis_color = Color {
            a: 0.35,
            ..base_color
        };
        let event_time_color = Color {
            a: 0.72,
            ..base_color
        };

        let pad_y = Timeline::PAD_Y;
        let axis_height = Timeline::AXIS_HEIGHT;
        // 时间轴主线在 Canvas 内向左靠齐（外层 Container 已提供左右 16px padding）
        // 刻度文字(~34px) + 间距(4px) + 刻度线(6px) = 48px，使时间文字从左侧 ~4px 处自然展开
        let line_x = 48.0;

        // 1. 竖直轴主线（使用实心矩形避免 0 宽度线段被 damage 剔除）
        let axis_rect = Path::rectangle(
            Point::new(line_x - 1.0, pad_y),
            iced::Size::new(1.0, axis_height),
        );
        frame.fill(&axis_rect, axis_color);

        // 2. 刻度线与时间标签（00:00 ~ 24:00，每小时 60px，仅保留整点短线）
        let hour_tick_len = 6.0;
        for h in 0..=24 {
            let y_hour = pad_y + (h as f32) * 60.0;

            // 整点刻度线（6px）
            let hour_tick = Path::rectangle(
                Point::new(line_x - hour_tick_len, y_hour - 0.75),
                iced::Size::new(hour_tick_len, 1.5),
            );
            frame.fill(&hour_tick, axis_color);

            // 整点文字标注（如 "00:00", "01:00", ... "24:00"）
            frame.fill_text(canvas::Text {
                content: format!("{:02}:00", h % 24),
                position: Point::new(line_x - hour_tick_len - 4.0, y_hour),
                color: base_color,
                size: 12.0.into(),
                align_x: text::Alignment::Right,
                align_y: alignment::Vertical::Center,
                ..canvas::Text::default()
            });
        }

        // 3. 今日日程事件卡片 (7 色预设调色板)
        let event_palette = [
            (Color::from_rgb8(56, 139, 253), Color::from_rgba8(56, 139, 253, 0.22)), // 0: 蓝色 Blue
            (Color::from_rgb8(46, 160, 67), Color::from_rgba8(46, 160, 67, 0.22)),   // 1: 绿色 Green
            (Color::from_rgb8(219, 109, 40), Color::from_rgba8(219, 109, 40, 0.22)), // 2: 橙色 Orange
            (Color::from_rgb8(163, 113, 247), Color::from_rgba8(163, 113, 247, 0.22)), // 3: 紫色 Purple
            (Color::from_rgb8(248, 81, 73), Color::from_rgba8(248, 81, 73, 0.22)),   // 4: 红色 Red
            (Color::from_rgb8(57, 197, 207), Color::from_rgba8(57, 197, 207, 0.22)), // 5: 青色 Cyan
            (Color::from_rgb8(227, 179, 65), Color::from_rgba8(227, 179, 65, 0.22)), // 6: 黄色 Yellow
        ];

        // 卡片与指示线右侧边界直接对齐 Canvas 边缘（由外层 Container 的 16px padding 统一切分安全边距）
        let right_padding = 0.0;
        let card_x = line_x;
        let card_w = (bounds.width - card_x - right_padding).max(60.0);

        // 3.1 全天事件（置于 00:00 顶部，左侧靠齐时间轴）
        let all_day_events: Vec<_> = self.events.iter().filter(|e| e.is_all_day).collect();
        for (i, event) in all_day_events.iter().enumerate() {
            let start_y = pad_y + (i as f32) * 24.0;
            let (accent_color, bg_color) = event_palette[event.color_index % event_palette.len()];
            let card_rect = Path::rounded_rectangle(
                Point::new(card_x, start_y),
                iced::Size::new(card_w, 20.0),
                iced::border::Radius {
                    top_left: 0.0,
                    top_right: 5.0,
                    bottom_right: 5.0,
                    bottom_left: 0.0,
                },
            );
            frame.fill(&card_rect, bg_color);
            let bar_rect = Path::rectangle(
                Point::new(card_x, start_y),
                iced::Size::new(3.0, 20.0),
            );
            frame.fill(&bar_rect, accent_color);
            frame.fill_text(canvas::Text {
                content: format!("[全天] {}", event.title),
                position: Point::new(card_x + 8.0, start_y + 3.0),
                color: base_color,
                size: 11.5.into(),
                ..canvas::Text::default()
            });
        }

        // 3.2 普通定时事件（左侧靠齐时间轴）
        for event in &self.events {
            if event.is_all_day {
                continue;
            }

            let start_min = (event.start.hour() * 60 + event.start.minute()) as f32;
            let end_min = if event.end.date_naive() > event.start.date_naive() {
                1440.0
            } else {
                (event.end.hour() * 60 + event.end.minute()) as f32
            };
            let end_min = if end_min <= start_min {
                (start_min + 30.0).min(1440.0)
            } else {
                end_min
            };

            let start_y = pad_y + start_min;
            let end_y = pad_y + end_min;
            let card_h = (end_y - start_y).max(24.0);

            let (accent_color, bg_color) = event_palette[event.color_index % event_palette.len()];

            let card_rect = Path::rounded_rectangle(
                Point::new(card_x, start_y),
                iced::Size::new(card_w, card_h),
                iced::border::Radius {
                    top_left: 0.0,
                    top_right: 6.0,
                    bottom_right: 6.0,
                    bottom_left: 0.0,
                },
            );
            frame.fill(&card_rect, bg_color);

            let bar_rect = Path::rectangle(
                Point::new(card_x, start_y),
                iced::Size::new(3.5, card_h),
            );
            frame.fill(&bar_rect, accent_color);

            let show_time = card_h >= 42.0;

            if show_time {
                // 卡片高度充足：显示标题与时间，两者之间保留舒适的 gap
                frame.fill_text(canvas::Text {
                    content: event.title.clone(),
                    position: Point::new(card_x + 8.0, start_y + 5.5),
                    color: base_color,
                    size: 13.0.into(),
                    ..canvas::Text::default()
                });

                let time_str = format!(
                    "{:02}:{:02} - {:02}:{:02}",
                    event.start.hour(),
                    event.start.minute(),
                    event.end.hour(),
                    event.end.minute()
                );

                let content_x = card_x + 8.0;
                let time_y = start_y + 29.0;

                // 闹钟图标
                frame.fill_text(canvas::Text {
                    content: Icon::ALARM.glyph().to_string(),
                    position: Point::new(content_x, time_y),
                    color: event_time_color,
                    size: 11.0.into(),
                    align_y: alignment::Vertical::Center,
                    font: pomelo_material_symbols::font(),
                    ..canvas::Text::default()
                });

                // 时间文本（与闹钟图标颜色保持一致且明亮）
                frame.fill_text(canvas::Text {
                    content: time_str,
                    position: Point::new(content_x + 14.0, time_y),
                    color: event_time_color,
                    size: 10.5.into(),
                    align_y: alignment::Vertical::Center,
                    ..canvas::Text::default()
                });
            } else {
                // 卡片高度过小：不显示时间，标题单行垂直居中展示
                frame.fill_text(canvas::Text {
                    content: event.title.clone(),
                    position: Point::new(card_x + 8.0, start_y + card_h / 2.0),
                    color: base_color,
                    size: 12.0.into(),
                    align_y: alignment::Vertical::Center,
                    ..canvas::Text::default()
                });
            }
        }

        // 4. 当前时间位置标记（每分钟 1px）
        let total_minutes = (self.now.hour() * 60 + self.now.minute()) as f32;
        let current_y = pad_y + total_minutes;

        // 玫瑰红游标颜色（rose-500: #F43F5E）
        let cursor_color = Color::from_rgb8(244, 63, 94);

        // 当前时间向右一直延伸到边界的水平指示线
        let line_w = (bounds.width - line_x).max(0.0);
        let mark_rect = Path::rectangle(
            Point::new(line_x, current_y - 1.0),
            iced::Size::new(line_w, 2.0),
        );
        frame.fill(&mark_rect, cursor_color);

        // 当前时间轴线上的小圆点
        let dot = Path::circle(Point::new(line_x, current_y), 4.5);
        frame.fill(&dot, cursor_color);

        vec![frame.into_geometry()]
    }
}

