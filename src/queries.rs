use crate::errors::AsyncQueueError;
use crate::sqlite_helpers::SqliteDateTime;
use crate::sqlite_task::{NewTask, Task, TaskId};
use chrono::Utc;
use sqlx::SqliteConnection;
use std::time::Duration;

impl Task {
	#[allow(dead_code)]
	pub(crate) async fn remove(connection: &mut SqliteConnection, id: TaskId) -> Result<u64, AsyncQueueError> {
		let result = sqlx::query!("DELETE FROM backie_tasks WHERE id = ?", id).execute(connection).await?;

		Ok(result.rows_affected())
	}

	#[allow(dead_code)]
	pub(crate) async fn fail_with_message(connection: &mut SqliteConnection, id: TaskId, error_message: &str) -> Result<Self, AsyncQueueError> {
		let error = serde_json::json!({
				"error": error_message,
		});
		let now = SqliteDateTime(Utc::now());

		let task = sqlx::query_as!(
			Self,
			r#"UPDATE backie_tasks 
            SET error_info = ?, done_at = ?
            WHERE id = ?
            RETURNING *"#,
			error,
			now,
			id
		)
		.fetch_one(connection)
		.await?;

		Ok(task)
	}

	#[allow(dead_code)]
	pub(crate) async fn schedule_retry(connection: &mut SqliteConnection, id: TaskId, backoff: Duration, error_message: &str) -> Result<Self, AsyncQueueError> {
		let error = serde_json::json!({
				"error": error_message,
		});
		let scheduled_at = SqliteDateTime(Utc::now() + chrono::Duration::from_std(backoff).unwrap_or_else(|_| chrono::Duration::max_value()));

		let task = sqlx::query_as!(
			Self,
			r#"UPDATE backie_tasks 
            SET error_info = ?,
                retries = retries + 1,
                scheduled_at = ?,
                running_at = NULL
            WHERE id = ?
            RETURNING *"#,
			error,
			scheduled_at,
			id
		)
		.fetch_one(connection)
		.await?;

		Ok(task)
	}

	#[allow(dead_code)]
	pub(crate) async fn fetch_next_pending(connection: &mut SqliteConnection, queue_name: &str, execution_timeout: Option<Duration>, task_names: &[String]) -> Option<Self> {
		let now = SqliteDateTime(Utc::now());
		let task_names_json = serde_json::to_value(task_names).unwrap();

		match execution_timeout {
			Some(timeout) => {
				let timeout_threshold = SqliteDateTime(Utc::now() - chrono::Duration::from_std(timeout).unwrap_or_else(|_| chrono::Duration::max_value()));

				sqlx::query_as!(
					Self,
					r#"SELECT * FROM backie_tasks
                    WHERE task_name IN (SELECT value FROM json_each(?))
                    AND scheduled_at < ?
                    AND done_at IS NULL
                    AND queue_name = ?
                    AND (running_at IS NULL OR running_at < ?)
                    ORDER BY created_at ASC
                    LIMIT 1"#,
					task_names_json,
					now,
					queue_name,
					timeout_threshold
				)
				.fetch_optional(connection)
				.await
				.ok()
				.flatten()
			}
			None => sqlx::query_as!(
				Self,
				r#"SELECT * FROM backie_tasks
                    WHERE task_name IN (SELECT value FROM json_each(?))
                    AND scheduled_at < ?
                    AND done_at IS NULL
                    AND queue_name = ?
                    AND running_at IS NULL
                    ORDER BY created_at ASC
                    LIMIT 1"#,
				task_names_json,
				now,
				queue_name
			)
			.fetch_optional(connection)
			.await
			.ok()
			.flatten(),
		}
	}

	#[allow(dead_code)]
	pub(crate) async fn set_running(connection: &mut SqliteConnection, task: Self) -> Result<Self, AsyncQueueError> {
		let now = SqliteDateTime(Utc::now());
		let task = sqlx::query_as!(
			Self,
			r#"UPDATE backie_tasks 
            SET running_at = ?
            WHERE id = ?
            RETURNING *"#,
			now,
			task.id
		)
		.fetch_one(connection)
		.await?;

		Ok(task)
	}

	#[allow(dead_code)]
	pub(crate) async fn set_done(connection: &mut SqliteConnection, id: TaskId) -> Result<Self, AsyncQueueError> {
		let now = SqliteDateTime(Utc::now());
		let task = sqlx::query_as!(
			Self,
			r#"UPDATE backie_tasks 
            SET done_at = ?
            WHERE id = ?
            RETURNING *"#,
			now,
			id
		)
		.fetch_one(connection)
		.await?;

		Ok(task)
	}

	#[allow(dead_code)]
	pub(crate) async fn insert(connection: &mut SqliteConnection, new_task: NewTask) -> Result<Self, AsyncQueueError> {
		let (task_name, queue_name, uniq_hash, payload, timeout_msecs, max_retries, backoff_mode) = new_task.into_values();
		let id = TaskId::from(uuid::Uuid::new_v4());
		let now = SqliteDateTime(Utc::now());

		let task = sqlx::query_as!(
			Self,
			r#"INSERT INTO backie_tasks (
                id, task_name, queue_name, uniq_hash, payload, 
                timeout_msecs, created_at, scheduled_at, 
                max_retries, backoff_mode, retries
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0)
            RETURNING *"#,
			id,
			task_name,
			queue_name,
			uniq_hash,
			payload,
			timeout_msecs,
			now,
			now,
			max_retries,
			backoff_mode
		)
		.fetch_one(connection)
		.await?;

		Ok(task)
	}
}

// use diesel::prelude::*;
// use diesel::ExpressionMethods;
// use diesel_async::{pg::AsyncPgConnection, RunQueryDsl};

// impl Task {
// 	pub(crate) async fn remove(connection: &mut AsyncPgConnection, id: TaskId) -> Result<u64, AsyncQueueError> {
// 		let query = backie_tasks::table.filter(backie_tasks::id.eq(id));
// 		Ok(diesel::delete(query).execute(connection).await? as u64)
// 	}
//
// 	pub(crate) async fn fail_with_message(connection: &mut AsyncPgConnection, id: TaskId, error_message: &str) -> Result<Task, AsyncQueueError> {
// 		let error = serde_json::json!({
// 				"error": error_message,
// 		});
// 		let query = backie_tasks::table.filter(backie_tasks::id.eq(id));
// 		Ok(
// 			diesel::update(query)
// 				.set((backie_tasks::error_info.eq(Some(error)), backie_tasks::done_at.eq(Utc::now())))
// 				.get_result::<Task>(connection)
// 				.await?,
// 		)
// 	}
//
// 	pub(crate) async fn schedule_retry(connection: &mut AsyncPgConnection, id: TaskId, backoff: Duration, error_message: &str) -> Result<Task, AsyncQueueError> {
// 		use crate::schema::backie_tasks::dsl;
//
// 		let error = serde_json::json!({
// 				"error": error_message,
// 		});
//
// 		let task = diesel::update(backie_tasks::table.filter(backie_tasks::id.eq(id)))
// 			.set((
// 				backie_tasks::error_info.eq(Some(error)),
// 				backie_tasks::retries.eq(dsl::retries + 1),
// 				backie_tasks::scheduled_at.eq(Utc::now() + chrono::Duration::from_std(backoff).unwrap_or(chrono::Duration::max_value())),
// 				backie_tasks::running_at.eq::<Option<DateTime<Utc>>>(None),
// 			))
// 			.get_result::<Task>(connection)
// 			.await?;
//
// 		Ok(task)
// 	}
//
// 	pub(crate) async fn fetch_next_pending(connection: &mut AsyncPgConnection, queue_name: &str, execution_timeout: Option<Duration>, task_names: &[String]) -> Option<Task> {
// 		if let Some(execution_timeout) = execution_timeout {
// 			backie_tasks::table
// 				.filter(backie_tasks::task_name.eq_any(task_names))
// 				.filter(backie_tasks::scheduled_at.lt(Utc::now())) // skip tasks scheduled for the future
// 				.order(backie_tasks::created_at.asc()) // get the oldest task first
// 				.filter(backie_tasks::done_at.is_null()) // and not marked as done
// 				.filter(backie_tasks::queue_name.eq(queue_name))
// 				.filter(
// 					backie_tasks::running_at
// 						.is_null()
// 						.or(backie_tasks::running_at.lt(Utc::now() - chrono::Duration::from_std(execution_timeout).unwrap_or(chrono::Duration::max_value()))),
// 				) // that is not marked as running already or expired
// 				.for_update()
// 				.skip_locked()
// 				.limit(1)
// 				.get_result::<Task>(connection)
// 				.await
// 				.ok()
// 		} else {
// 			backie_tasks::table
// 				.filter(backie_tasks::task_name.eq_any(task_names))
// 				.filter(backie_tasks::scheduled_at.lt(Utc::now())) // skip tasks scheduled for the future
// 				.order(backie_tasks::created_at.asc()) // get the oldest task first
// 				.filter(backie_tasks::done_at.is_null()) // and not marked as done
// 				.filter(backie_tasks::queue_name.eq(queue_name))
// 				.filter(backie_tasks::running_at.is_null()) // that is not marked as running already
// 				.for_update()
// 				.skip_locked()
// 				.limit(1)
// 				.get_result::<Task>(connection)
// 				.await
// 				.ok()
// 		}
// 	}
//
// 	pub(crate) async fn set_running(connection: &mut AsyncPgConnection, task: Task) -> Result<Task, AsyncQueueError> {
// 		Ok(diesel::update(&task).set((backie_tasks::running_at.eq(Utc::now()),)).get_result::<Task>(connection).await?)
// 	}
//
// 	pub(crate) async fn set_done(connection: &mut AsyncPgConnection, id: TaskId) -> Result<Task, AsyncQueueError> {
// 		Ok(
// 			diesel::update(backie_tasks::table.filter(backie_tasks::id.eq(id)))
// 				.set((backie_tasks::done_at.eq(Utc::now()),))
// 				.get_result::<Task>(connection)
// 				.await?,
// 		)
// 	}
//
// 	pub(crate) async fn insert(connection: &mut AsyncPgConnection, new_task: NewTask) -> Result<Task, AsyncQueueError> {
// 		Ok(diesel::insert_into(backie_tasks::table).values(new_task).get_result::<Task>(connection).await?)
// 	}
// }
//
//
