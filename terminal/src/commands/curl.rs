use pomelo_hal::HalError;

use super::Command;
use crate::shell::TerminalModel;

/// Terminal command to execute HTTP/HTTPS requests asynchronously.
pub struct CurlCommand;

impl Command for CurlCommand {
    fn name(&self) -> &'static str {
        "curl"
    }

    fn description(&self) -> &'static str {
        "Transfer data from or to a server (curl <url>)"
    }

    fn execute(&self, state: &mut TerminalModel, args: &[&str]) {
        let url = match args.first() {
            Some(u) => *u,
            None => {
                state.push_output("curl: try 'curl <URL>'");
                return;
            }
        };

        if !url.starts_with("http://") && !url.starts_with("https://") {
            state.push_output("curl: (3) URL using bad/illegal format or missing URL");
            return;
        }

        let board = crate::commands::get_board();
        let res = board.http().start(url);
        match res {
            Ok(()) => {
                state.push_output(&format!("curl: connecting to {}...", url));
            }
            Err(HalError::ClockNotSynced) => {
                state.push_output("curl: (60) SSL certificate problem: system clock is not synchronized with NTP");
            }
            Err(HalError::Busy) => {
                state.push_output("curl: error: an active HTTP transfer is already in progress");
            }
            Err(e) => {
                state.push_output(&format!("curl: error initiating request: {}", e));
            }
        }
    }
}
