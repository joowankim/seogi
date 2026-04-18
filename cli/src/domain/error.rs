use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_error_validation_display() {
        let err = DomainError::Validation("prefix must not be empty".to_string());
        assert_eq!(
            format!("{err}"),
            "Validation error: prefix must not be empty"
        );
    }
}
