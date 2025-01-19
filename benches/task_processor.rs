use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::time::Duration;
use anyhow::Result;

#[derive(Debug)]
pub struct BenchmarkConstraints {
    max_task_duration: Duration,
    min_throughput: f64,  // tasks per second
    max_memory_usage: usize, // in bytes
    max_queue_size: usize,
    max_error_rate: f64,  // percentage
}

#[derive(Debug)]
pub struct BenchmarkResult {
    total_duration: Duration,
    throughput: f64,
    peak_memory: usize,
    max_queue_len: usize,
    error_rate: f64,
    task_durations: Vec<Duration>,
}

pub async fn run_benchmark_suite(
    pool: &WorkerPool<SqliteTaskStore, MyApplicationContext>,
    constraints: &BenchmarkConstraints,
    num_tasks: usize,
) -> Result<BenchmarkResult> {
    let start = Instant::now();
    let initial_memory = get_memory_usage()?;
    let mut peak_memory = initial_memory;
    let mut max_queue_len = 0;
    let mut error_count = 0;
    let mut task_durations = Vec::with_capacity(num_tasks);

    let (tx, mut rx) = tokio::sync::mpsc::channel(num_tasks);

    for i in 0..num_tasks {
        let tx = tx.clone();
        let task_start = Instant::now();
        
        let task = MyTask::new(i as u16);
        task.enqueue::<SqliteTaskStore>(&mut pool.task_store.pool.acquire().await?).await?;

        tokio::spawn(async move {
            let duration = task_start.elapsed();
            tx.send((duration, None::<anyhow::Error>)).await.unwrap();
        });

        let queue_len = pool.task_store.queue_size("default").await?;
        max_queue_len = max_queue_len.max(queue_len);

        let current_memory = get_memory_usage()?;
        peak_memory = peak_memory.max(current_memory);
    }

    for _ in 0..num_tasks {
        if let Some((duration, error)) = rx.recv().await {
            if error.is_some() {
                error_count += 1;
            }
            task_durations.push(duration);
        }
    }

    let total_duration = start.elapsed();
    let throughput = num_tasks as f64 / total_duration.as_secs_f64();
    let error_rate = error_count as f64 / num_tasks as f64 * 100.0;

    Ok(BenchmarkResult {
        total_duration,
        throughput,
        peak_memory,
        max_queue_len,
        error_rate,
        task_durations,
    })
}

pub fn assert_benchmark_constraints(result: &BenchmarkResult, constraints: &BenchmarkConstraints) -> Result<()> {
    for (i, duration) in result.task_durations.iter().enumerate() {
        assert!(
            duration <= &constraints.max_task_duration,
            "Task {} exceeded maximum duration: {:?} > {:?}",
            i,
            duration,
            constraints.max_task_duration
        );
    }

    assert!(
        result.throughput >= constraints.min_throughput,
        "Throughput below minimum: {:.2} < {:.2} tasks/second",
        result.throughput,
        constraints.min_throughput
    );

    assert!(
        result.peak_memory <= constraints.max_memory_usage,
        "Memory usage exceeded maximum: {} > {} bytes",
        result.peak_memory,
        constraints.max_memory_usage
    );

    assert!(
        result.max_queue_len <= constraints.max_queue_size,
        "Queue size exceeded maximum: {} > {}",
        result.max_queue_len,
        constraints.max_queue_size
    );

    assert!(
        result.error_rate <= constraints.max_error_rate,
        "Error rate exceeded maximum: {:.2}% > {:.2}%",
        result.error_rate,
        constraints.max_error_rate
    );

    Ok(())
}

fn get_memory_usage() -> Result<usize> {
    Ok(std::process::Command::new("ps")
        .args(&["--no-headers", "-o", "rss", &format!("{}", std::process::id())])
        .output()?
        .stdout
        .iter()
        .filter(|b| b.is_ascii_digit())
        .fold(0, |acc, d| acc * 10 + (d - b'0') as usize) * 1024)
}

pub fn criterion_benchmark(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    c.bench_function("task_processor", |b| {
        b.iter(|| {
            rt.block_on(async {
                let task_store = SqliteTaskStore::create("sqlite::memory:").await.unwrap();
                let my_app_context = MyApplicationContext::new("Benchmark App", None);
                
                let pool = WorkerPool::new(task_store, move || my_app_context.clone())
                    .register_task_type::<MyTask>()
                    .build()
                    .unwrap();

                let constraints = BenchmarkConstraints {
                    max_task_duration: Duration::from_secs(5),
                    min_throughput: 100.0,
                    max_memory_usage: 512 * 1024 * 1024,
                    max_queue_size: 1000,
                    max_error_rate: 0.1,
                };

                black_box(run_benchmark_suite(&pool, &constraints, 100).await.unwrap())
            })
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

