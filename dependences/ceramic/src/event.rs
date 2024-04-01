pub mod anchor;
pub mod cacao;
pub mod commit;
pub mod errors;
pub mod ipld;
pub mod operator;
pub mod signed;
pub mod verify;

pub use self::anchor::*;
pub use self::ipld::*;
pub use self::operator::*;
pub use self::signed::*;
pub use self::verify::*;
use crate::stream::{LogType, StreamState};
use crate::types::jws::JwsWrap;
pub use crate::types::jws::ToCid;
use anyhow::{Context, Result};
use ceramic_http_client::api::StateLog;
use errors::EventError;
use libipld::cbor::DagCborCodec;
use libipld::prelude::Codec;
use libipld::Cid;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    pub cid: Cid,
    pub value: EventValue,
}

impl Event {
    pub fn genesis(&self) -> anyhow::Result<Cid> {
        match &self.value {
            EventValue::Signed(signed) => Ok(match signed.is_gensis() {
                true => self.cid,
                false => signed.payload()?.id.context(EventError::MissingId)?,
            }),
            EventValue::Anchor(anchor) => Ok(anchor.id),
        }
    }

    pub fn prev(&self) -> anyhow::Result<Option<Cid>> {
        match &self.value {
            EventValue::Signed(e) => Ok(e.payload()?.prev),
            EventValue::Anchor(e) => Ok(Some(e.prev)),
        }
    }

    pub fn log_type(&self) -> LogType {
        match &self.value {
            EventValue::Signed(signed) => match signed.is_gensis() {
                true => LogType::Genesis,
                false => LogType::Signed,
            },
            EventValue::Anchor(_) => LogType::Anchor,
        }
    }

    pub async fn apply_to(&self, state: &mut StreamState) -> anyhow::Result<()> {
        let prev_str = self.prev()?.map(|prev| prev.to_string());
        match (prev_str, &self.value) {
            // missing matching prev
            (Some(prev), _) => {
                let tip = state
                    .log
                    .last()
                    .context(EventError::MissingLastLog)?
                    .cid
                    .clone();
                if prev != tip {
                    {
                        anyhow::bail!(EventError::InvalidPreviousCid(prev, tip));
                    }
                }
            }
            // data event missing prev
            (None, EventValue::Signed(signed)) if !signed.is_gensis() => {
                anyhow::bail!(EventError::InvalidGenesisError)
            }
            // anchor event missing prev
            (None, EventValue::Anchor(_)) => anyhow::bail!("invalid genesis event"),
            _ => {}
        }
        let mut state_log = StateLog {
            cid: self.cid.to_string(),
            r#type: self.log_type() as u64,
            timestamp: None,
            expiration_time: None,
        };
        match &self.value {
            EventValue::Signed(signed) => {
                signed.apply_to(state)?;

                if let Some(cacao) = signed.cacao()? {
                    let exp = cacao.p.expiration_time()?;
                    state_log.expiration_time = exp.map(|x| x.timestamp());
                }
            }
            EventValue::Anchor(anchor) => {
                anchor.apply_to(state)?;

                // if let Some(proof) = anchor.proof()? {
                //     let timestamp = network::timestamp(proof).await?;
                //     state_log.timestamp = Some(timestamp);
                // };
            }
        };
        state.log.push(state_log);
        Ok(())
    }

    pub fn decode(cid: Cid, data: Vec<u8>) -> anyhow::Result<Self> {
        let codec = cid.codec();
        let value = EventValue::decode(codec, data)?;
        Ok(Event { cid, value })
    }
}

impl TryFrom<ceramic_http_client::api::Commit> for Event {
    type Error = anyhow::Error;

    fn try_from(value: ceramic_http_client::api::Commit) -> std::result::Result<Self, Self::Error> {
        match value.value {
            ceramic_http_client::api::CommitValue::Anchor(anchor) => Ok(Event {
                cid: value.cid.as_ref().try_into()?,
                value: EventValue::Anchor(Box::new(AnchorValue {
                    id: anchor.id.as_ref().try_into()?,
                    prev: anchor.prev.as_ref().try_into()?,
                    proof: anchor.proof.as_ref().try_into()?,
                    path: anchor.path,
                    proof_block: None,
                })),
            }),
            ceramic_http_client::api::CommitValue::Signed(signed) => Ok(Event {
                cid: value.cid.as_ref().try_into()?,
                value: EventValue::Signed(Box::new(SignedValue {
                    jws: signed.jws,
                    linked_block: Some(signed.linked_block.to_vec()?),
                    cacao_block: None,
                })),
            }),
        }
    }
}

impl TryFrom<ceramic_core::Jws> for Event {
    type Error = anyhow::Error;

    fn try_from(jws: ceramic_core::Jws) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            cid: jws.cid()?,
            value: EventValue::Signed(Box::new(SignedValue {
                jws,
                linked_block: None,
                cacao_block: None,
            })),
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum EventValue {
    Signed(Box<SignedValue>),
    Anchor(Box<AnchorValue>),
}

trait StreamStateApplyer {
    fn apply_to(&self, stream_state: &mut StreamState) -> anyhow::Result<()>;
}

impl EventValue {
    pub fn decode(codec: u64, data: Vec<u8>) -> Result<Self> {
        match codec {
            0x71 => Ok(EventValue::Anchor(Box::new(libipld::serde::from_ipld::<
                AnchorValue,
            >(
                DagCborCodec.decode(&data)?
            )?))),
            0x85 => Ok(EventValue::Signed(Box::new(data.try_into()?))),
            _ => anyhow::bail!(EventError::UnsupportedCodecError(codec)),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use libipld::Ipld;

    use super::*;

    #[test]
    fn decode_cacao_cap() -> anyhow::Result<()> {
        let genesis = crate::commit::example::genesis();
        let signed: SignedValue = genesis.genesis.try_into().unwrap();

        let cap = signed.cap();
        assert!(cap.is_ok());

        let cacao_cid = signed.cacao_link();
        assert!(cacao_cid.is_ok());
        assert_eq!(cap.unwrap(), cacao_cid.unwrap());

        Ok(())
    }

    #[test]
    fn test_decode_anchor_event() {
        // Test data
        let data = vec![
            164, 98, 105, 100, 216, 42, 88, 38, 0, 1, 133, 1, 18, 32, 254, 228, 97, 178, 152, 203,
            84, 252, 9, 177, 84, 148, 173, 107, 75, 252, 41, 230, 17, 181, 141, 244, 177, 224, 141,
            77, 198, 201, 12, 1, 141, 61, 100, 112, 97, 116, 104, 113, 48, 47, 48, 47, 48, 47, 49,
            47, 48, 47, 48, 47, 48, 47, 48, 47, 49, 100, 112, 114, 101, 118, 216, 42, 88, 38, 0, 1,
            133, 1, 18, 32, 44, 67, 74, 135, 93, 211, 163, 51, 90, 195, 121, 33, 70, 249, 27, 11,
            125, 248, 93, 7, 114, 194, 82, 233, 17, 100, 13, 7, 248, 44, 157, 233, 101, 112, 114,
            111, 111, 102, 216, 42, 88, 37, 0, 1, 113, 18, 32, 115, 27, 196, 150, 174, 107, 249,
            27, 43, 77, 252, 151, 46, 25, 88, 129, 238, 90, 58, 229, 7, 70, 5, 149, 238, 126, 123,
            225, 80, 136, 201, 125,
        ];

        let node: Ipld = DagCborCodec.decode(&data).unwrap();
        let result = libipld::serde::from_ipld::<AnchorValue>(node);
        assert!(result.is_ok());
        let result = result.unwrap();

        let expected = AnchorValue {
            id: Cid::from_str("bagcqcera73sgdmuyznkpycnrkskk222l7qu6menvrx2ldyenjxdmsdabru6q")
                .unwrap(),
            prev: Cid::from_str("bagcqcerafrbuvb252ortgwwdpequn6i3bn67qxiholbff2irmqgqp6bmtxuq")
                .unwrap(),
            proof: Cid::from_str("bafyreidtdpcjnltl7enswtp4s4xbsweb5zndvzihiyczl3t6ppqvbcgjpu")
                .unwrap(),
            path: "0/0/0/1/0/0/0/0/1".to_string(),
            proof_block: None,
        };

        assert_eq!(result, expected);
    }
}
