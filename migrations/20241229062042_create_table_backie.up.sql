-- Add up migration script here
CREATE TABLE backie_tasks (
  id TEXT PRIMARY KEY NOT NULL,
  task_name TEXT NOT NULL,
  queue_name TEXT NOT NULL,
  uniq_hash TEXT,
  payload TEXT NOT NULL,
  timeout_msecs INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  scheduled_at INTEGER NOT NULL,
  running_at INTEGER,
  done_at INTEGER,
  error_info TEXT,
  retries INTEGER NOT NULL DEFAULT 0,
  max_retries INTEGER NOT NULL,
  backoff_mode TEXT NOT NULL
);

CREATE INDEX idx_backie_tasks_scheduled ON backie_tasks (scheduled_at);

CREATE INDEX idx_backie_tasks_task_name ON backie_tasks (task_name);

CREATE INDEX idx_backie_tasks_queue_name ON backie_tasks (queue_name);

CREATE INDEX idx_backie_tasks_task_queue ON backie_tasks (task_name, queue_name);
