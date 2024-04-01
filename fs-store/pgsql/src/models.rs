use std::str::FromStr;

use ceramic_box::{Cid, StreamId};
use ceramic_box::{
	event::{AnchorValue, SignedValue, ToCid},
	EventValue,
};
use diesel::prelude::*;
use int_enum::IntEnum;

use dataverse_file_types::core::stream as stream_core;
use crate::errors::PgSqlEventError;

#[derive(Debug, Queryable, Selectable, Insertable, AsChangeset)]
#[diesel(table_name = crate::schema::events)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Event {
	pub cid: String,
	pub prev: Option<String>,
	pub genesis: String,
	pub blocks: Vec<Option<Vec<u8>>>,
}

impl TryInto<ceramic_box::Event> for Event {
	type Error = anyhow::Error;

	fn try_into(self) -> anyhow::Result<ceramic_box::Event> {
		let cid = Cid::try_from(self.cid)?;
		let value = match cid.codec() {
			0x71 => {
				let anchor = self.blocks[0].clone().unwrap();
				let proof = self.blocks[1].clone();
				AnchorValue::try_from((anchor, proof))?.into()
			}
			0x85 => {
				let jws = self.blocks[0].clone().unwrap();
				let linked_block = self.blocks[1].clone();
				let cacao_block = self.blocks[2].clone();
				SignedValue::try_from((jws, linked_block, cacao_block))?.into()
			}
			_ => anyhow::bail!(PgSqlEventError::UnsupportedCodecError(cid.codec())),
		};

		Ok(ceramic_box::Event { cid, value })
	}
}

impl TryFrom<ceramic_box::Event> for Event {
	type Error = anyhow::Error;

	fn try_from(value: ceramic_box::Event) -> Result<Self, Self::Error> {
		let cid = value.genesis()?;
		let event = Event {
			cid: value.cid.to_string(),
			prev: value.prev()?.map(|x| x.to_string()),
			genesis: cid.to_string(),
			blocks: match value.value {
				EventValue::Signed(signed) => {
					let jws = signed.jws.to_vec()?;
					vec![Some(jws), signed.linked_block, signed.cacao_block]
				}
				EventValue::Anchor(anchor) => {
					let block = anchor.to_vec()?;
					vec![Some(block), anchor.proof_block]
				}
			},
		};
		Ok(event)
	}
}

#[derive(Debug, Clone, Queryable, Selectable, Insertable, AsChangeset)]
#[diesel(table_name = crate::schema::streams)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Stream {
	pub stream_id: String,
	pub dapp_id: uuid::Uuid,
	pub tip: String,
	pub account: Option<String>,
	pub model_id: Option<String>,
	pub content: serde_json::Value,
}

impl Stream {
	pub fn stream_id(&self) -> anyhow::Result<StreamId> {
		StreamId::from_str(&self.stream_id)
	}
}

impl TryFrom<&stream_core::Stream> for Stream {
	type Error = anyhow::Error;

	fn try_from(value: &stream_core::Stream) -> Result<Self, Self::Error> {
		Ok(Self {
			stream_id: value.stream_id()?.to_string(),
			dapp_id: value.dapp_id,
			tip: value.tip.to_string(),
			account: value.account.clone(),
			model_id: value.model.clone().map(|x| x.to_string()),
			content: value.content.clone(),
		})
	}
}

impl TryInto<stream_core::Stream> for Stream {
	type Error = anyhow::Error;

	fn try_into(self) -> Result<stream_core::Stream, Self::Error> {
		let model = match &self.model_id {
			Some(model) => Some(StreamId::from_str(model)?),
			None => None,
		};
		let stream_id = self.stream_id()?;
		Ok(stream_core::Stream {
			r#type: stream_id.r#type.int_value(),
			dapp_id: self.dapp_id,
			genesis: stream_id.cid,
			tip: Cid::try_from(self.tip)?,
			account: self.account,
			model,
			content: self.content,
		})
	}
}
