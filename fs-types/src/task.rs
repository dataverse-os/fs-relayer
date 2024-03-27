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
