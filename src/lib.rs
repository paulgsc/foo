//#![warn(missing_docs)]
#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]
use crate::sqlite_helpers::SqliteValidate;
use sqlite_macros::SqliteType;
use sqlx::{sqlite::Sqlite, Decode, Encode, Type};
use std::fmt;
use std::str::FromStr;
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
#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash, serde::Serialize, serde::Deserialize, SqliteType)]
pub enum BackoffMode {
	/// No backoff, retry immediately
	NoBackoff,

	/// Exponential backoff
	ExponentialBackoff,
}

impl fmt::Display for BackoffMode {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::NoBackoff => write!(f, "NoBackoff"),
			Self::ExponentialBackoff => write!(f, "ExponentialBackoff"),
		}
	}
}

impl Default for BackoffMode {
	fn default() -> Self {
		Self::ExponentialBackoff
	}
}

impl BackoffMode {
	const fn next_attempt(&self, attempt: i32) -> Duration {
		match self {
			Self::NoBackoff => Duration::from_secs(0),
			Self::ExponentialBackoff => Duration::from_secs(2u64.saturating_pow(attempt.saturating_add(1) as u32)),
		}
	}
}

impl FromStr for BackoffMode {
	type Err = sqlx::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.to_lowercase().as_str() {
			"nobackoff" => Ok(Self::NoBackoff),
			"exponentialbackoff" => Ok(Self::ExponentialBackoff),
			_ => Err(sqlx::Error::Protocol("Invalid backoff mode".into())),
		}
	}
}

impl From<String> for BackoffMode {
	fn from(s: String) -> Self {
		Self::from_str(s.as_str()).unwrap_or(Self::NoBackoff)
	}
}

impl SqliteValidate for BackoffMode {
	type Error = sqlx::Error;

	fn validate(s: &str) -> Result<(), Self::Error> {
		Self::from_str(s).map(|_| ())
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
// mod schema;
mod sqlite_helpers;
mod sqlite_task;
mod store;
// mod task;
mod worker;
mod worker_pool;
