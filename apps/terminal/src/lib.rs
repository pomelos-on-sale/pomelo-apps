//! The terminal application for Pomelo OS running on WebAssembly.

extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use pomelo_sdk::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Message {
    Char(char),
    Backspace,
    Space,
    Enter,
    RunCommand(String),
    Clear,
    Exit,
}

pub struct TerminalApp {
    history: Vec<(String, String)>, // (prompt, output)
    current_input: String,
    prompt: String,
}

impl TerminalApp {
    pub fn new() -> Self {
        let initial_history = vec![
            (
                "".to_string(),
                "Welcome to Pomelo OS Terminal (WebAssembly)\nType 'help' for available commands.\n"
                    .to_string(),
            ),
        ];

        Self {
            history: initial_history,
            current_input: String::new(),
            prompt: "pomelo:~$ ".to_string(),
        }
    }

    fn execute_command(&mut self, cmd_line: &str) {
        let trimmed = cmd_line.trim();
        if trimmed.is_empty() {
            self.history.push((format!("{}{}", self.prompt, cmd_line), String::new()));
            return;
        }

        let mut parts = trimmed.split_whitespace();
        let cmd = parts.next().unwrap_or("");
        let args: Vec<&str> = parts.collect();

        let prompt_line = format!("{}{}", self.prompt, cmd_line);

        let output = match cmd {
            "help" => {
                "Available commands:\n  help        - Show this help\n  clear       - Clear screen\n  echo <text> - Print text\n  uname       - OS system information\n  time        - Display current time\n  free        - Memory status\n  launch <id> - Launch an application\n  exit        - Return to launcher\n".to_string()
            }
            "clear" => {
                self.history.clear();
                return;
            }
            "echo" => {
                args.join(" ")
            }
            "uname" => {
                "Pomelo OS 0.2.0 (Wasm32 interpreter / AOT)\nArchitecture: wasm32 / Xtensa LX7\n".to_string()
            }
            "time" => {
                let ms = pomelo_sdk::services::time_ms();
                format!("Current system time: {} ms\n", ms)
            }
            "free" => {
                "Total Flash: 16.0 MB\nWasm Heap: 64.0 KB linear static buffer\nStatus: Healthy\n".to_string()
            }
            "launch" => {
                if let Some(app_id) = args.first() {
                    log_info(format!("Terminal launching app: {app_id}"));
                    pomelo_sdk::services::launch_app(app_id);
                    format!("Launching app: {}\n", app_id)
                } else {
                    "Usage: launch <app_id>\n".to_string()
                }
            }
            "exit" | "back" => {
                pomelo_sdk::services::back();
                "Exiting terminal...\n".to_string()
            }
            unknown => {
                format!("Unknown command: {}\nType 'help' for command list.\n", unknown)
            }
        };

        self.history.push((prompt_line, output));
        if self.history.len() > 30 {
            self.history.remove(0);
        }
    }

    fn render_keyboard_row(&self, chars: &[char]) -> Element<Message> {
        let mut r = row!().spacing(6.0).align_y(Alignment::Center);

        for &c in chars {
            let label = text(c.to_string())
                .size(18.0)
                .color(Color::WHITE)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center);

            let btn = button(
                container(label)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(Alignment::Center)
                    .align_y(Alignment::Center),
            )
            .width(Length::FillPortion(1))
            .height(Length::Fixed(40.0))
            .padding(0.0)
            .background(Color::from_rgb8(45, 50, 65))
            .pressed_background(Color::from_rgb8(70, 78, 100))
            .border_radius(8.0)
            .on_press(Message::Char(c));

            r = r.push(btn);
        }

        r.into()
    }
}

impl Default for TerminalApp {
    fn default() -> Self {
        Self::new()
    }
}

impl Application for TerminalApp {
    type Message = Message;

    fn update(&mut self, message: Self::Message) {
        match message {
            Message::Char(c) => {
                self.current_input.push(c);
            }
            Message::Backspace => {
                self.current_input.pop();
            }
            Message::Space => {
                self.current_input.push(' ');
            }
            Message::Enter => {
                let cmd = core::mem::take(&mut self.current_input);
                self.execute_command(&cmd);
            }
            Message::RunCommand(cmd) => {
                self.execute_command(&cmd);
            }
            Message::Clear => {
                self.history.clear();
            }
            Message::Exit => {
                pomelo_sdk::services::back();
            }
        }
    }

    fn view(&self) -> Element<Self::Message> {
        // 1. Transcript list
        let mut transcript = col![].spacing(6.0);

        for (prompt_line, output) in &self.history {
            if !prompt_line.is_empty() {
                let p_txt = text(prompt_line)
                    .size(14.0)
                    .color(Color::from_rgb8(35, 209, 139));
                transcript = transcript.push(p_txt);
            }
            if !output.is_empty() {
                let o_txt = text(output)
                    .size(14.0)
                    .color(Color::from_rgb8(212, 212, 212));
                transcript = transcript.push(o_txt);
            }
        }

        // Active prompt line
        let input_prompt = text(&self.prompt)
            .size(14.0)
            .color(Color::from_rgb8(35, 209, 139));

        let current_text = text(format!("{}_", self.current_input))
            .size(14.0)
            .color(Color::WHITE);

        let active_line = row!()
            .spacing(4.0)
            .align_y(Alignment::Center)
            .push(input_prompt)
            .push(current_text);

        transcript = transcript.push(active_line);

        let transcript_area = container(transcript)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(12.0)
            .background(Color::from_rgb8(10, 10, 14));

        // Quick command chips
        let chip = |label: &'static str, cmd: &'static str| -> Element<Message> {
            button(
                container(
                    text(label)
                        .size(12.0)
                        .color(Color::from_rgb8(180, 200, 240))
                        .align_x(Alignment::Center)
                        .align_y(Alignment::Center),
                )
                .padding([4.0, 8.0])
                .align_x(Alignment::Center)
                .align_y(Alignment::Center),
            )
            .padding(0.0)
            .background(Color::from_rgb8(30, 36, 50))
            .pressed_background(Color::from_rgb8(50, 60, 80))
            .border_radius(6.0)
            .on_press(Message::RunCommand(cmd.to_string()))
            .into()
        };

        let chips_row = row!()
            .spacing(8.0)
            .padding([4.0, 8.0])
            .align_y(Alignment::Center)
            .push(chip("help", "help"))
            .push(chip("uname", "uname"))
            .push(chip("time", "time"))
            .push(chip("free", "free"))
            .push(chip("calc", "launch calculator"))
            .push(chip("counter", "launch counter"));

        // Virtual keyboard
        let row1_chars = ['q', 'w', 'e', 'r', 't', 'y', 'u', 'i', 'o', 'p'];
        let row2_chars = ['a', 's', 'd', 'f', 'g', 'h', 'j', 'k', 'l'];
        let row3_chars = ['z', 'x', 'c', 'v', 'b', 'n', 'm'];

        let r1 = self.render_keyboard_row(&row1_chars);
        let r2 = self.render_keyboard_row(&row2_chars);

        // Row 3 with Backspace
        let r3_keys = self.render_keyboard_row(&row3_chars);
        let bs_btn = button(
            container(text("⌫").size(16.0).color(Color::WHITE).align_x(Alignment::Center).align_y(Alignment::Center))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center),
        )
        .width(Length::Fixed(60.0))
        .height(Length::Fixed(40.0))
        .padding(0.0)
        .background(Color::from_rgb8(60, 40, 45))
        .pressed_background(Color::from_rgb8(80, 50, 55))
        .border_radius(8.0)
        .on_press(Message::Backspace);

        let r3 = row!()
            .spacing(6.0)
            .align_y(Alignment::Center)
            .push(r3_keys)
            .push(bs_btn);

        // Row 4: Exit, Space, Clear, Enter
        let exit_btn = button(
            container(text("Exit").size(14.0).color(Color::WHITE).align_x(Alignment::Center).align_y(Alignment::Center))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center),
        )
        .width(Length::Fixed(60.0))
        .height(Length::Fixed(40.0))
        .padding(0.0)
        .background(Color::from_rgb8(45, 50, 60))
        .border_radius(8.0)
        .on_press(Message::Exit);

        let space_btn = button(
            container(text("Space").size(14.0).color(Color::WHITE).align_x(Alignment::Center).align_y(Alignment::Center))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .height(Length::Fixed(40.0))
        .padding(0.0)
        .background(Color::from_rgb8(45, 50, 65))
        .pressed_background(Color::from_rgb8(70, 78, 100))
        .border_radius(8.0)
        .on_press(Message::Space);

        let enter_btn = button(
            container(text("↵ Enter").size(14.0).color(Color::WHITE).align_x(Alignment::Center).align_y(Alignment::Center))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center),
        )
        .width(Length::Fixed(80.0))
        .height(Length::Fixed(40.0))
        .padding(0.0)
        .background(Color::from_rgb8(37, 99, 235))
        .pressed_background(Color::from_rgb8(29, 78, 216))
        .border_radius(8.0)
        .on_press(Message::Enter);

        let r4 = row!()
            .spacing(6.0)
            .align_y(Alignment::Center)
            .push(exit_btn)
            .push(space_btn)
            .push(enter_btn);

        let keyboard = col![
            chips_row,
            r1,
            r2,
            r3,
            r4,
        ]
        .spacing(6.0)
        .padding(8.0);

        let keyboard_container = container(keyboard)
            .width(Length::Fill)
            .background(Color::from_rgb8(22, 24, 32));

        let page = col![
            transcript_area,
            keyboard_container,
        ]
        .width(Length::Fill)
        .height(Length::Fill);

        container(page)
            .width(Length::Fill)
            .height(Length::Fill)
            .background(Color::BLACK)
            .into()
    }
}

export_app!(TerminalApp);
