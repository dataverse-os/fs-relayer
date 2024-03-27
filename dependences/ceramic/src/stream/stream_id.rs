use int_enum::IntEnum;

#[derive(Debug)]
pub struct StreamIdType(ceramic_core::StreamIdType);

impl From<ceramic_core::StreamIdType> for StreamIdType {
	fn from(value: ceramic_core::StreamIdType) -> Self {
		StreamIdType(value)
	}
}

impl TryFrom<u64> for StreamIdType {
	type Error = anyhow::Error;

	fn try_from(value: u64) -> Result<Self, Self::Error> {
		Ok(StreamIdType(ceramic_core::StreamIdType::from_int(value)?))
	}
}

impl From<StreamIdType> for ceramic_core::StreamIdType {
	fn from(val: StreamIdType) -> Self {
		val.0
	}
}

impl From<StreamIdType> for u64 {
	fn from(_val: StreamIdType) -> Self {
		todo!()
	}
}
