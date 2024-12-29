-- Drop the indexes explicitly
DROP INDEX IF EXISTS idx_backie_tasks_queue_state;

DROP INDEX IF EXISTS idx_backie_tasks_state;

DROP INDEX IF EXISTS idx_backie_tasks_scheduled;

-- Add down migration script here
DROP TABLE IF EXISTS backie_tasks;
