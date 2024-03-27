use ceramic_core::StreamId;
use std::path::PathBuf;

#[derive(Debug)]
pub enum IrohClientError {
	ModelOfStreamNotFoundError(StreamId),
	StreamNotInModel(StreamId, StreamId),
	TaskLoadingFailed(PathBuf),
	StreamNotFound(StreamId),
}

impl std::fmt::Display for IrohClientError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::ModelOfStreamNotFoundError(stream_id) => {
				write!(f, "model of stream `{}` not found", stream_id)
			}
			Self::StreamNotInModel(stream_id, model_id) => {
				write!(f, "stream `{}` not found in model {}", stream_id, model_id)
			}
			Self::TaskLoadingFailed(data_path) => write!(
				f,
				"Failed to load tasks database from {}",
				data_path.display()
			),
			Self::StreamNotFound(stream_id) => write!(f, "stream not found: {}", stream_id),
		}
	}
}

impl std::error::Error for IrohClientError {}
