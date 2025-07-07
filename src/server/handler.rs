use crate::error::Result;
use crate::registry::ToolRegistry;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct MessageHandler {
    #[allow(dead_code)]
    registry: Arc<RwLock<ToolRegistry>>,
}

impl MessageHandler {
    pub fn new(registry: Arc<RwLock<ToolRegistry>>) -> Self {
        Self { registry }
    }

    pub async fn handle(&self, _message: Value) -> Result<Option<Value>> {
        // TODO: Implement proper message handling
        // This will be completed in subtask 1.3
        Ok(None)
    }
}
