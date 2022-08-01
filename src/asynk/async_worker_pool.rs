use crate::asynk::async_queue::AsyncQueue;
use crate::asynk::async_queue::AsyncQueueable;
use crate::asynk::async_worker::AsyncWorker;
use crate::asynk::Error;
use crate::{RetentionMode, SleepParams};
use async_recursion::async_recursion;
use bb8_postgres::tokio_postgres::tls::MakeTlsConnect;
use bb8_postgres::tokio_postgres::tls::TlsConnect;
use bb8_postgres::tokio_postgres::Socket;
use log::error;
use std::time::Duration;
use typed_builder::TypedBuilder;

#[derive(TypedBuilder, Clone)]
pub struct AsyncWorkerPool<Tls>
where
    Tls: MakeTlsConnect<Socket> + Clone + Send + Sync + 'static,
    <Tls as MakeTlsConnect<Socket>>::Stream: Send + Sync,
    <Tls as MakeTlsConnect<Socket>>::TlsConnect: Send,
    <<Tls as MakeTlsConnect<Socket>>::TlsConnect as TlsConnect<Socket>>::Future: Send,
{
    #[builder(setter(into))]
    pub queue: AsyncQueue<Tls>,
    #[builder(default, setter(into))]
    pub sleep_params: SleepParams,
    #[builder(default, setter(into))]
    pub retention_mode: RetentionMode,
    #[builder(setter(into))]
    pub number_of_workers: u32,
}

#[derive(TypedBuilder, Clone)]
pub struct WorkerParams {
    #[builder(setter(into, strip_option), default)]
    pub retention_mode: Option<RetentionMode>,
    #[builder(setter(into, strip_option), default)]
    pub sleep_params: Option<SleepParams>,
    #[builder(setter(into, strip_option), default)]
    pub task_type: Option<String>,
}

impl<Tls> AsyncWorkerPool<Tls>
where
    Tls: MakeTlsConnect<Socket> + Clone + Send + Sync + 'static,
    <Tls as MakeTlsConnect<Socket>>::Stream: Send + Sync,
    <Tls as MakeTlsConnect<Socket>>::TlsConnect: Send,
    <<Tls as MakeTlsConnect<Socket>>::TlsConnect as TlsConnect<Socket>>::Future: Send,
{
    pub async fn start(&mut self) {
        for _idx in 0..self.number_of_workers {
            let queue = self.queue.clone();
            let sleep_params = self.sleep_params.clone();
            let retention_mode = self.retention_mode.clone();

            tokio::spawn(async move {
                Self::supervise_worker(queue, sleep_params, retention_mode).await
            });
        }
    }

    #[async_recursion]
    pub async fn supervise_worker(
        queue: AsyncQueue<Tls>,
        sleep_params: SleepParams,
        retention_mode: RetentionMode,
    ) -> Result<(), Error> {
        let result =
            Self::run_worker(queue.clone(), sleep_params.clone(), retention_mode.clone()).await;

        tokio::time::sleep(Duration::from_secs(1)).await;

        match result {
            Err(err) => {
                error!("Worker failed. Restarting. {:?}", err);
                Self::supervise_worker(queue, sleep_params, retention_mode).await
            }
            Ok(_) => {
                error!("Worker stopped. Restarting");
                Self::supervise_worker(queue, sleep_params, retention_mode).await
            }
        }
    }

    pub async fn run_worker(
        mut queue: AsyncQueue<Tls>,
        sleep_params: SleepParams,
        retention_mode: RetentionMode,
    ) -> Result<(), Error> {
        let mut worker = AsyncWorker::builder()
            .queue(&mut queue as &mut dyn AsyncQueueable)
            .sleep_params(sleep_params)
            .retention_mode(retention_mode)
            .build();

        worker.run_tasks().await
    }
}
