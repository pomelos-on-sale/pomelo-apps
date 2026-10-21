//! Graphic clock layer for Daily Schedule.
//!
//! Renders a 24-hour circular dial with a thick gradient ring (diameter ~400px).
//! - Top (06:00, -90°): Sunrise warm gold
//! - Right (12:00, 0°): Midday clear sky blue
//! - Bottom (18:00, 90°): Sunset twilight glow
//! - Left (24:00/00:00, 180°): Deep midnight indigo (clearly visible on black)
//! - Cursor: Rose-500 (#F43F5E) matching the timeline cursor semantic.

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use chrono::{DateTime, Local, Timelike};
use iced::mouse;
use iced::widget::canvas::{self, Frame, Geometry, LineCap, LineJoin, Path, Stroke};
use iced::{Color, Element, Fill, Point, Radians, Rectangle, Renderer, Theme};
use pomelo_widgets::ThemeMode;

use crate::app::Message;

/// 直径约 400px 的圆形图形时钟
pub const CLOCK_DIAMETER: f32 = 400.0;
pub const CLOCK_RADIUS: f32 = CLOCK_DIAMETER / 2.0; // 200.0 px
pub const RING_THICKNESS: f32 = 30.0; // 向内增加的圆环厚度
pub const INNER_RADIUS: f32 = CLOCK_RADIUS - RING_THICKNESS; // 170.0 px
pub const MID_RADIUS: f32 = CLOCK_RADIUS - RING_THICKNESS / 2.0; // 185.0 px

/// 玫瑰红游标颜色（与日程时间轴游标保持 100% 语意一致：rose-500: #F43F5E）
pub const CURSOR_COLOR: Color = Color::from_rgb(
    244.0 / 255.0,
    63.0 / 255.0,
    94.0 / 255.0,
);

/// 4 大关键昼夜渐变时间锚点色：
/// 1. 06:00 晨光破晓 (暖金/朝阳橙)
pub const COLOR_DAWN: Color = Color::from_rgb(
    255.0 / 255.0,
    178.0 / 255.0,
    62.0 / 255.0,
);
/// 2. 12:00 盛昼天光 (清澈亮天蓝)
pub const COLOR_NOON: Color = Color::from_rgb(
    56.0 / 255.0,
    189.0 / 255.0,
    248.0 / 255.0,
);
/// 3. 18:00 晚霞落日 (暮霭粉紫/夕阳红)
pub const COLOR_DUSK: Color = Color::from_rgb(
    244.0 / 255.0,
    114.0 / 255.0,
    182.0 / 255.0,
);
/// 4. 24:00 深宵星夜 (深沉静谧夜空灰蓝，在黑色背景下清晰可辨且不刺眼)
pub const COLOR_MIDNIGHT: Color = Color::from_rgb(
    48.0 / 255.0,
    68.0 / 255.0,
    108.0 / 255.0,
);

/// 线性插值颜色
#[inline]
pub fn lerp_color(c1: Color, c2: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color {
        r: c1.r + (c2.r - c1.r) * t,
        g: c1.g + (c2.g - c1.g) * t,
        b: c1.b + (c2.b - c1.b) * t,
        a: c1.a + (c2.a - c1.a) * t,
    }
}

/// 计算指定时间点（0.0 ~ 24.0 小时）在 4 点昼夜循环中的渐变颜色
pub fn color_at_time(time_hours: f32) -> Color {
    let t = time_hours.rem_euclid(24.0);
    if (6.0..12.0).contains(&t) {
        let u = (t - 6.0) / 6.0;
        lerp_color(COLOR_DAWN, COLOR_NOON, u)
    } else if (12.0..18.0).contains(&t) {
        let u = (t - 12.0) / 6.0;
        lerp_color(COLOR_NOON, COLOR_DUSK, u)
    } else if (18.0..24.0).contains(&t) {
        let u = (t - 18.0) / 6.0;
        lerp_color(COLOR_DUSK, COLOR_MIDNIGHT, u)
    } else {
        // 0.0 .. 6.0
        let u = t / 6.0;
        lerp_color(COLOR_MIDNIGHT, COLOR_DAWN, u)
    }
}

/// 将 24 小时时间（0.0 ~ 24.0）映射为圆环上的几何角度（弧度制）：
/// - 06:00 -> -PI/2 (最顶端)
/// - 12:00 -> 0     (正右端)
/// - 18:00 -> PI/2  (最底端)
/// - 24:00 -> PI    (正左端)
#[inline]
pub fn angle_for_time(time_hours: f32) -> f32 {
    let t = time_hours.rem_euclid(24.0);
    (t - 12.0) * (PI / 12.0)
}

/// 渲染图形时钟全屏图层
pub fn view<'a>(now: DateTime<Local>, theme_mode: ThemeMode) -> Element<'a, Message> {
    iced::widget::canvas(GraphicClockCanvas { now, theme_mode })
        .width(Fill)
        .height(Fill)
        .into()
}

pub struct GraphicClockCanvas {
    pub now: DateTime<Local>,
    pub theme_mode: ThemeMode,
}

impl<Message> canvas::Program<Message> for GraphicClockCanvas {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        // 计算圆心位置（预留后续动态时间位移算法）
        let center = self.compute_center(bounds.size());

        // 1. 绘制 24 小时环形平滑渐变色带（细分 96 段微弧无缝衔接）
        const SEGMENTS: usize = 96;
        let step = TAU / (SEGMENTS as f32);

        for i in 0..SEGMENTS {
            // 从 6:00 (-PI/2) 开始顺时针旋转
            let a1 = -FRAC_PI_2 + (i as f32) * step;
            // 微量 overlap (0.004 rad) 防止抗锯齿间隙缝隙
            let a2 = a1 + step + 0.004;

            // 该弧段中心对应的 24 小时时间点
            let time_h = (6.0 + (i as f32 + 0.5) * (24.0 / SEGMENTS as f32)).rem_euclid(24.0);
            let col = color_at_time(time_h);

            let arc_path = Path::new(|builder| {
                builder.arc(canvas::path::Arc {
                    center,
                    radius: MID_RADIUS,
                    start_angle: Radians(a1),
                    end_angle: Radians(a2),
                });
            });

            frame.stroke(
                &arc_path,
                Stroke {
                    style: canvas::Style::Solid(col),
                    width: RING_THICKNESS,
                    line_cap: LineCap::Butt,
                    line_join: LineJoin::Miter,
                    ..Default::default()
                },
            );
        }

        // 2. 内外双边缘辅助轮廓（提升表盘精致质感）
        let rim_color = if self.theme_mode.is_light() {
            Color::from_rgba8(0, 0, 0, 0.12)
        } else {
            Color::from_rgba8(255, 255, 255, 0.15)
        };
        let rim_stroke = Stroke {
            style: canvas::Style::Solid(rim_color),
            width: 1.0,
            ..Default::default()
        };

        let inner_rim = Path::circle(center, INNER_RADIUS);
        frame.stroke(&inner_rim, rim_stroke);

        let outer_rim = Path::circle(center, CLOCK_RADIUS);
        frame.stroke(&outer_rim, rim_stroke);

        // 3. 当前时间点指示游标（使用日程时间轴游标玫瑰红色 #F43F5E）
        let now_h = self.now.hour() as f32
            + (self.now.minute() as f32) / 60.0
            + (self.now.second() as f32) / 3600.0;
        let now_angle = angle_for_time(now_h);
        let cos_a = now_angle.cos();
        let sin_a = now_angle.sin();

        // 贯穿整个厚度的径向指示短线（内外各延伸 2px，增强视觉锚定感）
        let p_inner = Point::new(
            center.x + (INNER_RADIUS - 2.0) * cos_a,
            center.y + (INNER_RADIUS - 2.0) * sin_a,
        );
        let p_outer = Point::new(
            center.x + (CLOCK_RADIUS + 2.0) * cos_a,
            center.y + (CLOCK_RADIUS + 2.0) * sin_a,
        );
        let p_mid = Point::new(
            center.x + MID_RADIUS * cos_a,
            center.y + MID_RADIUS * sin_a,
        );

        let cursor_line = Path::line(p_inner, p_outer);
        frame.stroke(
            &cursor_line,
            Stroke {
                style: canvas::Style::Solid(CURSOR_COLOR),
                width: 3.5,
                line_cap: LineCap::Round,
                line_join: LineJoin::Round,
                ..Default::default()
            },
        );

        // 中心游标圆点（半径 5.5px，玫瑰红底带高对比白色微型核心）
        let cursor_dot = Path::circle(p_mid, 5.5);
        frame.fill(&cursor_dot, CURSOR_COLOR);

        let cursor_core = Path::circle(p_mid, 2.0);
        frame.fill(&cursor_core, Color::WHITE);

        vec![frame.into_geometry()]
    }
}

impl GraphicClockCanvas {
    /// 计算圆心位置（预留后续动态时间位移算法）
    fn compute_center(&self, size: iced::Size) -> Point {
        // 基准点：X=140.0，Y=屏幕高度中心
        // 外半径 200px 意味着 X 范围从 -60px 延伸至 340px，横跨全画幅
        Point::new(140.0, size.height / 2.0)
    }
}
