use base64::{engine::general_purpose, Engine};

pub fn decode_base64(encoded: &str) -> anyhow::Result<Vec<u8>> {
	let decoded = match general_purpose::STANDARD.decode(encoded.as_bytes()) {
		Ok(decoded) => decoded,
		Err(_) => general_purpose::URL_SAFE_NO_PAD.decode(encoded.as_bytes())?,
	};
	Ok(decoded)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_decode_base64() {
		// base64 (js atob)
		assert!(decode_base64("e30=").is_ok());

		// base64 (js base64url)
		assert!(decode_base64("e30").is_ok())
	}
}
