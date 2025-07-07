use crate::error::{Error, Result};
use async_trait::async_trait;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[async_trait]
pub trait Transport: Send + Sync {
    async fn receive(&mut self) -> Result<Option<Value>>;
    async fn send(&mut self, message: Value) -> Result<()>;
}

pub struct StdioTransport {
    reader: BufReader<tokio::io::Stdin>,
    writer: tokio::io::Stdout,
}

impl StdioTransport {
    pub fn new() -> Self {
        Self {
            reader: BufReader::new(tokio::io::stdin()),
            writer: tokio::io::stdout(),
        }
    }
}

impl Default for StdioTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Transport for StdioTransport {
    async fn receive(&mut self) -> Result<Option<Value>> {
        let mut line = String::new();
        match self.reader.read_line(&mut line).await {
            Ok(0) => Ok(None),
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    return self.receive().await;
                }
                Ok(Some(serde_json::from_str(trimmed)?))
            }
            Err(e) => Err(Error::Io(e)),
        }
    }

    async fn send(&mut self, message: Value) -> Result<()> {
        let json = serde_json::to_string(&message)?;
        self.writer.write_all(json.as_bytes()).await?;
        self.writer.write_all(b"\n").await?;
        self.writer.flush().await?;
        Ok(())
    }
}
