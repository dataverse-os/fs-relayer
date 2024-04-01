use crate::config::Config;
use crate::fs_task;
use ceramic_box::kubo;
use fang::{AsyncQueue, AsyncWorkerPool};
use openssl::ssl::{SslConnector, SslMethod, SslVerifyMode};
use postgres_openssl::MakeTlsConnector;

pub type Queue = AsyncQueue<MakeTlsConnector>;

pub async fn new_queue(dsn: &str, max_pool_size: u32) -> anyhow::Result<Queue> {
    let mut queue = AsyncQueue::builder()
        .uri(dsn)
        // Max number of connections that are allowed
        .max_pool_size(max_pool_size)
        .build();

    let mut builder = SslConnector::builder(SslMethod::tls())?;
    builder.set_verify(SslVerifyMode::NONE);
    let connector = MakeTlsConnector::new(builder.build());
    queue.connect(connector).await?;
    Ok(queue)
}

pub fn build_pool(queue: Queue, num: u32) -> AsyncWorkerPool<AsyncQueue<MakeTlsConnector>> {
    AsyncWorkerPool::builder()
        .number_of_workers(num)
        .queue(queue)
        .build()
}

pub async fn task_queue(cfg: &Config) -> anyhow::Result<Queue> {
    // init kubo client for kubo task queue
    kubo::task::init_kubo(&cfg.kubo_path);
    let queue: fs_task::Queue = fs_task::new_queue(&cfg.queue_dsn, cfg.queue_pool).await?;
    let mut pool = fs_task::build_pool(queue.clone(), cfg.queue_worker);
    tracing::info!("starting queue");
    pool.start().await;
    return Ok(queue);
}
