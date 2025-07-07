use crate::error::{Error, Result};
use crate::notification::{Notification, NotificationSender};
use crate::parser::JustfileParser;
use crate::registry::ToolRegistry;
use crate::types::{JustTask, Parameter, ToolDefinition};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{mpsc, Mutex};
use tokio::time::sleep;
use tracing::{error, info, warn};

pub struct JustfileWatcher {
    registry: Arc<Mutex<ToolRegistry>>,
    parser: JustfileParser,
    watched_paths: Arc<Mutex<HashSet<PathBuf>>>,
    debounce_duration: Duration,
    notification_sender: Option<NotificationSender>,
    // Maps tool names to their source justfile paths
    tool_source_map: Arc<Mutex<HashMap<String, PathBuf>>>,
    // Maps justfile paths to their assigned names
    path_names: Arc<Mutex<HashMap<PathBuf, Option<String>>>>,
    // Whether we have multiple watch directories
    has_multiple_dirs: bool,
}

impl JustfileWatcher {
    pub fn new(registry: Arc<Mutex<ToolRegistry>>) -> Self {
        Self {
            registry,
            parser: JustfileParser::new().expect("Failed to create parser"),
            watched_paths: Arc::new(Mutex::new(HashSet::new())),
            debounce_duration: Duration::from_millis(500),
            notification_sender: None,
            tool_source_map: Arc::new(Mutex::new(HashMap::new())),
            path_names: Arc::new(Mutex::new(HashMap::new())),
            has_multiple_dirs: false,
        }
    }

    pub fn with_notification_sender(mut self, sender: NotificationSender) -> Self {
        self.notification_sender = Some(sender);
        self
    }
    
    pub async fn configure_names(&self, configs: &[(PathBuf, Option<String>)]) {
        let mut path_names = self.path_names.lock().await;
        for (path, name) in configs {
            // Store names for each watched directory
            if path.is_dir() {
                let justfile_path = path.join("justfile");
                path_names.insert(justfile_path.clone(), name.clone());
                
                let justfile_cap = path.join("Justfile");
                path_names.insert(justfile_cap, name.clone());
            } else {
                path_names.insert(path.clone(), name.clone());
            }
        }
    }
    
    pub fn set_multiple_dirs(&mut self, multiple: bool) {
        self.has_multiple_dirs = multiple;
    }

    pub async fn watch_paths(&self, paths: Vec<PathBuf>) -> Result<()> {
        let (tx, mut rx) = mpsc::channel(100);

        // Create watcher
        let mut watcher = RecommendedWatcher::new(
            move |res: std::result::Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    let _ = tx.blocking_send(event);
                }
            },
            Config::default(),
        )
        .map_err(|e| Error::Io(std::io::Error::other(e)))?;

        // Watch each path
        for path in paths {
            if path.exists() {
                if path.is_dir() {
                    watcher
                        .watch(&path, RecursiveMode::NonRecursive)
                        .map_err(|e| Error::Io(std::io::Error::other(e)))?;
                    info!("Watching directory: {}", path.display());

                    // Scan for existing justfiles in directory
                    let justfile_path = path.join("justfile");
                    if justfile_path.exists() {
                        self.parse_and_update_justfile(&justfile_path).await?;
                    }
                } else if path.file_name() == Some(std::ffi::OsStr::new("justfile")) {
                    let parent = path.parent().unwrap_or(Path::new("."));
                    watcher
                        .watch(parent, RecursiveMode::NonRecursive)
                        .map_err(|e| Error::Io(std::io::Error::other(e)))?;
                    info!("Watching justfile: {}", path.display());

                    // Parse the justfile
                    self.parse_and_update_justfile(&path).await?;
                }

                self.watched_paths.lock().await.insert(path.clone());
            }
        }

        // Handle events with debouncing
        let mut pending_updates = HashSet::new();
        let debounce_duration = self.debounce_duration;

        loop {
            tokio::select! {
                Some(event) = rx.recv() => {
                    if let Some(path) = self.extract_justfile_path(&event) {
                        pending_updates.insert(path);
                    }
                }
                _ = sleep(debounce_duration) => {
                    if !pending_updates.is_empty() {
                        let updates = pending_updates.drain().collect::<Vec<_>>();
                        for path in updates {
                            if let Err(e) = self.handle_justfile_change(&path).await {
                                error!("Error handling justfile change: {}", e);
                            }
                        }
                    }
                }
            }
        }
    }

    fn extract_justfile_path(&self, event: &Event) -> Option<PathBuf> {
        match &event.kind {
            EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => event
                .paths
                .iter()
                .find(|p| p.file_name() == Some(std::ffi::OsStr::new("justfile")))
                .cloned(),
            _ => None,
        }
    }

    async fn handle_justfile_change(&self, path: &Path) -> Result<()> {
        match path.try_exists() {
            Ok(true) => {
                info!("Justfile modified: {}", path.display());
                self.parse_and_update_justfile(path).await?;
            }
            Ok(false) => {
                info!("Justfile removed: {}", path.display());
                self.remove_justfile_tools(path).await?;
            }
            Err(e) => {
                warn!("Error checking justfile existence: {}", e);
            }
        }
        Ok(())
    }

    pub async fn parse_and_update_justfile(&self, path: &Path) -> Result<()> {
        let content = std::fs::read_to_string(path)?;
        let hash = ToolRegistry::compute_hash(&content);
        let tasks = self.parser.parse_file(path)?;

        let mut registry = self.registry.lock().await;
        let mut tool_map = self.tool_source_map.lock().await;

        // First, remove all tools from this justfile
        let tools_to_remove: Vec<String> = tool_map
            .iter()
            .filter(|(_, source_path)| source_path == &path)
            .map(|(name, _)| name.clone())
            .collect();

        for tool_name in &tools_to_remove {
            registry.remove_tool(tool_name)?;
            tool_map.remove(tool_name);
        }

        // Track which tools we've seen
        let mut seen_tools = HashSet::new();

        // Add or update tools from parsed tasks
        for task in tasks {
            let tool = self.task_to_tool(task, &hash, path).await?;
            let tool_name = tool.name.clone();
            seen_tools.insert(tool_name.clone());
            
            // Track the source path
            tool_map.insert(tool_name.clone(), path.to_path_buf());
            
            registry.add_tool(tool)?;
        }

        // Send notification if we made any changes
        if !seen_tools.is_empty() || !tools_to_remove.is_empty() {
            if let Some(ref sender) = self.notification_sender {
                let _ = sender.send(Notification::ToolsListChanged);
            }
        }

        Ok(())
    }

    async fn remove_justfile_tools(&self, path: &Path) -> Result<()> {
        let mut registry = self.registry.lock().await;
        let mut tool_map = self.tool_source_map.lock().await;

        // Find tools from this justfile using our map
        let tools_to_remove: Vec<String> = tool_map
            .iter()
            .filter(|(_, source_path)| source_path == &path)
            .map(|(name, _)| name.clone())
            .collect();

        let had_removals = !tools_to_remove.is_empty();
        for tool_name in &tools_to_remove {
            registry.remove_tool(tool_name)?;
            tool_map.remove(tool_name);
        }

        // Send notification if we removed tools
        if had_removals {
            if let Some(ref sender) = self.notification_sender {
                let _ = sender.send(Notification::ToolsListChanged);
            }
        }

        Ok(())
    }

    async fn task_to_tool(&self, task: JustTask, hash: &str, path: &Path) -> Result<ToolDefinition> {
        // Get the configured name for this path
        let path_names = self.path_names.lock().await;
        let configured_name = path_names.get(path).and_then(|n| n.as_ref());
        
        // Always create the internal name with full path for execution
        let internal_name = format!("just_{}_{}", task.name, path.display());
        
        // Build the display name based on configuration
        let display_name = if self.has_multiple_dirs {
            // Multiple directories: add @name suffix if we have a name
            if let Some(name) = configured_name {
                format!("just_{}@{}", task.name, name)
            } else {
                // Fall back to full path if no name configured
                format!("just_{}_{}", task.name, path.display())
            }
        } else {
            // Single directory: no suffix in the display name
            format!("just_{}", task.name)
        };

        // Generate description from comments or use default
        let description = if task.comments.is_empty() {
            if self.has_multiple_dirs {
                if let Some(name) = configured_name {
                    format!("Execute '{}' task from {}", task.name, name)
                } else {
                    format!("Execute '{}' task", task.name)
                }
            } else {
                format!("Execute '{}' task", task.name)
            }
        } else {
            task.comments.join(". ")
        };

        // Generate JSON schema for parameters
        let input_schema = self.generate_input_schema(&task.parameters);

        Ok(ToolDefinition {
            name: display_name,
            description,
            input_schema,
            dependencies: task.dependencies,
            source_hash: hash.to_string(),
            last_modified: SystemTime::now(),
            internal_name: Some(internal_name),
        })
    }

    fn generate_input_schema(&self, parameters: &[Parameter]) -> serde_json::Value {
        let mut properties = serde_json::Map::new();
        let mut required = Vec::new();

        for param in parameters {
            let mut param_schema = serde_json::Map::new();
            param_schema.insert("type".to_string(), json!("string"));

            if let Some(desc) = &param.description {
                param_schema.insert("description".to_string(), json!(desc));
            }

            if let Some(default) = &param.default {
                param_schema.insert("default".to_string(), json!(default));
            } else {
                required.push(param.name.clone());
            }

            properties.insert(param.name.clone(), json!(param_schema));
        }

        json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": properties,
            "required": required,
            "additionalProperties": false
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_watcher_creation() {
        let registry = Arc::new(Mutex::new(ToolRegistry::new()));
        let watcher = JustfileWatcher::new(registry);
        assert_eq!(watcher.debounce_duration, Duration::from_millis(500));
    }

    #[tokio::test]
    async fn test_parse_and_update_justfile() {
        let registry = Arc::new(Mutex::new(ToolRegistry::new()));
        let watcher = JustfileWatcher::new(registry.clone());

        let temp_dir = TempDir::new().unwrap();
        let justfile_path = temp_dir.path().join("justfile");

        let content = r#"
# Test task
test:
    echo "Running tests"
"#;
        fs::write(&justfile_path, content).unwrap();

        watcher
            .parse_and_update_justfile(&justfile_path)
            .await
            .unwrap();

        let reg = registry.lock().await;
        let tools = reg.list_tools();
        assert_eq!(tools.len(), 1);
        assert!(tools[0].name.contains("just_test"));
    }

    #[tokio::test]
    async fn test_task_to_tool() {
        let registry = Arc::new(Mutex::new(ToolRegistry::new()));
        let watcher = JustfileWatcher::new(registry);

        let task = JustTask {
            name: "test".to_string(),
            body: "echo test".to_string(),
            parameters: vec![
                Parameter {
                    name: "arg1".to_string(),
                    default: None,
                    description: Some("First argument".to_string()),
                },
                Parameter {
                    name: "arg2".to_string(),
                    default: Some("default".to_string()),
                    description: None,
                },
            ],
            dependencies: vec!["dep1".to_string()],
            comments: vec!["Test task".to_string()],
            line_number: 1,
        };

        let tool = watcher
            .task_to_tool(task, "hash123", Path::new("justfile"))
            .await
            .unwrap();

        assert_eq!(tool.description, "Test task");
        assert_eq!(tool.dependencies, vec!["dep1"]);
        assert_eq!(tool.source_hash, "hash123");

        let schema = tool.input_schema.as_object().unwrap();
        let properties = schema["properties"].as_object().unwrap();
        assert_eq!(properties.len(), 2);

        let required = schema["required"].as_array().unwrap();
        assert_eq!(required.len(), 1);
        assert_eq!(required[0], "arg1");
    }
}
