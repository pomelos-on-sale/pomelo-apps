//! Milky Way wallpaper data storage and rendering for app launcher.

use iced::{Element, Length};
#[cfg(not(feature = "desktop"))]
use iced::Size;

/// 2-byte aligned raw binary wrapper for direct Flash XIP (execute-in-place) zero-copy access.
#[repr(C, align(2))]
struct Aligned<const N: usize>([u8; N]);

static WALLPAPER_RAW: Aligned<412800> =
    Aligned(*include_bytes!(concat!(env!("OUT_DIR"), "/wallpaper_milkyway.bin")));

/// Pre-processed 480x430 RGB565 wallpaper pixels (480 * 430 = 206,400 u16 pixels).
///
/// On embedded target, cached into high-speed Octal PSRAM at boot (one-time ~10ms memcpy).
/// Reading from Octal PSRAM (8-bit bus @ 80MHz) during frame rendering avoids
/// continuous Flash cache misses over the Quad SPI bus (4-bit bus), restoring frame
/// render time to the 16ms target.
pub fn wallpaper_pixels() -> &'static [u16] {
    #[cfg(not(feature = "desktop"))]
    {
        use std::sync::OnceLock;
        static PSRAM_WALLPAPER: OnceLock<&'static [u16]> = OnceLock::new();
        *PSRAM_WALLPAPER.get_or_init(|| {
            let t0 = std::time::Instant::now();
            let raw_flash = unsafe {
                std::slice::from_raw_parts(
                    WALLPAPER_RAW.0.as_ptr() as *const u16,
                    480 * 430,
                )
            };
            let boxed: Box<[u16]> = raw_flash.to_vec().into_boxed_slice();
            let ptr = Box::leak(boxed);
            println!(
                "[wallpaper] cached 480x430 binary RGB565 into PSRAM in {:?}",
                t0.elapsed()
            );
            ptr
        })
    }
    #[cfg(feature = "desktop")]
    {
        unsafe {
            std::slice::from_raw_parts(
                WALLPAPER_RAW.0.as_ptr() as *const u16,
                480 * 430,
            )
        }
    }
}

#[cfg(feature = "desktop")]
pub(crate) fn render_wallpaper<'a, Message: 'a>() -> Element<'a, Message> {
    use std::sync::OnceLock;
    static WALLPAPER_HANDLE: OnceLock<iced::widget::image::Handle> = OnceLock::new();
    let handle = WALLPAPER_HANDLE.get_or_init(|| {
        let pixels = wallpaper_pixels();
        let target_w = 480u32;
        let target_h = 430u32;
        let mut rgba = Vec::with_capacity((target_w * target_h * 4) as usize);
        for &p565 in pixels {
            let r5 = (p565 >> 11) & 0x1F;
            let g6 = (p565 >> 5) & 0x3F;
            let b5 = p565 & 0x1F;

            let r = ((r5 as u32 * 255 + 15) / 31) as u8;
            let g = ((g6 as u32 * 255 + 31) / 63) as u8;
            let b = ((b5 as u32 * 255 + 15) / 31) as u8;
            rgba.extend_from_slice(&[r, g, b, 255]);
        }
        iced::widget::image::Handle::from_rgba(target_w, target_h, rgba)
    });
    iced::widget::image(handle.clone())
        .width(Length::Fill)
        .height(Length::Fill)
        .content_fit(iced::ContentFit::Fill)
        .into()
}

#[cfg(not(feature = "desktop"))]
struct WallpaperWidget {
    width: u16,
    height: u16,
    pixels: &'static [u16],
}

#[cfg(not(feature = "desktop"))]
impl<Message, Theme> iced::advanced::widget::Widget<Message, Theme, iced::Renderer>
    for WallpaperWidget
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
pub(crate) fn render_wallpaper<'a, Message: 'a>() -> Element<'a, Message> {
    Element::new(WallpaperWidget {
        width: 480,
        height: 430,
        pixels: wallpaper_pixels(),
    })
}
