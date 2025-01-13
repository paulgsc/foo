use crate::sqlite_helpers::SqliteValidate;
use crate::sqlite_helpers::{OptionalJsonValue, OptionalSqliteDateTime, SqliteDateTime};
use crate::BackoffMode;
use serde::{Deserialize, Serialize};
use sqlite_macros::SqliteType;
use sqlx::{Error, FromRow};
use std::borrow::Cow;
use std::fmt;
use std::str::FromStr;
use std::time::Duration;
use uuid::Uuid;

// use sqlx::sqlite::{SqliteArgumentValue, SqliteTypeInfo, SqliteValueRef};

/// States of a task.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TaskState {
	Ready,
	Running,
	Failed(String),
	Done,
}

#[derive(Clone, Copy, Debug, Ord, PartialOrd, Hash, PartialEq, Eq, Serialize, Deserialize, SqliteType)]
#[sqlite_type(validate = true, error = "Invalid UUID format")]
pub struct TaskId(Uuid);

impl fmt::Display for TaskId {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.0)
	}
}

impl From<Uuid> for TaskId {
	fn from(value: Uuid) -> Self {
		Self(value)
	}
}

impl From<TaskId> for Uuid {
	fn from(value: TaskId) -> Self {
		value.0
	}
}

impl FromStr for TaskId {
	type Err = uuid::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(Self(Uuid::parse_str(s)?))
	}
}

impl From<String> for TaskId {
	fn from(s: String) -> Self {
		Self::from_str(&s).expect("Invalid UUID string")
	}
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize, SqliteType)]
#[sqlite_type(validate = true)]
pub struct TaskHash(Cow<'static, str>);

impl std::fmt::Display for TaskHash {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

impl TaskHash {
	pub fn new<T: Into<String>>(hash: T) -> Self {
		Self(Cow::Owned(hash.into()))
	}
}

impl FromStr for TaskHash {
	type Err = sqlx::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		if s.is_empty() {
			Err(Error::Protocol("TaskHash cannot be empty".into()))
		} else {
			Ok(TaskHash(Cow::Owned(s.to_string())))
		}
	}
}

impl From<String> for TaskHash {
	fn from(s: String) -> Self {
		Self(Cow::Owned(s))
	}
}

impl From<TaskHash> for String {
	fn from(hash: TaskHash) -> Self {
		hash.0.into_owned()
	}
}

impl AsRef<str> for TaskHash {
	fn as_ref(&self) -> &str {
		&self.0
	}
}

impl SqliteValidate for TaskHash {
	type Error = sqlx::Error;

	fn validate(s: &str) -> Result<(), sqlx::Error> {
		if s.is_empty() {
			Err(sqlx::Error::Protocol("TaskHash cannot be empty".into()))
		} else {
			Ok(())
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq, SqliteType)]
#[sqlite_type(validate = true)]
pub struct OptionalTaskHash(pub Option<TaskHash>);

impl std::fmt::Display for OptionalTaskHash {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match &self.0 {
			Some(t) => write!(f, "{}", t),
			None => write!(f, "NULL"),
		}
	}
}

impl From<Option<String>> for OptionalTaskHash {
	fn from(option: Option<String>) -> Self {
		Self(option.map(TaskHash::from))
	}
}

impl FromStr for OptionalTaskHash {
	type Err = sqlx::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		if s.is_empty() {
			Ok(Self(None))
		} else {
			TaskHash::from_str(s).map(|hash| Self(Some(hash)))
		}
	}
}

#[derive(Debug, Eq, PartialEq, Clone, FromRow)]
pub struct Task {
	pub id: TaskId,
	pub task_name: String,
	pub queue_name: String,
	pub uniq_hash: OptionalTaskHash,
	pub payload: serde_json::Value,
	pub timeout_msecs: i64,
	#[sqlx(rename = "created_at")]
	pub created_at: SqliteDateTime,
	#[sqlx(rename = "scheduled_at")]
	pub scheduled_at: SqliteDateTime,
	#[sqlx(rename = "running_at")]
	pub running_at: OptionalSqliteDateTime,
	#[sqlx(rename = "done_at")]
	pub done_at: OptionalSqliteDateTime,
	pub error_info: OptionalJsonValue,
	pub retries: i64,
	pub max_retries: i64,
	pub backoff_mode: BackoffMode,
}

impl Task {
	#[must_use]
	pub fn state(&self) -> TaskState {
		match (self.done_at.0, &self.error_info.0) {
			(Some(_), Some(error)) => TaskState::Failed(error.to_string()),
			(Some(_), None) => TaskState::Done,
			(None, _) if self.running_at.0.is_some() => TaskState::Running,
			_ => TaskState::Ready,
		}
	}
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct NewTask {
	pub(crate) task_name: String,
	pub(crate) queue_name: String,
	pub(crate) uniq_hash: Option<TaskHash>,
	pub(crate) payload: serde_json::Value,
	pub(crate) timeout_msecs: i64,
	pub(crate) max_retries: i32,
	pub(crate) backoff_mode: BackoffMode,
}

impl NewTask {
	pub fn with_timeout<T>(background_task: T, timeout: Duration) -> Result<Self, serde_json::Error>
	where
		T: crate::BackgroundTask,
	{
		Ok(Self {
			task_name: T::TASK_NAME.to_string(),
			queue_name: T::QUEUE.to_string(),
			uniq_hash: background_task.uniq(),
			payload: serde_json::to_value(background_task)?,
			timeout_msecs: timeout.as_millis() as i64,
			max_retries: T::MAX_RETRIES,
			backoff_mode: T::BACKOFF_MODE,
		})
	}

	pub fn new<T>(background_task: T) -> Result<Self, serde_json::Error>
	where
		T: crate::BackgroundTask,
	{
		Self::with_timeout(background_task, Duration::from_secs(120))
	}

	#[must_use]
	pub fn into_values(self) -> (String, String, Option<TaskHash>, serde_json::Value, i64, i32, BackoffMode) {
		(
			self.task_name,
			self.queue_name,
			self.uniq_hash,
			self.payload,
			self.timeout_msecs,
			self.max_retries,
			self.backoff_mode,
		)
	}
}

#[derive(Debug, Clone, Copy)]
pub struct CurrentTask {
	id: TaskId,
	retries: i64,
	created_at: SqliteDateTime,
}

impl CurrentTask {
	#[must_use]
	pub const fn new(task: &Task) -> Self {
		Self {
			id: task.id,
			retries: task.retries,
			created_at: task.created_at,
		}
	}

	#[must_use]
	pub const fn id(&self) -> TaskId {
		self.id
	}

	#[must_use]
	pub const fn retry_count(&self) -> i64 {
		self.retries
	}

	#[must_use]
	pub const fn created_at(&self) -> SqliteDateTime {
		self.created_at
	}
}
