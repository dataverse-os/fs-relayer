use int_enum::IntEnum;
use serde::{Deserialize, Serialize};

/// Error type for file operations.
#[repr(i32)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, IntEnum)]
pub enum Status {
	Validated = 1,
	None = 0,
	NakedStream = -1,
	CACAOExpired = -2,
	BrokenContent = -3,
	BrokenFolder = -4,
}

impl Default for Status {
	fn default() -> Self {
		Self::None
	}
}

impl Serialize for Status {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		serializer.serialize_i32(self.int_value())
	}
}

impl<'de> Deserialize<'de> for Status {
	fn deserialize<D>(deserializer: D) -> Result<Status, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		let value = i32::deserialize(deserializer)?;
		let result = Status::from_int(value);
		result.map_err(|err| serde::de::Error::custom(format!("{}", err)))
	}
}
