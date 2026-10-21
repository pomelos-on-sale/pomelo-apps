//! Schedule layer for Daily Schedule.
//!
//! Consciously occupies the right half (50%) of the screen, rendering
//! the interactive schedule timeline with event cards and current time marker,
//! leaving the left half transparent for layered composition in the root stack.

use chrono::{DateTime, Local};
use iced::widget::row;
use iced::{Color, Element, Fill};
use pomelo_widgets::ThemeMode;

use crate::app::Message;
use crate::model::ScheduleEvent;
use crate::timeline::Timeline;

/// 渲染日程时间轴图层：
/// 占据右半边（50% 宽度），承载时间轴滚动与日程卡片交互。
/// 左半边保持透明留空（Space），供底层的时钟图层展现。
pub fn view<'a>(
    now: DateTime<Local>,
    events: &[ScheduleEvent],
    theme_mode: ThemeMode,
    graphic_clock: bool,
) -> Element<'a, Message> {
    let is_light = theme_mode.is_light();
    let bg_color = if graphic_clock {
        Color::TRANSPARENT
    } else if is_light {
        Color::from_rgb8(242, 242, 247)
    } else {
        Color::BLACK
    };
    let text_color = if is_light {
        Color::from_rgb8(20, 22, 38)
    } else {
        Color::WHITE
    };

    let timeline_panel = Timeline::with_events(now, events.to_vec(), bg_color, text_color).view();

    // 自觉占据右半边（50%），左半边留空透明
    row![
        iced::widget::space().width(Fill),
        timeline_panel,
    ]
    .width(Fill)
    .height(Fill)
    .into()
}
