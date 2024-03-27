use ceramic_core::StreamId;
use uuid::Uuid;

#[derive(Debug)]
pub enum StreamFileError {
	NoControllerError,
	IndexFileWithIdNotFound(String),
	IndexFileNotFound,
}

impl std::fmt::Display for StreamFileError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::NoControllerError => write!(f, "No Controller"),
			Self::IndexFileWithIdNotFound(content_id) => write!(f,"index file with contentId {} not found", content_id),
			Self::IndexFileNotFound => write!(f, "Index file not found"),
		}
	}
}

impl std::error::Error for StreamFileError {}


#[derive(Debug)]
pub enum FileClientError {
	StreamWithModelNotInDapp(StreamId,StreamId,Uuid),
	AnchorCommitUnsupported,
	NoPrevCommitFound,
	CommitStreamIdNotFoundOnStore(StreamId)
}

impl std::fmt::Display for FileClientError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::AnchorCommitUnsupported => write!(f, "anchor commit not supported"),
			Self::NoPrevCommitFound => write!(f,"donot have previous commit"),
			Self::CommitStreamIdNotFoundOnStore(stream_id) => write!(f, "publishing commit with stream_id {} not found in store", stream_id),
			Self::StreamWithModelNotInDapp(stream_id, model_id, dapp_id) => write!(f,"stream_id {} with model_id {} not belong to dapp {}", stream_id, model_id, dapp_id),
		}
	}
}

impl std::error::Error for FileClientError {}

#[derive(Debug)]
pub enum IndexFileError {
	FileTypeUnchangeable,
	LinkedModelNotInApp,
}

impl std::fmt::Display for IndexFileError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::FileTypeUnchangeable => write!(f, "file type cannot be changed"),
			Self::LinkedModelNotInApp => write!(f, "linked model not in same app"),
		}
	}
}

impl std::error::Error for IndexFileError {}

#[derive(Debug)]
pub enum IndexFolderError {
	AccessControlMissing,
}

impl std::fmt::Display for IndexFolderError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::AccessControlMissing => write!(f, "access control is missing for folder"),
		}
	}
}

impl std::error::Error for IndexFolderError {}
