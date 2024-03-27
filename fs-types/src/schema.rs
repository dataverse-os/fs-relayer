// @generated automatically by Diesel CLI.

diesel::table! {
	commit_proofs (tip) {
		tip -> Text,
		time -> Timestamptz,
		stream_id -> Text,
		model_id -> Text,
		index -> Int8,
		prev -> Nullable<Text>,
		hash -> Bytea,
		status -> Int8,
	}
}

diesel::table! {
	dapp_models (model_id) {
		model_id -> Text,
		app_id -> Uuid,
		encryptable -> Array<Nullable<Text>>,
		model_name -> Text,
	}
}

diesel::allow_tables_to_appear_in_same_query!(commit_proofs, dapp_models,);
