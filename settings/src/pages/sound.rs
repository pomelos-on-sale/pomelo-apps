//! The sound settings page: mute toggle and media volume slider.

use iced::widget::{container, slider};
use iced::{Color, Element, Length, Padding};
use pomelo_material_symbols::Icon;
use pomelo_widgets::{Item, ItemBadge, ItemGroup, Section, SystemPreferences};

use crate::i18n::{Key, LanguageExt as _};
use crate::pages::common::{back_bar, body, page, Page};
use crate::style;
use crate::Message;

/// The sound settings page.
pub(crate) fn sound_page<'a>(preferences: SystemPreferences) -> Page<'a> {
    let language = preferences.language;
    let theme = preferences.theme;
    let volume = preferences.volume;
    let is_muted = volume == 0;

    // 1. 静音 toggle 卡片
    let mute_badge = ItemBadge::new(
        if is_muted {
            Icon::VOLUME_OFF
        } else {
            Icon::VOLUME_UP
        },
        if is_muted {
            style::IconColor::Orange.color()
        } else {
            style::IconColor::Grey.color()
        },
    );

    let mute_item = Item::switch(
        language.text(Key::Mute),
        is_muted,
        Message::ToggleMute,
    )
    .with_badge(mute_badge);

    let mute_card: Element<'a, Message> = Section::group(
        ItemGroup::new().push(mute_item),
        theme,
    )
    .view();

    // 2. 音量调节 slider 卡片，上边带有“音量”文本
    let vol_slider = slider(0..=100, volume, Message::SetVolume)
        .step(1)
        .width(Length::Fill)
        .style(move |_t, _status| {
            slider::Style {
                rail: slider::Rail {
                    backgrounds: (
                        theme.accent().into(),
                        Color::from_rgba(1.0, 1.0, 1.0, 0.12).into(),
                    ),
                    width: 10.0,
                    border: iced::Border {
                        radius: 5.0.into(),
                        width: 0.0,
                        color: Color::TRANSPARENT,
                    },
                },
                handle: slider::Handle {
                    shape: slider::HandleShape::Circle { radius: 14.0 },
                    background: Color::WHITE.into(),
                    border_width: 2.0,
                    border_color: theme.accent(),
                },
            }
        });

    let slider_container = container(vol_slider)
        .padding(Padding {
            top: 16.0,
            bottom: 16.0,
            left: 16.0,
            right: 16.0,
        })
        .width(Length::Fill);

    let slider_card: Element<'a, Message> = Section::new(slider_container, theme)
        .header(language.text(Key::Volume))
        .view();

    page(
        back_bar(theme),
        body(vec![mute_card, slider_card], theme),
    )
}
