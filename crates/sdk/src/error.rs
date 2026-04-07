use thiserror::Error;

#[derive(Error, Debug)]
pub enum SdkError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("API error ({code}): {message}")]
    Api { code: String, message: String },

    #[error("Authentication failed: {0}")]
    Auth(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("NPC not found: {0}")]
    NotFound(String),

    #[error("Provider not found: {0}")]
    ProviderNotFound(String),

    #[error("Timeout after {0}ms")]
    Timeout(u64),

    #[error("Connection refused: {0}")]
    ConnectionRefused(String),
}

impl SdkError {
    pub fn api_error(message: impl Into<String>, code: impl Into<String>) -> Self {
        Self::Api {
            message: message.into(),
            code: code.into(),
        }
    }
}

pub type SdkResult<T> = Result<T, SdkError>;
