//#![warn(missing_docs)]
#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]

/// All possible options for retaining tasks in the db after their execution.
///
/// The default mode is [`RetentionMode::RemoveAll`]
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum RetentionMode {
    /// Keep all tasks
    KeepAll,

    /// Remove all finished tasks independently of their final execution state.
    RemoveAll,

    /// Remove only successfully finished tasks
    RemoveDone,
}

impl Default for RetentionMode {
    fn default() -> Self {
        Self::RemoveDone
    }
}

pub use runnable::BackgroundTask;
pub use store::{PgTaskStore, TaskStore};
pub use task::{CurrentTask, Task, TaskId, TaskState};
pub use worker_pool::WorkerPool;
pub use worker::Worker;
pub use queue::Queue;

pub mod errors;
mod queries;
mod queue;
mod runnable;
mod schema;
mod store;
mod task;
mod worker;
mod worker_pool;
