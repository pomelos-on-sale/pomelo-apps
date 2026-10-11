use super::Command;
use crate::shell::TerminalModel;

pub struct FreeCommand;

impl Command for FreeCommand {
    fn name(&self) -> &'static str {
        "free"
    }

    fn description(&self) -> &'static str {
        "Show memory and storage (LittleFS & MicroSD) status"
    }

    fn execute(&self, state: &mut TerminalModel, _args: &[&str]) {
        state.push_output("Memory: Free DRAM ~320KB, Free PSRAM ~16MB");
        state.push_output("Storage: /internal (LittleFS 11MB Partition)");
        if std::path::Path::new("/sdcard/.unformatted").exists()
            || std::path::Path::new("./sdcard/.unformatted").exists()
        {
            state.push_output("Storage: /sdcard (MicroSD Card Detected, Unformatted / FAT32 Required)");
        } else if std::path::Path::new("/sdcard").exists() || std::path::Path::new("./sdcard").exists() {
            state.push_output("Storage: /sdcard (MicroSD Card Mounted)");
        } else {
            state.push_output("Storage: /sdcard (MicroSD Card Not Inserted)");
        }
    }
}
