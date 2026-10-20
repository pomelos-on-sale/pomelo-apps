//! Launcher UI view rendering, paged grid, and hosted application screens.

use iced::widget::{button, column, container, stack, text, Column, Row, Space};
use iced::{Alignment, Border, Color, Element, Length};
use pomelo_material_symbols::Icon;

use crate::apps::{entry_of, AppIcon, AppId, CATALOGUE};
use crate::icon::{icon_style, render_bitmap_icon};
use crate::status;
use crate::style;
use crate::wallpaper::render_wallpaper;
use super::{Launcher, Message, Screen};

macro_rules! render_hosted_app {
    ($app:expr, $msg:path) => {
        if let Some(app) = &$app {
            app.view().map($msg)
        } else {
            Space::new().into()
        }
    };
}

impl Launcher {
    /// Describes the interface for the current state.
    pub fn view(&self) -> Element<'_, Message> {
        match self.screen {
            Screen::Grid => self.launcher(),
            Screen::App(AppId::Terminal) => render_hosted_app!(self.terminal, Message::Terminal),
            Screen::App(AppId::Calculator) => render_hosted_app!(self.calculator, Message::Calculator),
            Screen::App(AppId::DemoCounter) => render_hosted_app!(self.counter, Message::Counter),
            Screen::App(AppId::DemoHello) => render_hosted_app!(self.hello, Message::Hello),
            Screen::App(AppId::Settings) => render_hosted_app!(self.settings, Message::Settings),
            Screen::App(AppId::Music) => render_hosted_app!(self.music, Message::Music),
            Screen::App(AppId::DailySchedule) => {
                render_hosted_app!(self.daily_schedule, Message::DailySchedule)
            }
            Screen::App(AppId::WirelessClipboard) => {
                render_hosted_app!(self.wireless_clipboard, Message::WirelessClipboard)
            }
            Screen::App(AppId::Alarm) => render_hosted_app!(self.alarm, Message::Alarm),
        }
    }

    fn launcher(&self) -> Element<'_, Message> {
        let pages: Vec<Element<'_, Message>> =
            (0..self.pages()).map(|p| self.page(p)).collect();

        let paged_grid = pomelo_widgets::pager(pages)
            .current_page(self.page)
            .swipe_commit(style::SWIPE_COMMIT)
            .touch_slop(style::SLOP)
            .on_change(Message::PageChanged);

        let content = column![
            paged_grid,
            self.dots(),
            Space::new().height(Length::Fixed(style::GUTTER)),
        ]
        .height(Length::Fill);

        let wallpaper = render_wallpaper();
        let body = stack![wallpaper, content]
            .width(Length::Fill)
            .height(Length::Fill);

        let screen = column![
            self.status_bar(),
            body,
        ]
        .height(Length::Fill);

        container(screen).width(Length::Fill).height(Length::Fill).into()
    }

    /// One page of the grid: up to [`style::PER_PAGE`] tiles, evenly distributed in rows and columns.
    ///
    /// Both the columns (horizontal) and rows (vertical) are evenly spaced with flexible spaces
    /// (`space-evenly`), ensuring identical gaps between adjacent icons and towards the screen boundaries.
    pub(crate) fn page(&self, page: usize) -> Element<'_, Message> {
        let first = page * style::PER_PAGE;

        let mut page_column = Column::new()
            .width(Length::Fill)
            .height(Length::Fill);

        for row in 0..style::ROWS {
            page_column = page_column.push(Space::new().height(Length::Fill));

            let mut row_widget = Row::new()
                .width(Length::Fill)
                .align_y(Alignment::Center);

            for col in 0..style::COLUMNS {
                let index = first + row * style::COLUMNS + col;
                let tile: Element<'_, Message> = match CATALOGUE.get(index) {
                    Some(_) => self.tile(index),
                    None => self.placeholder_tile(),
                };

                row_widget = row_widget
                    .push(Space::new().width(Length::Fill))
                    .push(tile);
            }

            row_widget = row_widget.push(Space::new().width(Length::Fill));
            page_column = page_column.push(row_widget);
        }

        page_column = page_column.push(Space::new().height(Length::Fill));
        page_column.into()
    }

    /// One dot per page, the one that is up lit.
    fn dots(&self) -> Element<'_, Message> {
        let up = self.page;
        let is_light = self.preferences.theme.is_light();

        let dots = (0..self.pages()).map(|page| {
            let colour = if is_light {
                if page == up {
                    (31, 35, 40)
                } else {
                    (209, 213, 219)
                }
            } else {
                if page == up {
                    style::DOT_UP
                } else {
                    style::DOT_REST
                }
            };

            container(Space::new())
                .width(Length::Fixed(style::DOT))
                .height(Length::Fixed(style::DOT))
                .style(move |_theme| container::Style {
                    background: Some(Color::from_rgb8(colour.0, colour.1, colour.2).into()),
                    border: Border {
                        radius: (style::DOT / 2.0).into(),
                        ..Border::default()
                    },
                    ..container::Style::default()
                })
                .into()
        });

        container(Row::with_children(dots).spacing(style::DOT_GAP))
            .center_x(Length::Fill)
            .into()
    }

    /// How many pages the catalogue makes.
    pub(crate) fn pages(&self) -> usize {
        CATALOGUE.len().div_ceil(style::PER_PAGE)
    }


    /// One app tile: an icon button and label, sized strictly to [`style::TILE_WIDTH`] width.
    ///
    /// Only the app's application icon is tappable. Surrounding spaces and margins allow swipe
    /// gestures to pass through cleanly to [`pomelo_widgets::pager`].
    fn tile(&self, index: usize) -> Element<'_, Message> {
        let entry = &CATALOGUE[index];

        let icon_content: Element<'_, Message> = match entry.icon {
            AppIcon::Glyph(glyph) => container(
                text(glyph.glyph())
                    .size(style::GLYPH)
                    .font(pomelo_material_symbols::font()),
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into(),
            AppIcon::Bitmap(bitmap) => render_bitmap_icon(bitmap),
            AppIcon::Qoi(_) => iced::widget::Space::new().into(),
        };

        let icon_button = button(icon_content)
            .width(Length::Fixed(style::ICON))
            .height(Length::Fixed(style::ICON))
            .padding(0)
            .on_press(Message::Open(entry.id))
            .style(move |_theme, status| icon_style(entry, status));

        let label_raw = entry.localized_name(self.preferences.language);
        let label_display = style::truncate_label(label_raw, style::LABEL_MAX_WIDTH, style::LABEL);
        let label_size = style::LABEL;
        let label_color = if self.preferences.theme.is_light() {
            Color::from_rgb8(17, 24, 39)
        } else {
            Color::WHITE
        };

        let contents = column![
            icon_button,
            text(label_display)
                .size(label_size)
                .color(label_color)
                .wrapping(text::Wrapping::None),
        ]
        .spacing(style::GLYPH_GAP)
        .align_x(Alignment::Center);

        container(contents)
            .width(Length::Fixed(style::TILE_WIDTH))
            .center_x(Length::Fixed(style::TILE_WIDTH))
            .into()
    }

    /// An empty placeholder tile maintaining the exact geometry as [`tile`].
    fn placeholder_tile(&self) -> Element<'_, Message> {
        let label_size = style::LABEL;
        let placeholder = column![
            Space::new()
                .width(Length::Fixed(style::ICON))
                .height(Length::Fixed(style::ICON)),
            text(" ")
                .size(label_size)
                .color(Color::TRANSPARENT)
                .wrapping(text::Wrapping::None),
        ]
        .spacing(style::GLYPH_GAP)
        .align_x(Alignment::Center);

        container(placeholder)
            .width(Length::Fixed(style::TILE_WIDTH))
            .center_x(Length::Fixed(style::TILE_WIDTH))
            .into()
    }

    /// The clock, the signal and the battery.
    ///
    /// The three readings are the platform's to push, so the bar is built from what was pushed and
    /// not read from the board here: a `Program` cannot hold a subscription to a timer, and the
    /// board is the platform's half of the pair. What each reading looks like -- and why the signal
    /// has four bars while the icon set has three -- is `status`'s to say.
    fn status_bar(&self) -> Element<'_, Message> {
        let bg_icons: Vec<Icon> = self
            .running_apps
            .iter()
            .filter(|&&id| match self.screen {
                Screen::App(current) => current != id,
                Screen::Grid => true,
            })
            .map(|&id| {
                entry_of(id).icon.as_glyph().unwrap_or(Icon::APPS)
            })
            .collect();

        status::view(
            &self.clock,
            self.battery,
            self.charging,
            self.wifi,
            self.board.wifi().is_enabled(),
            &bg_icons,
            self.preferences.theme,
        )
    }
}
