use crate::error::Result;
use crate::types::{ChangeEvent, ChangeType, ToolDefinition};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
// use std::sync::Arc;
use tokio::sync::broadcast;

pub struct ToolRegistry {
    tools: HashMap<String, ToolDefinition>,
    change_tx: broadcast::Sender<ChangeEvent>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(100);
        Self {
            tools: HashMap::new(),
            change_tx: tx,
        }
    }

    pub fn add_tool(&mut self, tool: ToolDefinition) -> Result<()> {
        let display_name = tool.name.clone();
        let is_new = !self.tools.contains_key(&display_name);

        self.tools.insert(display_name.clone(), tool);

        if is_new {
            self.notify_change(ChangeType::Added, display_name)?;
        } else {
            self.notify_change(ChangeType::Modified, display_name)?;
        }

        Ok(())
    }

    pub fn remove_tool(&mut self, name: &str) -> Result<()> {
        if self.tools.remove(name).is_some() {
            self.notify_change(ChangeType::Removed, name.to_string())?;
        }
        Ok(())
    }

    pub fn get_tool(&self, name: &str) -> Option<&ToolDefinition> {
        self.tools.get(name)
    }

    pub fn list_tools(&self) -> Vec<&ToolDefinition> {
        self.tools.values().collect()
    }

    pub fn subscribe_changes(&self) -> broadcast::Receiver<ChangeEvent> {
        self.change_tx.subscribe()
    }

    pub fn compute_hash(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    fn notify_change(&self, change_type: ChangeType, tool_name: String) -> Result<()> {
        let event = ChangeEvent {
            change_type,
            tool_name,
            timestamp: std::time::SystemTime::now(),
        };

        // Ignore send errors if no receivers
        let _ = self.change_tx.send(event);
        Ok(())
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = ToolRegistry::new();
        assert!(registry.list_tools().is_empty());
    }

    #[test]
    fn test_hash_computation() {
        let hash1 = ToolRegistry::compute_hash("test content");
        let hash2 = ToolRegistry::compute_hash("test content");
        let hash3 = ToolRegistry::compute_hash("different content");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }
}
