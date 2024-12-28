//#![warn(missing_docs)]
#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]
use sqlx::{
	encode::IsNull,
	sqlite::{Sqlite, SqliteArgumentValue, SqliteTypeInfo, SqliteValueRef},
	Decode, Encode, Type,
};
use std::time::Duration;

/// All possible options for retaining tasks in the db after their execution.
///
/// The default mode is [`RetentionMode::RemoveAll`]
#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
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

/// All possible options for backoff between task retries.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash, serde::Serialize, serde::Deserialize)]
pub enum BackoffMode {
	/// No backoff, retry immediately
	NoBackoff,

	/// Exponential backoff
	ExponentialBackoff,
}

impl Default for BackoffMode {
	fn default() -> Self {
		Self::ExponentialBackoff
	}
}

impl BackoffMode {
	fn next_attempt(&self, attempt: i32) -> Duration {
		match self {
			Self::NoBackoff => Duration::from_secs(0),
			Self::ExponentialBackoff => Duration::from_secs(2u64.saturating_pow(attempt.saturating_add(1) as u32)),
		}
	}
}

impl Type<Sqlite> for BackoffMode {
	fn type_info() -> SqliteTypeInfo {
		<serde_json::Value as Type<Sqlite>>::type_info()
	}
}

impl Encode<'_, Sqlite> for BackoffMode {
	fn encode_by_ref(&self, args: &mut Vec<SqliteArgumentValue<'_>>) -> IsNull {
		match serde_json::to_value(self) {
			Ok(value) => value.encode_by_ref(args),
			Err(_) => IsNull::Yes,
		}
	}
}

impl<'r> Decode<'r, Sqlite> for BackoffMode {
	fn decode(value: SqliteValueRef<'r>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
		let json_value = <serde_json::Value as Decode<Sqlite>>::decode(value)?;
		let backoff_mode: Self = serde_json::from_value(json_value)?;
		Ok(backoff_mode)
	}
}

pub use runnable::BackgroundTask;
pub use sqlite_task::{CurrentTask, NewTask, Task, TaskHash, TaskId, TaskState};
pub use store::{BackgroundTaskExt, TaskStore};
pub use worker::Worker;
pub use worker_pool::{QueueConfig, WorkerPool};

// #[cfg(feature = "async_postgres")]
// pub use store::PgTaskStore;

mod catch_unwind;
pub mod errors;
#[cfg(feature = "async_postgres")]
mod queries;
mod runnable;
#[cfg(feature = "async_postgres")]
mod schema;
mod sqlite_helpers;
mod sqlite_task;
mod store;
// mod task;
mod worker;
mod worker_pool;
