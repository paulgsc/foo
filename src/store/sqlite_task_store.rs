use crate::errors::AsyncQueueError;
use crate::sqlite_task::{NewTask, Task, TaskId, TaskState};
use crate::{BackgroundTask, TaskStore};
use sqlx::{Acquire, Pool, Sqlite, SqliteConnection, SqlitePool};
use std::time::Duration;

/// An async queue that uses `SQLite` as storage for tasks.
#[derive(Debug, Clone)]
pub struct SqliteTaskStore {
	pub pool: Pool<Sqlite>,
}

impl SqliteTaskStore {
	#[allow(dead_code)]
	pub const fn new(pool: Pool<Sqlite>) -> Self {
		Self { pool }
	}

	/// Create a new `SQLite` pool with the given connection string
	#[allow(dead_code)]
	pub async fn create(database_url: &str) -> Result<Self, sqlx::Error> {
		let pool = SqlitePool::connect(database_url).await?;
		Ok(Self::new(pool))
	}
}

#[async_trait::async_trait]
impl TaskStore for SqliteTaskStore {
	type Connection = SqliteConnection;

	async fn pull_next_task(&self, queue_name: &str, execution_timeout: Option<Duration>, task_names: &[String]) -> Result<Option<Task>, AsyncQueueError> {
		let mut conn = self.pool.acquire().await.map_err(AsyncQueueError::from)?;

		let mut tx = conn.begin().await.map_err(AsyncQueueError::from)?;

		let pending_task = match Task::fetch_next_pending(&mut tx, queue_name, execution_timeout, task_names).await {
			Some(task) => task,
			None => {
				tx.commit().await.map_err(AsyncQueueError::from)?;
				return Ok(None);
			}
		};

		let result = Task::set_running(&mut tx, pending_task).await?;

		tx.commit().await.map_err(AsyncQueueError::from)?;

		Ok(Some(result))
	}

	async fn set_task_state(&self, id: TaskId, state: TaskState) -> Result<(), AsyncQueueError> {
		let mut conn = self.pool.acquire().await.map_err(AsyncQueueError::from)?;
		match state {
			TaskState::Done => {
				Task::set_done(&mut conn, id).await?;
			}
			TaskState::Failed(error_msg) => {
				Task::fail_with_message(&mut conn, id, &error_msg).await?;
			}
			_ => (),
		}
		Ok(())
	}

	async fn remove_task(&self, id: TaskId) -> Result<u64, AsyncQueueError> {
		let mut conn = self.pool.acquire().await.map_err(AsyncQueueError::from)?;
		let result = Task::remove(&mut conn, id).await?;

		Ok(result)
	}

	async fn enqueue<T: BackgroundTask>(connection: &mut Self::Connection, task: T) -> Result<(), AsyncQueueError> {
		let new_task = NewTask::new(task)?;
		Task::insert(connection, new_task).await?;
		Ok(())
	}

	async fn schedule_task_retry(&self, id: TaskId, backoff: Duration, error: &str) -> Result<Task, AsyncQueueError> {
		let mut conn = self.pool.acquire().await.map_err(AsyncQueueError::from)?;
		let task = Task::schedule_retry(&mut conn, id, backoff, error).await?;
		Ok(task)
	}
}
