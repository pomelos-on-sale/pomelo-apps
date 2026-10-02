use pomelo_sdk::col;
use pomelo_sdk::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Message {
    Tap,
    Reset,
}

#[derive(Default)]
pub struct CounterApp {
    count: u32,
}

impl Application for CounterApp {
    type Message = Message;

    fn update(&mut self, message: Self::Message) {
        match message {
            Message::Tap => {
                self.count += 1;
                log_info(format!("Counter tapped! Count is now {}", self.count));
            }
            Message::Reset => {
                self.count = 0;
                log_info("Counter reset!");
            }
        }
    }

    fn view(&self) -> Element<Self::Message> {
        let is_light = false;
        let card_bg = if is_light {
            Color::WHITE
        } else {
            Color::from_rgb8(17, 24, 39)
        };
        let text_color = if is_light {
            Color::from_rgb8(17, 24, 39)
        } else {
            Color::WHITE
        };
        let sub_text = if is_light {
            Color::from_rgb8(107, 114, 128)
        } else {
            Color::from_rgb8(156, 163, 175)
        };

        let count_str = if self.count < 10 {
            format!("0{}", self.count)
        } else {
            format!("{}", self.count)
        };

        container(
            col![
                // Header
                container(
                    text("TOUCH COUNTER")
                        .size(16.0_f32)
                        .color(sub_text)
                )
                .center_x(Length::Fill),

                // Big Counter Number
                container(
                    text(count_str)
                        .size(96.0_f32)
                        .color(text_color)
                )
                .center_x(Length::Fill)
                .center_y(Length::Fill),

                // Button row
                row![
                    button(
                        container(text("+ 1 TAP").size(18.0_f32).color(Color::WHITE))
                            .center_x(Length::Fill)
                            .center_y(Length::Fill)
                    )
                    .width(Length::Fixed(180.0_f32))
                    .height(Length::Fixed(56.0_f32))
                    .background(Color::from_rgb8(59, 130, 246))
                    .pressed_background(Color::from_rgb8(37, 99, 235))
                    .border_radius(16.0_f32)
                    .on_press(Message::Tap),

                    button(
                        container(text("RESET").size(16.0_f32).color(Color::WHITE))
                            .center_x(Length::Fill)
                            .center_y(Length::Fill)
                    )
                    .width(Length::Fixed(100.0_f32))
                    .height(Length::Fixed(56.0_f32))
                    .background(Color::from_rgb8(75, 85, 99))
                    .pressed_background(Color::from_rgb8(55, 65, 81))
                    .border_radius(16.0_f32)
                    .on_press(Message::Reset),
                ]
                .spacing(12.0_f32)
                .align_y(Alignment::Center),
            ]
            .spacing(20.0_f32)
            .padding(24.0_f32)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Center)
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .background(card_bg)
        .into()
    }
}

export_app!(CounterApp);
