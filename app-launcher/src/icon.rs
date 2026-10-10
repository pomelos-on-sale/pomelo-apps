//! App icon button styles and bitmap icon rendering for app launcher.

use iced::widget::{button, container};
use iced::{Border, Color, Element, Length, Shadow};
#[cfg(not(feature = "desktop"))]
use iced::Size;

use crate::apps::Entry;
use crate::style;
use pomelo_widgets::BitmapIcon;

/// An app icon button: accent color at rest, subtly lit when pressed.
/// For bitmap icons, transparent at rest with a subtle translucent highlight when pressed.
pub(crate) fn icon_style(entry: &'static Entry, status: button::Status) -> button::Style {
    if entry.icon.is_bitmap() {
        let bg = match status {
            button::Status::Pressed => Some(Color::from_rgba(1.0, 1.0, 1.0, 0.15).into()),
            _ => None,
        };
        return button::Style {
            background: bg,
            text_color: Color::WHITE,
            border: Border {
                radius: style::ICON_RADIUS.into(),
                ..Border::default()
            },
            shadow: Shadow::default(),
            snap: false,
        };
    }

    let base_color = entry.color();
    let bg = match status {
        button::Status::Pressed => {
            let (r, g, b) = entry.accent;
            Color::from_rgb(
                ((r as f32 * 1.35).min(255.0)) / 255.0,
                ((g as f32 * 1.35).min(255.0)) / 255.0,
                ((b as f32 * 1.35).min(255.0)) / 255.0,
            )
        }
        _ => base_color,
    };

    button::Style {
        background: Some(bg.into()),
        text_color: Color::WHITE,
        border: Border {
            radius: style::ICON_RADIUS.into(),
            ..Border::default()
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

#[cfg(feature = "desktop")]
pub(crate) fn render_bitmap_icon<'a, Message: 'a>(icon: BitmapIcon) -> Element<'a, Message> {
    let pixel_count = (icon.width as usize) * (icon.height as usize);
    let mut rgba = Vec::with_capacity(pixel_count * 4);
    for i in 0..pixel_count {
        let p565 = icon.rgb565[i];
        let r5 = (p565 >> 11) & 0x1F;
        let g6 = (p565 >> 5) & 0x3F;
        let b5 = p565 & 0x1F;

        let r = ((r5 as u32 * 255 + 15) / 31) as u8;
        let g = ((g6 as u32 * 255 + 31) / 63) as u8;
        let b = ((b5 as u32 * 255 + 15) / 31) as u8;
        let a = icon.alpha[i];

        rgba.extend_from_slice(&[r, g, b, a]);
    }
    let handle = iced::widget::image::Handle::from_rgba(
        icon.width as u32,
        icon.height as u32,
        rgba,
    );
    container(
        iced::widget::image(handle)
            .width(Length::Fixed(style::ICON))
            .height(Length::Fixed(style::ICON)),
    )
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .into()
}

#[cfg(feature = "desktop")]
pub(crate) fn render_qoi_icon<'a, Message: 'a>(qoi_bytes: &'static [u8]) -> Element<'a, Message> {
    use std::sync::OnceLock;
    static ICON_HANDLE: OnceLock<iced::widget::image::Handle> = OnceLock::new();
    let handle = ICON_HANDLE.get_or_init(|| {
        let (header, decoded) = qoi::decode_to_vec(qoi_bytes).expect("decode QOI icon");
        let rgba = match header.channels {
            qoi::Channels::Rgb => {
                let mut rgba = Vec::with_capacity((header.width * header.height * 4) as usize);
                for chunk in decoded.chunks_exact(3) {
                    rgba.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
                }
                rgba
            }
            qoi::Channels::Rgba => decoded,
        };
        iced::widget::image::Handle::from_rgba(header.width, header.height, rgba)
    });
    container(
        iced::widget::image(handle.clone())
            .width(Length::Fixed(style::ICON))
            .height(Length::Fixed(style::ICON)),
    )
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .into()
}

#[cfg(feature = "desktop")]
pub(crate) fn render_qoi_wallpaper<'a, Message: 'a>(qoi_bytes: &'static [u8]) -> Element<'a, Message> {
    use std::sync::OnceLock;
    static WALLPAPER_HANDLE: OnceLock<iced::widget::image::Handle> = OnceLock::new();
    let handle = WALLPAPER_HANDLE.get_or_init(|| {
        let (header, decoded) = qoi::decode_to_vec(qoi_bytes).expect("decode QOI wallpaper");
        let rgba = match header.channels {
            qoi::Channels::Rgb => {
                let mut rgba = Vec::with_capacity((header.width * header.height * 4) as usize);
                for chunk in decoded.chunks_exact(3) {
                    rgba.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
                }
                rgba
            }
            qoi::Channels::Rgba => decoded,
        };
        iced::widget::image::Handle::from_rgba(header.width, header.height, rgba)
    });
    iced::widget::image(handle.clone())
        .width(Length::Fill)
        .height(Length::Fill)
        .content_fit(iced::ContentFit::Fill)
        .into()
}

#[cfg(not(feature = "desktop"))]
struct BitmapIconWidget {
    icon: BitmapIcon,
    size: f32,
}

#[cfg(not(feature = "desktop"))]
impl<Message, Theme> iced::advanced::widget::Widget<Message, Theme, iced::Renderer>
    for BitmapIconWidget
{
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fixed(self.size), Length::Fixed(self.size))
    }

    fn layout(
        &mut self,
        _tree: &mut iced::advanced::widget::Tree,
        _renderer: &iced::Renderer,
        limits: &iced::advanced::layout::Limits,
    ) -> iced::advanced::layout::Node {
        iced::advanced::layout::Node::new(limits.resolve(
            Length::Fixed(self.size),
            Length::Fixed(self.size),
            Size::new(self.size, self.size),
        ))
    }

    fn draw(
        &self,
        _tree: &iced::advanced::widget::Tree,
        renderer: &mut iced::Renderer,
        _theme: &Theme,
        _style: &iced::advanced::renderer::Style,
        layout: iced::advanced::Layout<'_>,
        _cursor: iced::advanced::mouse::Cursor,
        _viewport: &iced::Rectangle,
    ) {
        renderer.draw_bitmap_565(
            layout.bounds(),
            self.icon.width,
            self.icon.height,
            self.icon.rgb565,
            self.icon.alpha,
        );
    }
}

#[cfg(not(feature = "desktop"))]
pub(crate) fn render_bitmap_icon<'a, Message: 'a>(icon: BitmapIcon) -> Element<'a, Message> {
    container(
        Element::new(BitmapIconWidget {
            icon,
            size: style::ICON,
        })
    )
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .into()
}

#[cfg(not(feature = "desktop"))]
const DITHER_OFFSETS: [f32; 64] = [
    0.015625, 0.515625, 0.140625, 0.640625, 0.046875, 0.546875, 0.171875, 0.671875,
    0.765625, 0.265625, 0.890625, 0.390625, 0.796875, 0.296875, 0.921875, 0.421875,
    0.203125, 0.703125, 0.078125, 0.578125, 0.234375, 0.734375, 0.109375, 0.609375,
    0.953125, 0.453125, 0.828125, 0.328125, 0.984375, 0.484375, 0.859375, 0.359375,
    0.0625,   0.5625,   0.1875,   0.6875,   0.03125,  0.53125,  0.15625,  0.65625,
    0.8125,   0.3125,   0.9375,   0.4375,   0.78125,  0.28125,  0.90625,  0.40625,
    0.25,     0.75,     0.125,    0.625,    0.21875,  0.71875,  0.09375,  0.59375,
    1.0,      0.5,      0.875,    0.375,    0.96875,  0.46875,  0.84375,  0.34375,
];

#[cfg(not(feature = "desktop"))]
#[inline(always)]
fn dither_rgb888_to_rgb565(r: u8, g: u8, b: u8, x: i32, y: i32) -> u16 {
    let idx_r = (((y & 7) << 3) | (x & 7)) as usize;
    let idx_g = ((((y + 4) & 7) << 3) | ((x + 2) & 7)) as usize;
    let idx_b = ((((y + 2) & 7) << 3) | ((x + 4) & 7)) as usize;

    let d_r = DITHER_OFFSETS[idx_r];
    let d_g = DITHER_OFFSETS[idx_g];
    let d_b = DITHER_OFFSETS[idx_b];

    let r5 = ((r as f32 * 0.125 + d_r) as u32).min(31) as u16;
    let g6 = ((g as f32 * 0.250 + d_g) as u32).min(63) as u16;
    let b5 = ((b as f32 * 0.125 + d_b) as u32).min(31) as u16;

    (r5 << 11) | (g6 << 5) | b5
}

#[cfg(not(feature = "desktop"))]
fn get_qoi_icon(data: &'static [u8]) -> BitmapIcon {
    use std::sync::OnceLock;
    static ICON_CACHE: OnceLock<BitmapIcon> = OnceLock::new();
    *ICON_CACHE.get_or_init(|| {
        let (header, decoded) = qoi::decode_to_vec(data).expect("decode QOI icon");
        let w = header.width;
        let h = header.height;
        let count = (w * h) as usize;
        let mut rgb565 = Vec::with_capacity(count);
        let mut alpha = Vec::with_capacity(count);
        let channels = header.channels.as_u8() as usize;
        for i in 0..count {
            let idx = i * channels;
            let r = decoded[idx];
            let g = decoded[idx + 1];
            let b = decoded[idx + 2];
            let a = if channels >= 4 { decoded[idx + 3] } else { 255 };
            let px = (i % w as usize) as i32;
            let py = (i / w as usize) as i32;
            rgb565.push(dither_rgb888_to_rgb565(r, g, b, px, py));
            alpha.push(a);
        }
        let static_rgb565: &'static [u16] = Box::leak(rgb565.into_boxed_slice());
        let static_alpha: &'static [u8] = Box::leak(alpha.into_boxed_slice());
        BitmapIcon {
            width: w as u16,
            height: h as u16,
            rgb565: static_rgb565,
            alpha: static_alpha,
        }
    })
}

#[cfg(not(feature = "desktop"))]
pub(crate) fn render_qoi_icon<'a, Message: 'a>(data: &'static [u8]) -> Element<'a, Message> {
    let icon = get_qoi_icon(data);
    render_bitmap_icon(icon)
}

#[cfg(not(feature = "desktop"))]
struct QoiWallpaperWidget {
    width: u16,
    height: u16,
    pixels: &'static [u16],
}

#[cfg(not(feature = "desktop"))]
impl<Message, Theme> iced::advanced::widget::Widget<Message, Theme, iced::Renderer>
    for QoiWallpaperWidget
{
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(
        &mut self,
        _tree: &mut iced::advanced::widget::Tree,
        _renderer: &iced::Renderer,
        limits: &iced::advanced::layout::Limits,
    ) -> iced::advanced::layout::Node {
        iced::advanced::layout::Node::new(limits.resolve(
            Length::Fill,
            Length::Fill,
            Size::new(self.width as f32, self.height as f32),
        ))
    }

    fn draw(
        &self,
        _tree: &iced::advanced::widget::Tree,
        renderer: &mut iced::Renderer,
        _theme: &Theme,
        _style: &iced::advanced::renderer::Style,
        layout: iced::advanced::Layout<'_>,
        _cursor: iced::advanced::mouse::Cursor,
        _viewport: &iced::Rectangle,
    ) {
        renderer.draw_image_565_opaque(layout.bounds(), self.width, self.height, self.pixels);
    }
}

#[cfg(not(feature = "desktop"))]
fn get_wallpaper(qoi_bytes: &'static [u8]) -> (u16, u16, &'static [u16]) {
    use std::sync::OnceLock;
    static WALLPAPER_CACHE: OnceLock<(u16, u16, &'static [u16])> = OnceLock::new();
    *WALLPAPER_CACHE.get_or_init(|| {
        let t0 = std::time::Instant::now();
        let (header, decoded) = qoi::decode_to_vec(qoi_bytes).expect("decode QOI wallpaper");
        let w = header.width;
        let h = header.height;
        let count = (w * h) as usize;
        let mut rgb565 = Vec::with_capacity(count);
        let channels = header.channels.as_u8() as usize;
        for i in 0..count {
            let idx = i * channels;
            let r = decoded[idx];
            let g = decoded[idx + 1];
            let b = decoded[idx + 2];
            let px = (i % w as usize) as i32;
            let py = (i / w as usize) as i32;
            rgb565.push(dither_rgb888_to_rgb565(r, g, b, px, py));
        }
        let static_pixels: &'static [u16] = Box::leak(rgb565.into_boxed_slice());
        println!(
            "[wallpaper] pre-decoded {}x{} QOI wallpaper to RGB565 in {:?}",
            w,
            h,
            t0.elapsed()
        );
        (w as u16, h as u16, static_pixels)
    })
}

#[cfg(not(feature = "desktop"))]
pub(crate) fn render_qoi_wallpaper<'a, Message: 'a>(data: &'static [u8]) -> Element<'a, Message> {
    let (width, height, pixels) = get_wallpaper(data);
    Element::new(QoiWallpaperWidget {
        width,
        height,
        pixels,
    })
}
