pub mod errors;
pub mod models;
pub mod schema;

use anyhow::Context;
use dataverse_file_system::file::{IndexFile, StreamFileLoader};
use diesel::dsl::sql;
use diesel::sql_types::{Bool, Text};
use int_enum::IntEnum;
use std::collections::HashMap;
use std::sync::Arc;

use ceramic_core::{Cid, StreamId};
use dataverse_ceramic::{kubo, Ceramic, Event, EventsUploader, StreamState};
use dataverse_ceramic::{EventsLoader, StreamLoader, StreamOperator, StreamsLoader};
use dataverse_core::stream::{Stream, StreamStore};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use errors::{ConnectionPoolError, PgSqlClientError};

#[derive(Clone)]
pub struct Client {
	pub operator: Arc<dyn StreamOperator>,
	pub pool: Pool<ConnectionManager<PgConnection>>,
}

impl Client {
	pub fn new(operator: Arc<dyn StreamOperator>, dsn: &str) -> anyhow::Result<Self> {
		let manager = ConnectionManager::<PgConnection>::new(dsn);

		let pool = match Pool::builder().test_on_check_out(true).build(manager) {
			Ok(it) => it,
			Err(err) => {
				anyhow::bail!(ConnectionPoolError::PoolInitializationError(format!(
					"failed build connection pool: {}",
					err
				)));
			}
		};
		Ok(Self { operator, pool })
	}

	async fn load_events_from_db(
		&self,
		stream_id: &StreamId,
		mut tip: Option<Cid>,
	) -> anyhow::Result<Vec<Event>> {
		let conn = &mut self.pool.get()?;
		let events: Vec<models::Event> = schema::events::table
			.filter(schema::events::genesis.eq(stream_id.cid.to_string()))
			.select(models::Event::as_select())
			.load(conn)?;

		let mut map: HashMap<Cid, Event> = HashMap::new();
		for event in events {
			let event: Event = event.try_into()?;
			map.insert(event.cid, event);
		}

		let mut result = Vec::new();
		if tip.is_none() {
			while let Some(cid) = tip {
				let event = match map.get(&cid) {
					Some(event) => event,
					None => anyhow::bail!("missing event {} for stream {}", cid, stream_id),
				};
				result.push(event.clone());
				tip = event.prev()?;
			}
			result.reverse();
		} else {
			let mut prev_map: HashMap<Cid, Cid> = HashMap::new();
			for (cid, event) in &map {
				if let Some(prev) = event.prev()? {
					prev_map.insert(prev, *cid);
				}
			}
			let mut prev = stream_id.cid;
			let genesis = map.get(&prev).context(PgSqlClientError::MissingGenesis)?;
			result.push(genesis.clone());
			while let Some(cid) = prev_map.get(&prev) {
				let event = match map.get(cid) {
					Some(event) => event,
					None => anyhow::bail!(PgSqlClientError::MissingEventForStream(
						*cid,
						stream_id.clone()
					)),
				};
				result.push(event.clone());
				prev = *cid;
			}
		}

		Ok(result)
	}

	async fn save_events_to_db(&self, events: Vec<Event>) -> anyhow::Result<()> {
		let conn = &mut self.pool.get()?;
		for event in events {
			let event: models::Event = event.try_into()?;
			diesel::insert_into(schema::events::table)
				.values(&event)
				.on_conflict(schema::events::cid)
				.do_nothing()
				.execute(conn)?;
		}
		Ok(())
	}
}

#[async_trait::async_trait]
impl StreamStore for Client {
	async fn list_all_streams(&self) -> anyhow::Result<Vec<Stream>> {
		let conn = &mut self.pool.get()?;
		let streams: Vec<models::Stream> = schema::streams::table.load(conn)?;
		let mut result = Vec::new();
		for stream in streams {
			let stream = stream.try_into()?;
			result.push(stream);
		}
		Ok(result)
	}

	async fn save_stream(&self, stream: &Stream) -> anyhow::Result<()> {
		let stream: models::Stream = stream.try_into()?;
		let conn = &mut self.pool.get()?;
		let execute = diesel::insert_into(schema::streams::table)
			.values(&stream)
			.on_conflict(schema::streams::stream_id)
			.do_update()
			.set(&stream)
			.execute(conn);
		if let Err(err) = execute {
			tracing::error!(?stream, "db exec error: {}", err);
			anyhow::bail!(PgSqlClientError::DbExecError)
		}
		Ok(())
	}
	async fn load_stream(&self, stream_id: &StreamId) -> anyhow::Result<Option<Stream>> {
		let conn = &mut self.pool.get()?;
		let stream: Option<models::Stream> = schema::streams::table
			.filter(schema::streams::stream_id.eq(stream_id.to_string()))
			.first(conn)
			.optional()?;
		if let Some(stream) = stream {
			let stream = stream.try_into()?;
			return Ok(Some(stream));
		}
		Ok(None)
	}
}

#[async_trait::async_trait]
impl kubo::Store for Client {
	async fn get(
		&self,
		_id: Option<String>,
		stream_id: Option<StreamId>,
	) -> anyhow::Result<Option<Cid>> {
		if let Some(stream_id) = stream_id {
			let conn = &mut self.pool.get()?;
			let stream: Option<models::Stream> = schema::streams::table
				.filter(schema::streams::stream_id.eq(stream_id.to_string()))
				.first(conn)
				.optional()?;
			if let Some(stream) = stream {
				return Ok(Some(Cid::try_from(stream.tip.to_string())?));
			}
		}
		Ok(None)
	}

	async fn push(
		&self,
		_id: Option<String>,
		stream_id: Option<StreamId>,
		tip: Cid,
	) -> anyhow::Result<()> {
		if let Some(stream_id) = stream_id {
			let conn = &mut self.pool.get()?;
			let stream: Option<models::Stream> = schema::streams::table
				.filter(schema::streams::stream_id.eq(stream_id.to_string()))
				.first(conn)
				.optional()?;
			if let Some(mut stream) = stream {
				stream.tip = tip.to_string();
				diesel::insert_into(schema::streams::table)
					.values(&stream)
					.on_conflict(schema::streams::stream_id)
					.do_update()
					.set(&stream)
					.execute(conn)?;
			}
		}
		Ok(())
	}
}

#[async_trait::async_trait]
impl StreamLoader for Client {
	async fn load_stream_state(
		&self,
		ceramic: &Ceramic,
		stream_id: &StreamId,
		tip: Option<Cid>,
	) -> anyhow::Result<StreamState> {
		let tip = match tip {
			Some(tip) => tip,
			None => match self.load_stream(stream_id).await? {
				Some(stream) => stream.tip,
				None => anyhow::bail!("missing stream: {}", stream_id),
			},
		};
		let events = self.load_events(ceramic, stream_id, Some(tip)).await?;
		StreamState::make(stream_id.r#type.int_value(), events).await
	}
}

#[async_trait::async_trait]
impl StreamsLoader for Client {
	async fn load_stream_states(
		&self,
		_ceramic: &Ceramic,
		account: Option<String>,
		model_id: &StreamId,
	) -> anyhow::Result<Vec<StreamState>> {
		let conn = &mut self.pool.get()?;
		let model_id = model_id.to_string();
		let mut query = schema::streams::table.into_boxed();
		query = query.filter(schema::streams::model_id.eq(model_id));

		if let Some(account) = account {
			query = query.filter(schema::streams::account.eq(account));
		}

		let streams: Vec<models::Stream> = query.load(conn)?;
		let mut result = Vec::new();
		for stream in streams {
			let stream_id = stream.stream_id()?;
			let tip = Some(Cid::try_from(stream.tip.to_string())?);
			let commits: Vec<Event> = self.load_events(_ceramic, &stream_id, tip).await?;
			let state = StreamState::make(stream_id.r#type.int_value(), commits).await?;
			result.push(state);
		}
		return Ok(result);
	}
}

#[async_trait::async_trait]
impl EventsLoader for Client {
	async fn load_events(
		&self,
		ceramic: &Ceramic,
		stream_id: &StreamId,
		tip: Option<Cid>,
	) -> anyhow::Result<Vec<Event>> {
		match self.load_events_from_db(stream_id, tip).await {
			Ok(result) => Ok(result),
			Err(err) => {
				tracing::warn!(
					stream_id = stream_id.to_string(),
					"failed load events from db: {}",
					err
				);

				let result = self.operator.load_events(ceramic, stream_id, tip).await?;
				self.save_events_to_db(result.clone()).await?;
				Ok(result)
			}
		}
	}
}

#[async_trait::async_trait]
impl StreamFileLoader for Client {
	async fn load_index_file_by_content_id(
		&self,
		ceramic: &Ceramic,
		index_file_model_id: &StreamId,
		content_id: &String,
	) -> anyhow::Result<(StreamState, IndexFile)> {
		let conn = &mut self.pool.get()?;
		let stream: Result<Option<models::Stream>, _> = schema::streams::table
			.filter(schema::streams::model_id.eq(index_file_model_id.to_string()))
			.filter(sql::<Bool>("content->>'contentId' = ").bind::<Text, _>(content_id))
			.first(conn)
			.optional();

		let stream: Option<models::Stream> = match stream {
			Ok(stream) => stream,
			Err(err) => {
				tracing::warn!(
					model_id = index_file_model_id.to_string(),
					content_id = content_id,
					?err,
					"load index file by content_id sql error",
				);
				return Err(err.into());
			}
		};
		if let Some(stream) = stream {
			let stream_id = stream.stream_id()?;
			let tip: Cid = stream.tip.parse()?;
			let state = self
				.load_stream_state(ceramic, &stream_id, Some(tip))
				.await?;
			let index_file = serde_json::from_value::<IndexFile>(state.content.clone())?;
			return Ok((state, index_file));
		}
		anyhow::bail!("index file with content_id {} not found", content_id)
	}
}

#[async_trait::async_trait]
impl EventsUploader for Client {
	async fn upload_event(
		&self,
		ceramic: &Ceramic,
		stream_id: &StreamId,
		event: Event,
	) -> anyhow::Result<()> {
		self.save_events_to_db(vec![event.clone()]).await?;
		self.operator.upload_event(ceramic, stream_id, event).await
	}
}
