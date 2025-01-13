use async_trait::async_trait;
use foo::{BackgroundTask, CurrentTask, QueueConfig, RetentionMode};
use foo::{SqliteTaskStore, WorkerPool};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

#[derive(Clone, Debug)]
pub struct MyApplicationContext {
	app_name: String,
	notify_finished: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
}

impl MyApplicationContext {
	pub fn new(app_name: &str, notify_finished: tokio::sync::oneshot::Sender<()>) -> Self {
		Self {
			app_name: app_name.to_string(),
			notify_finished: Arc::new(Mutex::new(Some(notify_finished))),
		}
	}

	pub async fn notify_finished(&self) {
		let mut lock = self.notify_finished.lock().await;
		if let Some(sender) = lock.take() {
			sender.send(()).unwrap();
		}
	}
}

#[derive(Serialize, Deserialize)]
pub struct MyTask {
	pub number: u16,
}

impl MyTask {
	pub fn new(number: u16) -> Self {
		Self { number }
	}
}

#[async_trait]
impl BackgroundTask for MyTask {
	const TASK_NAME: &'static str = "my_task";
	type AppData = MyApplicationContext;
	type Error = anyhow::Error;

	async fn run(&self, task: CurrentTask, ctx: Self::AppData) -> Result<(), Self::Error> {
		log::info!("[{}] Hello from {}! the current number is {}", task.id(), ctx.app_name, self.number);
		tokio::time::sleep(Duration::from_secs(3)).await;

		log::info!("[{}] done..", task.id());
		Ok(())
	}
}

#[derive(Serialize, Deserialize)]
pub struct MyFailingTask {
	pub number: u16,
}

impl MyFailingTask {
	pub fn new(number: u16) -> Self {
		Self { number }
	}
}

#[async_trait]
impl BackgroundTask for MyFailingTask {
	const TASK_NAME: &'static str = "my_failing_task";
	type AppData = MyApplicationContext;
	type Error = anyhow::Error;

	async fn run(&self, task: CurrentTask, _ctx: Self::AppData) -> Result<(), Self::Error> {
		log::info!("[{}] the current number is {}", task.id(), self.number);
		tokio::time::sleep(Duration::from_secs(3)).await;

		log::info!("[{}] done..", task.id());
		Ok(())
	}
}

#[derive(Serialize, Deserialize)]
struct EmptyTask {
	pub idx: u64,
}

#[async_trait]
impl BackgroundTask for EmptyTask {
	const TASK_NAME: &'static str = "empty_task";
	const QUEUE: &'static str = "loaded_queue";
	type AppData = MyApplicationContext;
	type Error = anyhow::Error;

	async fn run(&self, task: CurrentTask, _ctx: Self::AppData) -> Result<(), Self::Error> {
		log::info!("[{}] empty task done..", task.id());
		Ok(())
	}
}

#[derive(Serialize, Deserialize)]
struct FinalTask;

#[async_trait]
impl BackgroundTask for FinalTask {
	const TASK_NAME: &'static str = "final_task";
	const QUEUE: &'static str = "loaded_queue";
	type AppData = MyApplicationContext;
	type Error = anyhow::Error;

	async fn run(&self, _task: CurrentTask, ctx: Self::AppData) -> Result<(), Self::Error> {
		ctx.notify_finished().await;
		Ok(())
	}
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	env_logger::init();

	// Create SQLite database file and pool
	const STORAGE: &str = "/mnt/storage/users/dev/databases/backie";
	let database_url = format!("sqlite://{}/backie_tasks.db", STORAGE);
	let task_store = SqliteTaskStore::create(database_url.as_str()).await?;

	let (notify_finished, _) = tokio::sync::oneshot::channel();

	// Some global application context I want to pass to my background tasks
	let my_app_context = MyApplicationContext::new("Backie Example App", notify_finished);

	// Register the task types I want to use and start the worker pool
	let _ = WorkerPool::new(task_store, move || my_app_context.clone())
		.register_task_type::<MyTask>()
		.register_task_type::<MyFailingTask>()
		.register_task_type::<EmptyTask>()
		.register_task_type::<FinalTask>()
		.configure_queue("default".into())
		.configure_queue(
			QueueConfig::new("loaded_queue")
				.pull_interval(Duration::from_millis(100))
				.retention_mode(RetentionMode::RemoveDone)
				// Note: Reduced number of workers as SQLite doesn't handle as many concurrent connections as PostgreSQL
				.num_workers(50),
		);

	log::info!("Workers created ...");

	Ok(())
}
