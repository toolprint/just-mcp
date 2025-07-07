use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Execution error: {0}")]
    Execution(String),

    #[error("Registry error: {0}")]
    Registry(String),

    #[error("Server error: {0}")]
    Server(String),

    #[error("File watch error: {0}")]
    Watch(#[from] notify::Error),

    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),

    #[error("Invalid tool name: {0}")]
    InvalidToolName(String),

    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    #[error("Timeout error: {0}")]
    Timeout(String),

    #[error("Other error: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;
