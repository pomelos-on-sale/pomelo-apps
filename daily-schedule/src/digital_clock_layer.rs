//! Digital clock layer for Daily Schedule.
//!
//! Consciously occupies the left half (50%) of the screen, rendering
//! the date, day of week, hours, and minutes, leaving the right half
//! transparent for layered composition in the root stack.

use chrono::{DateTime, Datelike, Local, Timelike, Weekday};
use iced::alignment;
use iced::widget::{column, container, row, space, text};
use iced::{Element, Fill};

use pomelo_widgets::preferences::{FontSizeTier, Language};

/// 渲染数字时钟图层：
/// 占据左半边（50% 宽度），中间垂直居中显示两行大号时分，上方显示日期星期。
/// 右半边保持透明留空，供顶层 schedule_layer 展现。
pub fn view<'a, Message: 'a>(now: DateTime<Local>, language: Language) -> Element<'a, Message> {
    // 1. 日期和星期
    let weekday_str = match (language, now.weekday()) {
        (Language::English, Weekday::Mon) => "Mon",
        (Language::English, Weekday::Tue) => "Tue",
        (Language::English, Weekday::Wed) => "Wed",
        (Language::English, Weekday::Thu) => "Thu",
        (Language::English, Weekday::Fri) => "Fri",
        (Language::English, Weekday::Sat) => "Sat",
        (Language::English, Weekday::Sun) => "Sun",
        (_, Weekday::Mon) => "周一",
        (_, Weekday::Tue) => "周二",
        (_, Weekday::Wed) => "周三",
        (_, Weekday::Thu) => "周四",
        (_, Weekday::Fri) => "周五",
        (_, Weekday::Sat) => "周六",
        (_, Weekday::Sun) => "周日",
    };
    let date_text = match language {
        Language::English => {
            let month_str = match now.month() {
                1 => "Jan",
                2 => "Feb",
                3 => "Mar",
                4 => "Apr",
                5 => "May",
                6 => "Jun",
                7 => "Jul",
                8 => "Aug",
                9 => "Sep",
                10 => "Oct",
                11 => "Nov",
                _ => "Dec",
            };
            format!("{}, {} {}", weekday_str, month_str, now.day())
        }
        Language::Chinese => format!("{}月{}日 {}", now.month(), now.day(), weekday_str),
    };
    let element1 = text(date_text).size(FontSizeTier::Standard.base_size());

    // 2. 时间 Column[Hours, Minutes]
    let hour_widget = text(format!("{:02}", now.hour()))
        .size(150)
        .line_height(1.0);

    let minute_widget = text(format!("{:02}", now.minute()))
        .size(150)
        .line_height(1.0);

    let time_column = column![hour_widget, minute_widget]
        .align_x(alignment::Horizontal::Center)
        .spacing(2);

    // 上下对称的槽位高度（确保中间的时间绝对处于垂直正中）
    const SLOT_HEIGHT: f32 = 32.0;

    let top_container = container(element1)
        .height(SLOT_HEIGHT)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center);

    let bottom_container = container(space())
        .height(SLOT_HEIGHT);

    // 内部 Column: [top_container, time_column, bottom_container]
    let left_column = column![top_container, time_column, bottom_container]
        .align_x(alignment::Horizontal::Center)
        .spacing(12);

    let left_panel = container(left_column)
        .width(Fill)
        .height(Fill)
        .center(Fill);

    // 自觉占据左半边（50%），右半边留空透明
    row![left_panel, space().width(Fill)]
        .width(Fill)
        .height(Fill)
        .into()
}
