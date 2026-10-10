//! System background task management and coordination.
//!
//! Provides an extensible pipeline for coordinating system-level background jobs
//! (Wi-Fi autoconnect, SNTP time synchronization, update checks, etc.) via Iced tasks and messages.
//!
//! Long-running blocking calls are dispatched to dedicated worker threads so that the single-threaded
//! UI `Pump` frame loop is never stalled.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use iced::Task;
use pomelo_hal::Board;

use crate::Message;

/// Identifier for system-level background tasks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SystemTaskId {
    /// Wi-Fi autoconnect at boot or upon credential reload.
    WifiAutoConnect,
    /// SNTP network time synchronization.
    TimeSync,
}

/// Lifecycle status of a system task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskStatus {
    Idle,
    Running,
    Success,
    Failed(String),
}

/// Messages emitted during system task lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SystemTaskMessage {
    /// A task has started execution.
    Started(SystemTaskId),
    /// A task has finished execution with success or failure.
    Finished {
        id: SystemTaskId,
        result: Result<(), String>,
    },
}

/// System task manager tracking the progress of all known background tasks.
#[derive(Debug, Clone, Default)]
pub struct TaskManager {
    statuses: HashMap<SystemTaskId, TaskStatus>,
}

impl TaskManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn status(&self, id: SystemTaskId) -> TaskStatus {
        self.statuses.get(&id).cloned().unwrap_or(TaskStatus::Idle)
    }

    pub fn set_status(&mut self, id: SystemTaskId, status: TaskStatus) {
        self.statuses.insert(id, status);
    }

    pub fn is_running(&self, id: SystemTaskId) -> bool {
        matches!(self.statuses.get(&id), Some(TaskStatus::Running))
    }
}

/// Dispatches a system task to run asynchronously on a background worker thread,
/// yielding an Iced `Task<Message>` that delivers the completion message upon finish.
pub fn perform(board: Arc<Board>, id: SystemTaskId) -> Task<Message> {
    Task::perform(
        async move {
            let (tx, rx) = iced::futures::channel::oneshot::channel();

            let task_board = Arc::clone(&board);
            let thread_name = format!("task_{id:?}");

            std::thread::Builder::new()
                .name(thread_name)
                .spawn(move || {
                    let res = execute_task(&task_board, id);
                    let _ = tx.send(res);
                })
                .expect("failed to spawn system task worker thread");

            match rx.await {
                Ok(result) => SystemTaskMessage::Finished { id, result },
                Err(_) => SystemTaskMessage::Finished {
                    id,
                    result: Err("worker thread terminated unexpectedly".to_string()),
                },
            }
        },
        Message::SystemTask,
    )
}

fn execute_task(board: &Board, id: SystemTaskId) -> Result<(), String> {
    match id {
        SystemTaskId::WifiAutoConnect => {
            board
                .wifi()
                .autoconnect()
                .map_err(|e| format!("{e:?}"))
        }
        SystemTaskId::TimeSync => {
            // Wait up to 5 seconds for SNTP sync
            board
                .time()
                .sync(None, Duration::from_secs(5))
                .map(|_unix| ())
                .map_err(|e| format!("{e:?}"))
        }
    }
}
