#[cfg(feature = "async_postgres")]
use crate::schema::backie_tasks;
use crate::BackoffMode;
use chrono::DateTime;
use chrono::Utc;
use diesel_derive_newtype::DieselNewType;
use serde::Serialize;
use std::borrow::Cow;
use std::fmt;
use std::fmt::Display;
use uuid::Uuid;

/// States of a task.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TaskState {
    /// The task is ready to be executed.
    Ready,

    /// The task is running.
    Running,

    /// The task has failed to execute.
    Failed(String),

    /// The task finished successfully.
    Done,
}

#[derive(Clone, Copy, Debug, Ord, PartialOrd, Hash, PartialEq, Eq, DieselNewType, Serialize)]
pub struct TaskId(Uuid);

impl Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
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

#[derive(Clone, Debug, Hash, PartialEq, Eq, DieselNewType, Serialize)]
pub struct TaskHash(Cow<'static, str>);

impl TaskHash {
    pub fn new<T: Into<String>>(hash: T) -> Self {
        TaskHash(Cow::Owned(hash.into()))
    }
}

impl<'a> From<&'a TaskHash> for &'a str {
    fn from(value: &'a TaskHash) -> Self {
        &value.0
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
#[cfg_attr(
    feature = "async_postgres",
    derive(diesel::Queryable, diesel::Identifiable)
)]
#[cfg_attr(feature = "async_postgres", diesel(table_name = backie_tasks))]
pub struct Task {
    /// Unique identifier of the task.
    pub id: TaskId,

    /// Name of the type of task.
    pub task_name: String,

    /// Queue name that the task belongs to.
    pub queue_name: String,

    /// Unique hash is used to identify and avoid duplicate tasks.
    pub uniq_hash: Option<TaskHash>,

    /// Representation of the task.
    pub payload: serde_json::Value,

    /// Max timeout that the task can run for.
    pub timeout_msecs: i64,

    /// Creation time of the task.
    pub created_at: DateTime<Utc>,

    /// Date time when the task is scheduled to run.
    pub scheduled_at: DateTime<Utc>,

    /// Date time when the task is started to run.
    pub running_at: Option<DateTime<Utc>>,

    /// Date time when the task is finished.
    pub done_at: Option<DateTime<Utc>>,

    /// Failure reason, when the task is failed.
    pub error_info: Option<serde_json::Value>,

    /// Number of times a task was retried.
    pub retries: i32,

    /// Maximum number of retries allow for this task before it is maked as failure.
    pub max_retries: i32,

    /// Backoff mode for this task.
    pub backoff_mode: serde_json::Value, // We use JSON to allow the backoff mode to be changed in the future and maybe hold configuration values
}

impl Task {
    pub fn state(&self) -> TaskState {
        if self.done_at.is_some() {
            if self.error_info.is_some() {
                // TODO: use a proper error type
                TaskState::Failed(self.error_info.clone().unwrap().to_string())
            } else {
                TaskState::Done
            }
        } else if self.running_at.is_some() {
            TaskState::Running
        } else {
            TaskState::Ready
        }
    }

    pub fn backoff_mode(&self) -> BackoffMode {
        serde_json::from_value(self.backoff_mode.clone()).expect("Invalid backoff mode")
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
#[cfg_attr(feature = "async_postgres", derive(diesel::Insertable))]
#[cfg_attr(feature = "async_postgres", diesel(table_name = backie_tasks))]
pub struct NewTask {
    task_name: String,
    queue_name: String,
    uniq_hash: Option<TaskHash>,
    payload: serde_json::Value,
    timeout_msecs: i64,
    max_retries: i32,
    backoff_mode: serde_json::Value,
}

impl NewTask {
    #[cfg(feature = "async_postgres")]
    pub(crate) fn with_timeout<T>(
        background_task: T,
        timeout: std::time::Duration,
    ) -> Result<Self, serde_json::Error>
    where
        T: crate::BackgroundTask,
    {
        let uniq_hash = background_task.uniq();
        let payload = serde_json::to_value(background_task)?;

        Ok(Self {
            task_name: T::TASK_NAME.to_string(),
            queue_name: T::QUEUE.to_string(),
            uniq_hash,
            payload,
            timeout_msecs: timeout.as_millis() as i64,
            max_retries: T::MAX_RETRIES,
            backoff_mode: serde_json::to_value(T::BACKOFF_MODE)?,
        })
    }

    #[cfg(feature = "async_postgres")]
    pub(crate) fn new<T>(background_task: T) -> Result<Self, serde_json::Error>
    where
        T: crate::BackgroundTask,
    {
        use std::time::Duration;

        Self::with_timeout(background_task, Duration::from_secs(120))
    }
}

#[cfg(test)]
impl From<NewTask> for Task {
    fn from(new_task: NewTask) -> Self {
        Self {
            id: TaskId(Uuid::new_v4()),
            task_name: new_task.task_name,
            queue_name: new_task.queue_name,
            uniq_hash: new_task.uniq_hash,
            payload: new_task.payload,
            timeout_msecs: new_task.timeout_msecs,
            created_at: Utc::now(),
            scheduled_at: Utc::now(),
            running_at: None,
            done_at: None,
            error_info: None,
            retries: 0,
            max_retries: new_task.max_retries,
            backoff_mode: new_task.backoff_mode,
        }
    }
}

pub struct CurrentTask {
    id: TaskId,
    retries: i32,
    created_at: DateTime<Utc>,
}

impl CurrentTask {
    pub(crate) fn new(task: &Task) -> Self {
        Self {
            id: task.id,
            retries: task.retries,
            created_at: task.created_at,
        }
    }

    pub fn id(&self) -> TaskId {
        self.id
    }

    pub fn retry_count(&self) -> i32 {
        self.retries
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}
