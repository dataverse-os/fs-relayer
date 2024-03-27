use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ContentType {
	pub resource: ContentTypeResourceType,
	pub resource_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[derive(Default)]
pub enum ContentTypeResourceType {
	#[default]
 CERAMIC,
	WEAVEDB,
	IPFS,
}



#[cfg(test)]
mod tests {
	use serde_json::json;

	use super::*;

	#[test]
	fn deserialize_content_type() {
		let data = json!({
			"resource": "CERAMIC",
			"resourceId": "123"
		});

		let content_type: ContentType = serde_json::from_value(data).unwrap();

		assert_eq!(content_type.resource, ContentTypeResourceType::CERAMIC);
		assert_eq!(content_type.resource_id.unwrap(), "123");
	}
}
