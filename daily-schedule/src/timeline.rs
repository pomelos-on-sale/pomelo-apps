use chrono::{DateTime, Local, Timelike};
use iced::alignment;
use iced::mouse;
use iced::widget::canvas::{Geometry, LineCap, Path, Stroke, stroke};
use iced::widget::operation;
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{canvas, container, scrollable, text};
use iced::{Color, Element, Fill, Font, Point, Rectangle, Renderer, Task, Theme};

use crate::app::Message;

/// 竖直时间轴组件（可上下滚动，隐藏滚动条）
pub struct Timeline {
    pub now: DateTime<Local>,
}

impl Timeline {
    /// 唯一标识该可滚动时间轴
    pub const ID: &str = "timeline_scrollable";

    /// Canvas 内部上下端点缓冲各 16px，轴线净长 1440px（每小时 60px，每分钟 1px）
    pub const PAD_Y: f32 = 16.0;
    pub const AXIS_HEIGHT: f32 = 1440.0;
    pub const TOTAL_HEIGHT: f32 = Self::AXIS_HEIGHT + Self::PAD_Y * 2.0;

    pub fn new(now: DateTime<Local>) -> Self {
        Self { now }
    }

    /// 产生一个让时间轴对焦到当前时间（居中显示）的操作任务
    pub fn focus(now: DateTime<Local>) -> Task<Message> {
        let total_minutes =
            (now.hour() * 60 + now.minute()) as f32 + (now.second() as f32 / 60.0);
        let current_y = Self::PAD_Y + total_minutes;

        // 视口净高 416 (480 - 64 padding)，中心为 208
        let viewport_height = 416.0;
        let max_scroll = (Self::TOTAL_HEIGHT - viewport_height).max(0.0);
        let target_y = (current_y - viewport_height / 2.0).clamp(0.0, max_scroll);

        operation::scroll_to(
            Self::ID,
            scrollable::AbsoluteOffset {
                x: 0.0,
                y: target_y,
            },
        )
    }

    pub fn view<'a>(self) -> Element<'a, Message> {
        let content = canvas(self)
            .width(Fill)
            .height(Self::TOTAL_HEIGHT);

        // 隐藏 scrollbar，用户滑动时触发 Message::TimelineScrolled
        let scroll = scrollable(content)
            .id(Self::ID)
            .direction(Direction::Vertical(Scrollbar::hidden()))
            .on_scroll(|_viewport| Message::TimelineScrolled)
            .width(Fill)
            .height(Fill);

        // 外层 Container 静止不动，提供固定的上下可视边距（上下 32px，左右 0px）
        // 内部的 canvas 在此安全视口内滚动，上下边距永远保持存在
        container(scroll)
            .width(Fill)
            .height(Fill)
            .padding([32.0, 0.0])
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

        let base_color = palette.background.strong.text;
        let axis_color = Color {
            a: 0.35,
            ..base_color
        };
        let tick_dim_color = Color {
            a: 0.2,
            ..base_color
        };

        let pad_y = Timeline::PAD_Y;
        let axis_height = Timeline::AXIS_HEIGHT;
        let line_x = (bounds.width * 0.38).max(60.0);

        // 1. 竖直轴主线
        let axis_line = Path::line(
            Point::new(line_x, pad_y),
            Point::new(line_x, pad_y + axis_height),
        );
        frame.stroke(
            &axis_line,
            Stroke {
                width: 2.0,
                style: stroke::Style::Solid(axis_color),
                line_cap: LineCap::Round,
                ..Stroke::default()
            },
        );

        // 2. 刻度线与时间标签（00:00 ~ 24:00，每小时 60px）
        for h in 0..=24 {
            let y_hour = pad_y + (h as f32) * 60.0;

            // 整点刻度线（长刻度，12px）
            let hour_tick = Path::line(
                Point::new(line_x - 12.0, y_hour),
                Point::new(line_x, y_hour),
            );
            frame.stroke(
                &hour_tick,
                Stroke {
                    width: 1.5,
                    style: stroke::Style::Solid(axis_color),
                    ..Stroke::default()
                },
            );

            // 整点文字标注（如 "00:00", "01:00", ... "24:00"）
            frame.fill_text(canvas::Text {
                content: format!("{:02}:00", h % 24),
                position: Point::new(line_x - 16.0, y_hour),
                color: base_color,
                size: 12.0.into(),
                align_x: text::Alignment::Right,
                align_y: alignment::Vertical::Center,
                font: Font::MONOSPACE,
                ..canvas::Text::default()
            });

            // 绘制小时内的子刻度（每 15 分钟）
            if h < 24 {
                for m in [15, 30, 45] {
                    let y_sub = y_hour + (m as f32);
                    let (tick_len, stroke_color) = if m == 30 {
                        (7.0, axis_color) // 半点刻度
                    } else {
                        (4.0, tick_dim_color) // 15分/45分细刻度
                    };

                    let sub_tick = Path::line(
                        Point::new(line_x - tick_len, y_sub),
                        Point::new(line_x, y_sub),
                    );
                    frame.stroke(
                        &sub_tick,
                        Stroke {
                            width: 1.0,
                            style: stroke::Style::Solid(stroke_color),
                            ..Stroke::default()
                        },
                    );
                }
            }
        }

        // 3. 当前时间位置标记（每分钟 1px）
        let total_minutes = (self.now.hour() * 60 + self.now.minute()) as f32;
        let current_y = pad_y + total_minutes;

        // 当前时间的小圆点
        let dot = Path::circle(Point::new(line_x, current_y), 5.0);
        frame.fill(&dot, palette.primary.base.color);

        // 当前时间向右延伸的水平刻度线
        let mark_line = Path::line(
            Point::new(line_x, current_y),
            Point::new(line_x + 24.0, current_y),
        );
        frame.stroke(
            &mark_line,
            Stroke {
                width: 2.0,
                style: stroke::Style::Solid(palette.primary.base.color),
                ..Stroke::default()
            },
        );

        vec![frame.into_geometry()]
    }
}
