use crate::errors::AsyncQueueError;
use crate::sqlite_task::{NewTask, Task, TaskId, TaskState};
use crate::{BackgroundTask, TaskStore};
use sqlx::{Pool, Sqlite, SqlitePool};
use std::time::Duration;

/// An async queue that uses `SQLite` as storage for tasks.
#[derive(Debug, Clone)]
pub struct SqliteTaskStore {
	pool: Pool<Sqlite>,
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
	type Connection = Pool<Sqlite>;

	async fn pull_next_task(&self, queue_name: &str, execution_timeout: Option<Duration>, task_names: &[String]) -> Result<Option<Task>, AsyncQueueError> {
		let mut tx = self.pool.begin().await.map_err(AsyncQueueError::from)?;

		// Convert task_names array to a comma-separated string for SQL IN clause
		let task_names_list = task_names.iter().map(|s| format!("'{s}'")).collect::<Vec<_>>().join(",");

		let timeout_clause = if let Some(timeout) = execution_timeout {
			format!(
				"AND (last_execution_date IS NULL OR 
                 DATETIME(last_execution_date, '+{} seconds') <= DATETIME('now'))",
				timeout.as_secs()
			)
		} else {
			String::new()
		};

		let query = format!(
			r#"
            SELECT * FROM tasks
            WHERE queue_name = ?
            AND state = 'pending'
            AND name IN ({task_names_list})
            {timeout_clause}
            ORDER BY priority DESC, created_at ASC
            LIMIT 1
            "#
		);

		let task = sqlx::query_as::<_, Task>(&query)
			.bind(queue_name)
			.fetch_optional(&mut *tx)
			.await
			.map_err(AsyncQueueError::from)?;

		if let Some(task) = task {
			// Update the task state to running
			sqlx::query(
				r"
                UPDATE tasks
                SET state = 'running',
                    last_execution_date = DATETIME('now')
                WHERE id = ?
                ",
			)
			.bind(task.id)
			.execute(&mut *tx)
			.await
			.map_err(AsyncQueueError::from)?;

			tx.commit().await.map_err(AsyncQueueError::from)?;
			Ok(Some(task))
		} else {
			Ok(None)
		}
	}

	async fn set_task_state(&self, id: TaskId, state: TaskState) -> Result<(), AsyncQueueError> {
		match state {
			TaskState::Done => {
				sqlx::query(
					r"
                    UPDATE tasks
                    SET state = 'done',
                        completed_at = DATETIME('now')
                    WHERE id = ?
                    ",
				)
				.bind(id)
				.execute(&self.pool)
				.await
				.map_err(AsyncQueueError::from)?;
			}
			TaskState::Failed(error_msg) => {
				sqlx::query(
					r"
                    UPDATE tasks
                    SET state = 'failed',
                        error = ?,
                        failed_at = DATETIME('now')
                    WHERE id = ?
                    ",
				)
				.bind(error_msg)
				.bind(id)
				.execute(&self.pool)
				.await
				.map_err(AsyncQueueError::from)?;
			}
			_ => (),
		}
		Ok(())
	}

	async fn remove_task(&self, id: TaskId) -> Result<u64, AsyncQueueError> {
		let result = sqlx::query(
			r"
            DELETE FROM tasks
            WHERE id = ?
            ",
		)
		.bind(id)
		.execute(&self.pool)
		.await
		.map_err(AsyncQueueError::from)?;

		Ok(result.rows_affected())
	}

	async fn enqueue<T: BackgroundTask>(connection: &Self::Connection, task: T) -> Result<(), AsyncQueueError> {
		let new_task = NewTask::new(task)?;

		sqlx::query(
			r"
            INSERT INTO tasks (
                name, queue_name, priority, payload,
                state, created_at
            )
            VALUES (?, ?, ?, ?, 'pending', DATETIME('now'))
            ",
		)
		.bind(&new_task.task_name)
		.bind(&new_task.queue_name)
		.bind(&new_task.payload)
		.execute(connection)
		.await
		.map_err(AsyncQueueError::Sqlx)?;

		Ok(())
	}

	async fn schedule_task_retry(&self, id: TaskId, backoff: Duration, error: &str) -> Result<Task, AsyncQueueError> {
		let retry_at = format!("DATETIME('now', '+{} seconds')", backoff.as_secs());

		sqlx::query(&format!(
			r#"
                UPDATE tasks
                SET state = 'pending',
                    retry_count = retry_count + 1,
                    last_error = ?,
                    next_retry_date = {retry_at}
                WHERE id = ?
                RETURNING *
                "#
		))
		.bind(error)
		.bind(id)
		.fetch_one(&self.pool)
		.await
		.map_err(AsyncQueueError::from)
		.and_then(|row| Task::try_from(row).map_err(AsyncQueueError::Sqlx))
	}
}
