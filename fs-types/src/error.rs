#[derive(Debug)]
pub enum FilePolicyError {
	AttemptToModifyProtectedFields,
	PatchValidationFailed
}

impl std::fmt::Display for FilePolicyError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::PatchValidationFailed => write!(f, "validate patch field"),
			Self::AttemptToModifyProtectedFields => write!(f, "attempt to modify protected fields"),
		}
	}
}

impl std::error::Error for FilePolicyError {}

pub struct IllegalError {
	pub code: i64,
	pub message: String,
}

impl IllegalError {
	pub fn new(code: i64, message: String) -> Self {
		Self { code, message }
	}
}

macro_rules! error {
	($name:ident, $code:expr, $message:expr) => {
		#[allow(non_snake_case, missing_docs, dead_code)]
		pub fn $name<T>() -> IllegalError {
			IllegalError::new($code, $message.to_string())
		}
	};
}

// basic error
error!(EmptyWallet, 0x0001, "The wallet cannot be empty");
error!(EmptyPKH, 0x0002, "The pkh cannot be empty");
error!(EmptyAppId, 0x0003, "The appId cannot be empty");
error!(EmptyModelName, 0x0004, "The modelName cannot be empty");
error!(EmptyStreamId, 0x0005, "The streamId cannot be empty");
error!(
	EmptyStreamContent,
	0x0006, "The streamContent cannot be empty"
);
error!(EmptyDocument, 0x0007, "The document cannot be empty");
error!(bERR_8, 0x0008, "The encryptedSymmetricKey cannot be empty");
error!(bERR_9, 0x0009, "The decryptionConditions cannot be empty");
error!(EmptyLitAuthSig, 0x000A, "The litAuthSig cannot be empty");
error!(EmptyFolderName, 0x000B, "The folderName cannot be empty");
error!(EmptyFolderId, 0x000C, "The folderId cannot be empty");
error!(EmptyMirrorId, 0x000D, "The mirrorId cannot be empty");
error!(EmptyFileId, 0x000E, "The fileId cannot be empty");
error!(
	EmptyStreamIdOrFileId,
	0x000F, "The streamId and the fileId cannot both be empty"
);
error!(EmptyAPIKey, 0x0010, "The API Key cannot be empty");
error!(
	EmptyAction,
	0x0011, "The action and actionType cannot be empty"
);
error!(EmptyRelationId, 0x0012, "The relationId cannot be empty");
error!(
	bERR_19,
	0x0013, "With isCommentEncrypted passed in, comment cannot be empty"
);
error!(bERR_20, 0x0014, "The dataUnionName cannot be empty");
error!(
	bERR_21,
	0x0015, "The contentType and actionType cannot be passed in together"
);
error!(bERR_22, 0x0016, "The dataUnionId cannot be empty");
error!(bERR_23, 0x0017, "The fileName cannot be empty");

// advance error
error!(ErrorNoPermission, 0x1001, "No permissions");
error!(ErrorIdentityNotConnected, 0x1002, "Identity not connected");
error!(ERR_3, 0x1003, "The ceramicClient is not initialized");
error!(ERR_4, 0x1004, "The ceramicClient is invalid");
error!(ERR_5, 0x1005, "Session not authorized");
error!(ERR_6, 0x1006, "Session expired");
error!(ERR_7, 0x1007, "No permission to update stream");
error!(ERR_8, 0x1008, "The folderType is not defined");
error!(
	ERR_9,
	0x1009, "Cannot generate DecryptionConditions in this folder or file type"
);
error!(
	ERR_10,
	0x100A, "The modelId was not found in the application registry"
);
error!(ERR_11, 0x100B, "Not all streams were updated successfully");
error!(ERR_12, 0x100C, "provider is not initialized");
error!(ERR_13, 0x100D, "not in the list of supported wallets");
error!(
	ERR_14,
	0x100E, "The fileType and the decryption condition do not match"
);
error!(ERR_15, 0x100F, "The wallet is not connected");
error!(ERR_16, 0x1010, "The wallet has no signer");
error!(
	ERR_17,
	0x1011, "Encrypted field does not conform to json format"
);
error!(
	ERR_18,
	0x1012, "Unlocking is not supported when the file type is not payable"
);
error!(ERR_19, 0x1013, "There is a contradiction between a and b");
error!(ERR_20, 0x1014, "The DApp does not exist");
error!(ERR_21, 0x1015, "Particle netWork is not connected");
error!(ERR_22, 0x1016, "Not in the pkh list");
error!(ERR_23, 0x1017, "Current pkh is invalid");
error!(ERR_24, 0x1017, "No permission to operate the modelId");
error!(ERR_25, 0x1018, "The fields cannot be modified");
error!(
	ERR_26,
	0x1019, "There is no valid identity under the application corresponding to this modelId"
);
error!(ERR_27, 0x101A, "Already monetized");
error!(ERR_28, 0x101B, "You must connect capability");
error!(
	ERR_29,
	0x101C, "Unrecognized chain ID. Try adding the chain from https://chainlist.org"
);
error!(
	ERR_30,
	0x101D, "Please mint PKP NFT at https://explorer.litprotocol.com/mint-pkp first."
);
error!(
	ERR_31,
	0x101E, "The cid has not been synchronized on IPFS yet"
);
error!(
	ERR_32,
	0x101E, "The fileBase64 cannot be empty, or the file was not correctly converted to base64"
);
error!(
	ERR_33,
	0x101F, "The folderId does not exsit or has been deleted"
);
error!(
	ERR_34,
	0x1020, "The fileId does not exsit or has been deleted"
);
error!(ERR_35, 0x1021, "Unable to move into the payable folder");
error!(ERR_36, 0x1022, "Unable to delete the payable folder");
error!(
	ERR_37,
	0x1023, "Unable to move files from the payable folder"
);
error!(
	ERR_38,
	0x1024, "Unable to remove files from the payable folder"
);
error!(ERR_39, 0x1025, "ContentId already exists");
error!(
	ERR_40,
	0x1026, "Cannot perform the same 'collect' or 'unlock' action on the same file"
);
error!(
	ERR_41,
	0x1027, "The relationId does not exsit or has been deleted or is not supported"
);
error!(ERR_42, 0x1028, "Already collected");
error!(ERR_43, 0x1029, "Not collected, unable to unlock");
error!(
	ERR_44,
	0x102A, "Unlocking time has not arrived, unable to unlock"
);
error!(ERR_45, 0x102B, "Already unlocked");
error!(
	ERR_46,
	0x102C, "Monetizing to a datatoken is not supported when the file is not an index file"
);
error!(
	ERR_47,
	0x102D, "Collecting is not supported when the file type is not payable"
);
error!(ERR_48, 0x102E, "Collecting is not supported when the file is not an index file or the folder is not an union folder");
error!(
	ERR_49,
	0x102F, "Unlocking is not supported when the file is not an index file or an action file"
);
error!(
	ERR_50,
	0x1030, "The dataUnionId does not exsit or has been deleted"
);
error!(
	ERR_51,
	0x1031, "Monetize an action file to a datatoken is not supported "
);
error!(
	ERR_52,
	0x1032, "The file type that the data union can store does not match the current file type"
);
error!(
	ERR_53,
	0x1033, "The file is not a datatoken, please collect the data union to which the file belongs"
);
error!(ERR_54, 0x1034, "Failed to retreive from IPFS");
error!(ERR_55, 0x1035, "Unmatched system call used");
error!(ERR_56, 0x1036, "The file is not a bare file");
error!(ERR_57, 0x1037, "Not unlock");
