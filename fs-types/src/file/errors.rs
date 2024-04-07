#[derive(Debug)]
pub enum FileError {
    NoControllerError,
    IndexFileWithIdNotFound(String),
    IndexFileNotFound,
}

impl std::fmt::Display for FileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoControllerError => write!(f, "No Controller"),
            Self::IndexFileWithIdNotFound(content_id) => {
                write!(f, "index file with contentId {} not found", content_id)
            }
            Self::IndexFileNotFound => write!(f, "Index file not found"),
        }
    }
}

impl std::error::Error for FileError {}

#[derive(Debug)]
pub enum IndexFileError {
    FileTypeUnchangeable,
    LinkedModelNotInApp,
}

impl std::fmt::Display for IndexFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FileTypeUnchangeable => write!(f, "file type cannot be changed"),
            Self::LinkedModelNotInApp => write!(f, "linked model not in same app"),
        }
    }
}

impl std::error::Error for IndexFileError {}

#[derive(Debug)]
pub enum IndexFolderError {
    AccessControlMissing,
}

impl std::fmt::Display for IndexFolderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AccessControlMissing => write!(f, "access control is missing for folder"),
        }
    }
}

impl std::error::Error for IndexFolderError {}
