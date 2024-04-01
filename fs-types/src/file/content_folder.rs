pub struct ContentFolder {
    pub fs_version: String,
    pub index_folder_id: String,
    pub mirror_file_ids: Vec<String>,
    pub encrypted_file_keys: Option<String>,
    pub reserved: Option<String>,
}
