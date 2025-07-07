use crate::error::Result;
use crate::registry::ToolRegistry;
// use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;

pub mod handler;
pub mod protocol;
pub mod transport;

pub use transport::StdioTransport;

pub struct Server {
    #[allow(dead_code)]
    registry: Arc<RwLock<ToolRegistry>>,
    transport: Box<dyn transport::Transport>,
}

impl Server {
    pub fn new(transport: Box<dyn transport::Transport>) -> Self {
        Self {
            registry: Arc::new(RwLock::new(ToolRegistry::new())),
            transport,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        tracing::info!("Starting just-mcp server");

        loop {
            match self.transport.receive().await {
                Ok(Some(message)) => {
                    if let Err(e) = self.handle_message(message).await {
                        tracing::error!("Error handling message: {}", e);
                    }
                }
                Ok(None) => {
                    tracing::info!("Transport closed, shutting down");
                    break;
                }
                Err(e) => {
                    tracing::error!("Transport error: {}", e);
                    return Err(e);
                }
            }
        }

        Ok(())
    }

    async fn handle_message(&mut self, _message: Value) -> Result<()> {
        // TODO: Implement message handling
        // This will be implemented in subtask 1.3
        Ok(())
    }
}
