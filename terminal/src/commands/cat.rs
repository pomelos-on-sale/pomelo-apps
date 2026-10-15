use std::fs::File;
use std::io::{BufRead, BufReader};

use super::Command;
use crate::shell::{resolve_path, TerminalModel};

pub struct CatCommand;

impl Command for CatCommand {
    fn name(&self) -> &'static str {
        "cat"
    }

    fn description(&self) -> &'static str {
        "Concatenate and display file content"
    }

    fn execute(&self, state: &mut TerminalModel, args: &[&str]) {
        if let Some(filename) = args.first() {
            let path_str = resolve_path(&state.cwd, filename);
            match File::open(&path_str) {
                Ok(file) => {
                    let reader = BufReader::new(file);
                    let mut lines_output = 0;
                    for line_res in reader.lines() {
                        if lines_output >= 12 {
                            break;
                        }
                        match line_res {
                            Ok(line) => {
                                state.push_output(&line);
                                lines_output += 1;
                            }
                            Err(e) => {
                                state.push_output(&format!("cat: error reading '{}': {}", filename, e));
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    state.push_output(&format!("cat: {}: {}", filename, e));
                }
            }
        } else {
            state.push_output("cat: missing file operand");
        }
    }
}
