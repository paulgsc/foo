-- Add up migration script here
CREATE TABLE backie_tasks (
  id TEXT PRIMARY KEY NOT NULL,
  task_name TEXT NOT NULL,
  queue_name TEXT NOT NULL,
  uniq_has TEXT,
  payload TEXT NOT NULL,
  timeout_msecs INTEGER NOT NULL,
  state TEXT NOT NULL,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  scheduled_at TIMESTAMP NOT NULL,
  running_at TIMESTAMP,
  done_at TIMESTAMP,
  error_info TEXT,
  retries INTEGER NOT NULL DEFAULT 0,
  max_retries INTEGER NOT NULL,
  backoff_mode TEXT NOT NULL
);

CREATE INDEX idx_backie_tasks_queue_state ON backie_tasks (queue_name, state);

CREATE INDEX idx_backie_tasks_state ON backie_tasks (state);

CREATE INDEX idx_backie_tasks_scheduled ON backie_tasks (scheduled_at);
