use crate::sqlite_helpers::{OptionalJsonValue, OptionalSqliteDateTime, SqliteDateTime};
use crate::BackoffMode;
use serde::{Deserialize, Serialize};
use sqlx::database::HasArguments;
use sqlx::encode::IsNull;
use sqlx::sqlite::{SqliteRow, SqliteTypeInfo, SqliteValueRef};
use sqlx::{Decode, Encode, Error, FromRow, Row, Sqlite, Type};
use std::borrow::Cow;
use std::str::FromStr;
use std::time::Duration;
use uuid::Uuid;

/// States of a task.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TaskState {
	Ready,
	Running,
	Failed(String),
	Done,
}

#[derive(Clone, Copy, Debug, Ord, PartialOrd, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskId(Uuid);

impl std::fmt::Display for TaskId {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

impl Type<Sqlite> for TaskId {
	fn type_info() -> SqliteTypeInfo {
		<&str as Type<Sqlite>>::type_info()
	}
}

impl Encode<'_, Sqlite> for TaskId {
	fn encode_by_ref(&self, buf: &mut <Sqlite as HasArguments>::ArgumentBuffer) -> IsNull {
		<std::string::String as Encode<'_, Sqlite>>::encode(self.0.to_string(), buf)
	}
}

impl<'r> Decode<'r, Sqlite> for TaskId {
	fn decode(value: SqliteValueRef<'r>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
		let s = <String as Decode<Sqlite>>::decode(value)?;
		Ok(Self(Uuid::parse_str(&s)?))
	}
}

impl Encode<'_, Sqlite> for TaskHash {
	fn encode_by_ref(&self, buf: &mut <Sqlite as HasArguments>::ArgumentBuffer) -> IsNull {
		self.0.clone().encode(buf)
	}
}

impl<'r> Decode<'r, Sqlite> for TaskHash {
	fn decode(value: SqliteValueRef<'r>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
		let s = <String as Decode<Sqlite>>::decode(value)?;
		Ok(Self::new(s))
	}
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskHash(Cow<'static, str>);

impl TaskHash {
	pub fn new<T: Into<String>>(hash: T) -> Self {
		Self(Cow::Owned(hash.into()))
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

impl Type<Sqlite> for TaskHash {
	fn type_info() -> SqliteTypeInfo {
		<&str as Type<Sqlite>>::type_info()
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionalTaskHash(pub Option<TaskHash>);

impl From<Option<String>> for OptionalTaskHash {
	fn from(option: Option<String>) -> Self {
		Self(option.map(TaskHash::from))
	}
}

impl Type<Sqlite> for OptionalTaskHash {
	fn type_info() -> SqliteTypeInfo {
		<&str as Type<Sqlite>>::type_info()
	}
}

impl Encode<'_, Sqlite> for OptionalTaskHash {
	fn encode_by_ref(&self, buf: &mut <Sqlite as HasArguments>::ArgumentBuffer) -> IsNull {
		match &self.0 {
			Some(hash) => {
				hash.encode_by_ref(buf);
				IsNull::No
			}
			None => IsNull::Yes,
		}
	}
}

impl<'r> Decode<'r, Sqlite> for OptionalTaskHash {
	fn decode(value: SqliteValueRef<'r>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
		let text: Option<String> = <Option<String> as Decode<Sqlite>>::decode(value)?;
		Ok(Self(text.map(TaskHash::from)))
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
	pub retries: i32,
	pub max_retries: i32,
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

impl TryFrom<SqliteRow> for Task {
	type Error = sqlx::Error;

	fn try_from(row: SqliteRow) -> Result<Self, Self::Error> {
		let id = row
			.try_get::<String, _>("id")
			.and_then(|id| Uuid::parse_str(&id).map_err(|e| Error::Protocol(format!("Invalid UUID: {e}"))).map(TaskId::from))?;

		Ok(Self {
			id,
			task_name: row.try_get("task_name")?,
			queue_name: row.try_get("queue_name")?,
			uniq_hash: row.try_get("uniq_hash")?,
			payload: row.try_get("payload")?,
			timeout_msecs: row.try_get("timeout_msecs")?,
			created_at: row.try_get("created_at")?,
			scheduled_at: row.try_get("scheduled_at")?,
			running_at: row.try_get("running_at")?,
			done_at: row.try_get("done_at")?,
			error_info: row.try_get("error_info")?,
			retries: row.try_get("retries")?,
			max_retries: row.try_get("max_retries")?,
			backoff_mode: row.try_get("backoff_mode")?,
		})
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
	retries: i32,
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
	pub const fn retry_count(&self) -> i32 {
		self.retries
	}

	#[must_use]
	pub const fn created_at(&self) -> SqliteDateTime {
		self.created_at
	}
}
