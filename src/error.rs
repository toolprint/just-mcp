use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Parse error at line {line}, column {column}: {message}")]
    Parse {
        message: String,
        line: usize,
        column: usize,
    },

    #[error("Execution error: command '{command}' failed with exit code {exit_code:?}: {stderr}")]
    Execution {
        command: String,
        exit_code: Option<i32>,
        stderr: String,
    },

    #[error("Just command error: {0}")]
    JustCommand(String),

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

    #[error("Task not found: {0}")]
    TaskNotFound(String),

    #[error("Timeout error: {0}")]
    Timeout(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Other error: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;
