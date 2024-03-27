pub mod commit_id;
pub mod operator;
pub mod patch;
// pub mod stream_id;

use super::commit_id::CommitId;
use crate::event::{Event, VerifyOption};
use crate::types::stream_id::{StreamId, StreamIdType};
use crate::types::strings::MultiBase32String;
use ceramic_http_client::api::StateLog;
use int_enum::IntEnum;
use libipld::Cid;
pub use operator::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::str::FromStr;

#[repr(u64)]
#[derive(Copy, Clone, Debug, Eq, IntEnum, PartialEq)]
pub enum LogType {
    Genesis = 0,
    Signed = 1,
    Anchor = 2,
}

#[repr(u64)]
#[derive(Copy, Clone, Debug, Eq, IntEnum, PartialEq)]
pub enum SignatureStatus {
    GENESIS = 0,
    PARTIAL = 1,
    SIGNED = 2,
}

/// Current state of stream
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamState {
    /// Type of stream
    pub r#type: u64,
    /// Content of stream
    pub content: Value,
    /// Log of stream
    pub log: Vec<StateLog>,
    /// Metadata for stream
    pub metadata: Value,
    // pub metadata: Metadata,
    /// Signature for stream
    pub signature: i32,
    /// Anchor status for stream
    pub anchor_status: AnchorStatus,
    /// Anchor proof for stream
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor_proof: Option<AnchorProof>,
    /// Type of document
    pub doctype: String,
}

#[repr(u64)]
#[derive(Copy, Clone, Debug, Eq, IntEnum, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AnchorStatus {
    NotRequested = 0,
    Pending = 1,
    Processing = 2,
    Anchored = 3,
    Failed = 4,
    Replaced = 5,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnchorProof {
    pub chain_id: String,
    pub root: MultiBase32String,
    pub tx_hash: MultiBase32String,
    pub tx_type: Option<String>,
}

impl StreamState {
    pub async fn make(r#type: u64, events: Vec<Event>) -> anyhow::Result<Self> {
        let mut state = StreamState {
            r#type,
            ..Default::default()
        };

        for event in events {
            event.apply_to(&mut state).await?;
            let model = state.must_model()?;
            let opts = vec![
                VerifyOption::ResourceModelsContain(model.clone()),
                // cannot get anchor time (should get time of txHash from rpc)
                // VerifyOption::ExpirationTimeBefore(Utc::now()),
            ];
            event.verify_signature(opts)?;
        }
        Ok(state)
    }

    pub async fn make_from_map(
        stream_id: StreamId,
        tip: Cid,
        event_map: &HashMap<Cid, Event>,
    ) -> anyhow::Result<Self> {
        let mut state = StreamState {
            r#type: stream_id.r#type.int_value(),
            ..Default::default()
        };
        let mut tip = tip;
        loop {
            let event = match event_map.get(&tip) {
                Some(event) => event,
                None => anyhow::bail!("event {} not found", tip),
            };
            event.apply_to(&mut state).await?;
            match event.prev()? {
                Some(prev) => tip = prev,
                None => break,
            }
            let model = state.must_model()?;
            let opts = vec![
                VerifyOption::ResourceModelsContain(model.clone()),
                // cannot get anchor time (should get time of txHash from rpc)
                // VerifyOption::ExpirationTimeBefore(Utc::now()),
            ];
            event.verify_signature(opts)?;
        }
        Ok(state)
    }

    /// Get controllers for stream
    pub fn controllers(&self) -> Vec<String> {
        let mut controllers = vec![];
        if let Some(controllers_json) = self.metadata.get("controllers") {
            if let Some(controllers_vec) = controllers_json.as_array() {
                for controller in controllers_vec {
                    if let Some(controller_str) = controller.as_str() {
                        controllers.push(controller_str.to_string());
                    }
                }
            }
        }
        controllers
    }

    /// Get model id for stream
    pub fn model(&self) -> anyhow::Result<Option<StreamId>> {
        self.metadata
            .get("model")
            .map(|model| {
                let model = model.as_str().expect("model is not string");
                StreamId::from_str(model)
            })
            .transpose()
    }

    pub fn must_model(&self) -> anyhow::Result<StreamId> {
        self.model().and_then(|model| {
            model.ok_or_else(|| anyhow::anyhow!("model not found in metadata stream_id"))
        })
    }

    pub fn stream_id(&self) -> anyhow::Result<StreamId> {
        let cid = &self.log.first().expect("log is empty").cid;
        Ok(StreamId {
            r#type: StreamIdType::from_int(self.r#type)?,
            cid: Cid::from_str(cid.as_ref())?,
        })
    }

    pub fn commit_ids(&self) -> anyhow::Result<Vec<CommitId>> {
        let mut commit_ids = vec![];
        let stream_id = self.stream_id()?;
        for log in self.log.iter() {
            let commit_id = CommitId {
                stream_id: stream_id.clone(),
                tip: Cid::from_str(log.cid.as_ref())?,
            };
            commit_ids.push(commit_id);
        }
        Ok(commit_ids)
    }
}

impl Default for StreamState {
    fn default() -> Self {
        Self {
            r#type: 0,
            content: Default::default(),
            metadata: Default::default(),
            signature: 2,
            anchor_status: AnchorStatus::Pending,
            log: vec![],
            doctype: "MID".to_string(),
            anchor_proof: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::stream::StreamState;

    use super::*;

    #[test]
    fn test_serialize_anchor_status() {
        let status = AnchorStatus::Anchored;
        let status = serde_json::to_value(status);
        assert!(status.is_ok());
        let status = status.unwrap();
        assert_eq!(status, json!("ANCHORED"));
    }

    #[test]
    fn test_deserialize_anchor_status() {
        let status = json!("ANCHORED");
        let status = serde_json::from_value::<AnchorStatus>(status);
        assert!(status.is_ok());
        let status = status.unwrap();
        assert_eq!(status, AnchorStatus::Anchored);

        let status = "ANCHORED";
        let status =
            serde_json::from_value::<AnchorStatus>(serde_json::Value::String(status.to_string()));
        println!("{:?}", status);
        assert!(status.is_ok());
        let status = status.unwrap();
        assert_eq!(status, AnchorStatus::Anchored);
    }

    #[test]
    fn decode_anchor_proof() {
        let data = json!({
          "root": "bafyreiaxfjkme33rujt5wfajbl7r6pcdhjw4gfzwmxqe7xs4wf3dwvxdpy",
          "txHash": "bagjqcgzasq3bv55stn7sg6m6zhmfq2fhsdgt4sef4fwozianarbmemjmhu6q",
          "txType": "f(bytes32)",
          "chainId": "eip155:1"
        });
        let data = serde_json::from_value::<AnchorProof>(data);
        assert!(data.is_ok());
    }

    #[test]
    fn test_decode_from_json() {
        let data = json!({
          "type": 3,
          "content": {
            "comment": "eyJtaXJyb3JOYW1lIjoicG9zdCIsIm5vdGUiOiIiLCJ0YWdzIjpbXX0",
            "fileType": 0,
            "contentId": "kjzl6kcym7w8y5fhg4cl0xi8npfke3jmaaeeic9dwx64bgunnqa6amortdgdsym",
            "createdAt": "2023-04-04T07:25:14.877Z",
            "updatedAt": "2023-04-04T07:25:14.877Z",
            "appVersion": "0.2.0",
            "contentType": "kjzl6hvfrbw6cb1jfm9wiuqelthhvv3hzpb2urkbcwdum1g0ao2qygdj0qdqn5g"
          },
          "metadata": {
            "controllers": [
              "did:pkh:eip155:137:0x312eA852726E3A9f633A0377c0ea882086d66666"
            ],
            "model": "kjzl6hvfrbw6c763ubdhowzao0m4yp84cxzbfnlh4hdi5alqo4yrebmc0qpjdi5"
          },
          "signature": 2,
          "anchorStatus": "ANCHORED",
          "log": [
            {
              "cid": "bagcqcerayswtqarydm2rgeh37yir45ccvfkj3qhwhfmu4vdjjrtny5l4rpia",
              "type": 0,
              "expirationTime": 1681197855,
              "timestamp": 1680629255
            },
            {
              "cid": "bafyreid43i4yornrup5nuiiu5bavu3k5se4z7wrokwd2oznvanp27eo7xe",
              "type": 2,
              "timestamp": 1680629255
            }
          ],
          "anchorProof": {
            "root": "bafyreihtmj5y6lbm23uulkwddp2hdiw4frhe6ofiunoqqjkcxasvuxlbrq",
            "txHash": "bagjqcgzaxm7xafnnfsvocyf7sya7m5qm64jztmcvwykwn3q6uvyw52mj6iua",
            "txType": "f(bytes32)",
            "chainId": "eip155:1"
          },
          "doctype": "MID"
        });
        let data = serde_json::from_value::<StreamState>(data);

        assert!(data.is_ok());
        let data = data.unwrap();
        assert_eq!(
            data.controllers(),
            vec!["did:pkh:eip155:137:0x312eA852726E3A9f633A0377c0ea882086d66666"]
        );
        assert_eq!(
            data.must_model().unwrap().to_string(),
            "kjzl6hvfrbw6c763ubdhowzao0m4yp84cxzbfnlh4hdi5alqo4yrebmc0qpjdi5"
        );

        let stream_id = data.stream_id().unwrap().to_string();
        assert_eq!(
            stream_id,
            "kjzl6kcym7w8y9s94kcardbh5u0ao76bci07xnnxjw1ew3i4eackykj76uagqfk"
        );
        assert_eq!(
            data.commit_ids().unwrap(),
            vec![
                CommitId::from_str(
                    &stream_id,
                    "bagcqcerayswtqarydm2rgeh37yir45ccvfkj3qhwhfmu4vdjjrtny5l4rpia"
                )
                .unwrap(),
                CommitId::from_str(
                    &stream_id,
                    "bafyreid43i4yornrup5nuiiu5bavu3k5se4z7wrokwd2oznvanp27eo7xe"
                )
                .unwrap()
            ],
        );
    }
}
