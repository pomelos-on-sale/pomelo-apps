//! The calculator application for Pomelo OS running on WebAssembly.

mod format;
mod keys;
mod model;
pub mod style;

use pomelo_sdk::prelude::*;
use serde::{Deserialize, Serialize};

pub use format::{add_commas, eval_op, format_raw_number};
pub use keys::{Entry, Key, Kind, LAYOUT};
pub use model::CalcModel;
pub use style::{ThemeMode, SCREEN};

/// What the calculator reacts to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Message {
    /// A key was pressed.
    Key(Key),
}

/// The calculator.
pub struct Calculator {
    model: CalcModel,
    theme_mode: ThemeMode,
}

impl Calculator {
    pub fn new() -> Self {
        Self {
            model: CalcModel::new(),
            theme_mode: ThemeMode::Dark,
        }
    }

    pub fn theme_mode(&self) -> ThemeMode {
        self.theme_mode
    }

    pub fn set_theme_mode(&mut self, theme: ThemeMode) {
        self.theme_mode = theme;
    }

    pub fn display(&self) -> String {
        self.model.primary_display()
    }

    pub fn expression(&self) -> String {
        self.model.secondary_display().to_string()
    }

    fn key(&self, entry: &'static Entry) -> Element<Message> {
        let palette = style::palette_for(entry.kind, self.theme_mode());

        let key_btn = button(
            container(
                text(entry.label)
                    .size(style::KEY_FONT)
                    .color(palette.text)
                    .align_x(Alignment::Center)
                    .align_y(Alignment::Center),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(0.0)
        .background(palette.fill)
        .pressed_background(palette.pressed)
        .border_radius(style::KEY_RADIUS)
        .on_press(Message::Key(entry.key));

        container(key_btn)
            .width(Length::FillPortion(entry.span))
            .height(Length::Fill)
            .padding([0.0, style::COLUMN_GAP / 2.0])
            .into()
    }

    fn display_card(&self) -> Element<Message> {
        let theme_mode = self.theme_mode();
        let primary = text(self.model.primary_display())
            .size(style::PRIMARY_FONT)
            .color(style::text_primary_for(theme_mode))
            .width(Length::Fill)
            .align_x(Alignment::End);

        let secondary = text(self.model.secondary_display())
            .size(style::SECONDARY_FONT)
            .color(style::text_secondary_for(theme_mode))
            .width(Length::Fill)
            .align_x(Alignment::End);

        container(col![secondary, primary].spacing(style::LINE_GAP))
            .padding([style::CARD_PAD_V, style::CARD_PAD_H])
            .width(Length::Fill)
            .height(Length::FillPortion(style::CARD_SHARE))
            .align_y(Alignment::Center)
            .background(style::card_for(theme_mode))
            .border_radius(style::CARD_RADIUS)
            .into()
    }

    fn keypad(&self) -> Element<Message> {
        let mut col_widget = col![];

        for (index, row_entries) in LAYOUT.iter().enumerate() {
            if index > 0 {
                col_widget = col_widget.push(gap(style::KEY_GAP_SHARE));
            }

            let mut row_w = row!().width(Length::Fill).height(Length::FillPortion(style::KEY_SHARE));
            for entry in row_entries.iter() {
                row_w = row_w.push(self.key(entry));
            }
            col_widget = col_widget.push(row_w);
        }

        col_widget
            .width(Length::Fill)
            .height(Length::FillPortion(style::KEYPAD_SHARE))
            .into()
    }
}

impl Default for Calculator {
    fn default() -> Self {
        Self::new()
    }
}

impl Application for Calculator {
    type Message = Message;

    fn update(&mut self, message: Self::Message) {
        match message {
            Message::Key(key) => key.apply(&mut self.model),
        }
    }

    fn view(&self) -> Element<Self::Message> {
        let page = col![
            gap(style::BAND_SHARE),
            self.display_card(),
            gap(style::CARD_GAP_SHARE),
            self.keypad(),
            gap(style::BAND_SHARE),
        ]
        .width(Length::Fill)
        .height(Length::Fill);

        container(page)
            .padding([0.0, style::PAGE_MARGIN])
            .width(Length::Fill)
            .height(Length::Fill)
            .background(style::background_for(self.theme_mode()))
            .into()
    }
}

fn gap(share: u16) -> Element<Message> {
    space().height(Length::FillPortion(share)).into()
}

export_app!(Calculator);
