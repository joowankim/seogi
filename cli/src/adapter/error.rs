use thiserror::Error;

#[derive(Debug, Error)]
pub enum AdapterError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Date parse error: {0}")]
    DateParse(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_error_database_display() {
        let sqlite_err = rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_ERROR),
            Some("test error".to_string()),
        );
        let err = AdapterError::Database(sqlite_err);
        assert!(format!("{err}").starts_with("Database error:"));
    }

    #[test]
    fn test_adapter_error_io_display() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
        let err = AdapterError::Io(io_err);
        assert_eq!(format!("{err}"), "IO error: not found");
    }

    #[test]
    fn test_adapter_error_from_rusqlite() {
        let sqlite_err = rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_ERROR),
            Some("conversion test".to_string()),
        );
        let err: AdapterError = sqlite_err.into();
        assert!(matches!(err, AdapterError::Database(_)));
    }

    #[test]
    fn test_adapter_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        let err: AdapterError = io_err.into();
        assert!(matches!(err, AdapterError::Io(_)));
    }
}
