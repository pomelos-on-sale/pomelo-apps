pub mod cat;
pub mod cd;
pub mod clear;
pub mod curl;
pub mod echo;
pub mod free;
pub mod help;
pub mod ls;
pub mod pwd;
pub mod rm;
pub mod wifi;

use std::sync::{Arc, OnceLock, RwLock};
use pomelo_hal::Board;

use crate::shell::TerminalModel;

static BOARD_INSTANCE: OnceLock<RwLock<Arc<Board>>> = OnceLock::new();

/// Set the shared Board instance for hardware-backed terminal commands.
pub fn set_board(board: Arc<Board>) {
    if let Some(lock) = BOARD_INSTANCE.get() {
        let mut guard = lock.write().unwrap_or_else(|e| e.into_inner());
        *guard = board;
    } else {
        let _ = BOARD_INSTANCE.set(RwLock::new(board));
    }
}

/// Get the shared Board instance, defaulting to desktop simulator if not set.
pub fn get_board() -> Arc<Board> {
    #[cfg(not(target_os = "espidf"))]
    {
        let lock = BOARD_INSTANCE.get_or_init(|| RwLock::new(Arc::new(Board::simulated())));
        let guard = lock.read().unwrap_or_else(|e| e.into_inner());
        Arc::clone(&guard)
    }
    #[cfg(target_os = "espidf")]
    {
        let lock = BOARD_INSTANCE
            .get()
            .expect("Board instance not initialized for terminal on ESP-IDF");
        let guard = lock.read().unwrap_or_else(|e| e.into_inner());
        Arc::clone(&guard)
    }
}

pub trait Command: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn execute(&self, state: &mut TerminalModel, args: &[&str]);
}

pub struct CommandRegistry {
    commands: Vec<Box<dyn Command>>,
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandRegistry {
    pub fn new() -> Self {
        let mut reg = Self {
            commands: Vec::new(),
        };
        reg.register(help::HelpCommand);
        reg.register(ls::LsCommand);
        reg.register(cd::CdCommand);
        reg.register(pwd::PwdCommand);
        reg.register(cat::CatCommand);
        reg.register(rm::RmCommand);
        reg.register(free::FreeCommand);
        reg.register(echo::EchoCommand);
        reg.register(clear::ClearCommand);
        reg.register(wifi::WifiCommand);
        reg.register(curl::CurlCommand);
        reg
    }

    pub fn register(&mut self, cmd: impl Command + 'static) {
        self.commands.push(Box::new(cmd));
    }

    pub fn list(&self) -> &[Box<dyn Command>] {
        &self.commands
    }

    pub fn dispatch(&self, state: &mut TerminalModel, input_line: &str) {
        let parts: Vec<&str> = input_line.split_whitespace().collect();
        if parts.is_empty() {
            return;
        }

        let cmd_name = parts[0];
        let args = &parts[1..];

        for cmd in &self.commands {
            if cmd.name() == cmd_name {
                cmd.execute(state, args);
                return;
            }
        }

        state.push_output(&format!(
            "{}: command not found. Type 'help' for available commands.",
            cmd_name
        ));
    }
}

/// Global shared registry instance
pub fn get_registry() -> &'static CommandRegistry {
    static REGISTRY: std::sync::OnceLock<CommandRegistry> = std::sync::OnceLock::new();
    REGISTRY.get_or_init(CommandRegistry::new)
}
