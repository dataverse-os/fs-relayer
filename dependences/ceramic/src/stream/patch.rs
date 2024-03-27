use super::StreamState;

impl TryFrom<ceramic_http_client::api::StreamState> for StreamState {
	type Error = anyhow::Error;

	fn try_from(value: ceramic_http_client::api::StreamState) -> Result<Self, Self::Error> {
		let anchor_proof = value.anchor_proof.map(serde_json::from_value).transpose()?;
		let anchor_status = serde_json::from_value(serde_json::Value::String(value.anchor_status))?;

		Ok(Self {
			r#type: value.r#type,
			content: value.content,
			log: value.log,
			metadata: value.metadata,
			signature: value.signature,
			anchor_status,
			anchor_proof,
			doctype: value.doctype,
		})
	}
}
