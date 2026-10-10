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
    let handle = iced::widget::image::Handle::from_rgba(
        header.width,
        header.height,
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
struct QoiIconWidget {
    data: &'static [u8],
    size: f32,
}

#[cfg(not(feature = "desktop"))]
impl<Message, Theme> iced::advanced::widget::Widget<Message, Theme, iced::Renderer>
    for QoiIconWidget
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
        renderer.draw_qoi(layout.bounds(), self.data);
    }
}

#[cfg(not(feature = "desktop"))]
pub(crate) fn render_qoi_icon<'a, Message: 'a>(data: &'static [u8]) -> Element<'a, Message> {
    container(
        Element::new(QoiIconWidget {
            data,
            size: style::ICON,
        })
    )
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .into()
}

#[cfg(not(feature = "desktop"))]
struct QoiWallpaperWidget {
    data: &'static [u8],
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
            Size::new(480.0, 430.0),
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
        renderer.draw_qoi(layout.bounds(), self.data);
    }
}

#[cfg(not(feature = "desktop"))]
pub(crate) fn render_qoi_wallpaper<'a, Message: 'a>(data: &'static [u8]) -> Element<'a, Message> {
    Element::new(QoiWallpaperWidget { data })
}
