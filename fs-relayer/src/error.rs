use ceramic_box::StreamId;
use uuid::Uuid;

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