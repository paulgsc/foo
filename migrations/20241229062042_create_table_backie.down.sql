-- Drop the indexes explicitly
DROP INDEX IF EXISTS idx_backie_tasks_scheduled;

DROP INDEX IF EXISTS idx_backie_tasks_task_name;

DROP INDEX IF EXISTS idx_backie_tasks_queue_name;

DROP INDEX IF EXISTS idx_backie_tasks_task_queue;

-- Add down migration script here
DROP TABLE IF EXISTS backie_tasks;
