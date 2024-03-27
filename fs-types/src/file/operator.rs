use std::collections::HashMap;

use ceramic_http_client::{FilterQuery, OperationFilter};
use dataverse_ceramic::{event::EventsUploader, Ceramic, StreamId, StreamState, StreamsLoader};

use crate::file::errors::StreamFileError;

use super::index_file::IndexFile;

#[async_trait::async_trait]
pub trait StreamFileLoader: StreamsLoader + EventsUploader + Send + Sync {
	async fn load_index_file_by_content_id(
		&self,
		ceramic: &Ceramic,
		index_file_model_id: &StreamId,
		content_id: &String,
	) -> anyhow::Result<(StreamState, IndexFile)> {
		let stream_states = self
			.load_stream_states(ceramic, None, index_file_model_id)
			.await?;
		for state in stream_states {
			match serde_json::from_value::<IndexFile>(state.content.clone()) {
				Ok(index_file) => {
					if index_file.content_id == *content_id {
						return Ok((state, index_file));
					}
				}
				Err(err) => {
					let stream_id = state.stream_id()?.to_string();
					tracing::warn!(
						stream_id,
						content = state.content.to_string(),
						"filed parse stream as index_file: {}",
						err
					);
				}
			}
		}
		anyhow::bail!(StreamFileError::IndexFileWithIdNotFound(content_id.clone()))
	}
}

#[async_trait::async_trait]
impl StreamFileLoader for dataverse_ceramic::http::Client {
	async fn load_index_file_by_content_id(
		&self,
		ceramic: &Ceramic,
		model_id: &StreamId,
		content_id: &String,
	) -> anyhow::Result<(StreamState, IndexFile)> {
		let mut where_filter = HashMap::new();
		where_filter.insert(
			"contentId".to_string(),
			OperationFilter::EqualTo(content_id.clone().into()),
		);

		let query = Some(FilterQuery::Where(where_filter));
		let streams = self.query_model(ceramic, None, model_id, query).await?;
		if streams.len() != 1 {
			anyhow::bail!(StreamFileError::IndexFileNotFound)
		}

		let state = match streams.first() {
			Some(state) => state,
			_ => anyhow::bail!(StreamFileError::IndexFileWithIdNotFound(content_id.clone())),
		};
		Ok((
			state.clone(),
			serde_json::from_value::<IndexFile>(state.content.clone())?,
		))
	}
}
