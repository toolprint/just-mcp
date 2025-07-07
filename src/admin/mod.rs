use crate::error::Result;
use crate::registry::ToolRegistry;
use crate::types::ToolDefinition;
use crate::watcher::JustfileWatcher;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};

pub struct AdminTools {
    registry: Arc<Mutex<ToolRegistry>>,
    watcher: Arc<JustfileWatcher>,
    watch_paths: Vec<PathBuf>,
}

impl AdminTools {
    pub fn new(
        registry: Arc<Mutex<ToolRegistry>>,
        watcher: Arc<JustfileWatcher>,
        watch_paths: Vec<PathBuf>,
    ) -> Self {
        Self {
            registry,
            watcher,
            watch_paths,
        }
    }

    pub async fn register_admin_tools(&self) -> Result<()> {
        let mut registry = self.registry.lock().await;

        // Register sync() tool
        let sync_tool = ToolDefinition {
            name: "just_admin_sync".to_string(),
            description: "Manually re-scan justfiles and update the tool registry".to_string(),
            input_schema: json!({
                "$schema": "http://json-schema.org/draft-07/schema#",
                "type": "object",
                "properties": {},
                "required": [],
                "additionalProperties": false
            }),
            dependencies: vec![],
            source_hash: "admin_tool_sync_v1".to_string(),
            last_modified: std::time::SystemTime::now(),
        };

        registry.add_tool(sync_tool)?;

        // TODO: Add create_task, modify_task, remove_task tools in future subtasks

        Ok(())
    }

    pub async fn sync(&self) -> Result<SyncResult> {
        info!("Starting manual justfile sync");

        let start_time = std::time::Instant::now();
        let mut scanned_files = 0;
        let mut found_tasks = 0;
        let mut errors = Vec::new();

        // Clear the registry cache
        {
            let mut registry = self.registry.lock().await;
            // Remove all non-admin tools
            let tools_to_remove: Vec<String> = registry
                .list_tools()
                .iter()
                .filter(|tool| !tool.name.starts_with("just_admin_"))
                .map(|tool| tool.name.clone())
                .collect();

            for tool_name in tools_to_remove {
                registry.remove_tool(&tool_name)?;
            }
        }

        // Re-scan all watch paths
        for path in &self.watch_paths {
            if path.exists() {
                if path.is_dir() {
                    // Scan for justfiles in directory
                    let justfile_path = path.join("justfile");
                    if justfile_path.exists() {
                        info!("Found justfile: {}", justfile_path.display());
                        match self.scan_justfile(&justfile_path).await {
                            Ok(task_count) => {
                                scanned_files += 1;
                                found_tasks += task_count;
                            }
                            Err(e) => {
                                warn!("Error scanning {}: {}", justfile_path.display(), e);
                                errors.push(format!("{}: {}", justfile_path.display(), e));
                            }
                        }
                    }

                    // Also check for capitalized Justfile
                    let justfile_cap = path.join("Justfile");
                    if justfile_cap.exists() {
                        info!("Found Justfile: {}", justfile_cap.display());
                        match self.scan_justfile(&justfile_cap).await {
                            Ok(task_count) => {
                                scanned_files += 1;
                                found_tasks += task_count;
                            }
                            Err(e) => {
                                warn!("Error scanning {}: {}", justfile_cap.display(), e);
                                errors.push(format!("{}: {}", justfile_cap.display(), e));
                            }
                        }
                    }
                } else if path.file_name() == Some(std::ffi::OsStr::new("justfile"))
                    || path.file_name() == Some(std::ffi::OsStr::new("Justfile"))
                {
                    // Direct justfile path
                    match self.scan_justfile(path).await {
                        Ok(task_count) => {
                            scanned_files += 1;
                            found_tasks += task_count;
                        }
                        Err(e) => {
                            warn!("Error scanning {}: {}", path.display(), e);
                            errors.push(format!("{}: {}", path.display(), e));
                        }
                    }
                }
            } else {
                warn!("Watch path does not exist: {}", path.display());
                errors.push(format!("Path not found: {}", path.display()));
            }
        }

        let duration = start_time.elapsed();

        info!(
            "Sync completed in {:?}: {} files scanned, {} tasks found, {} errors",
            duration,
            scanned_files,
            found_tasks,
            errors.len()
        );

        Ok(SyncResult {
            scanned_files,
            found_tasks,
            errors,
            duration_ms: duration.as_millis() as u64,
        })
    }

    async fn scan_justfile(&self, path: &std::path::Path) -> Result<usize> {
        info!("Scanning justfile: {}", path.display());

        // Use the watcher's parse_and_update_justfile method
        self.watcher.parse_and_update_justfile(path).await?;

        // Count the tasks added for this justfile
        let registry = self.registry.lock().await;
        let path_suffix = format!("_{}", path.display());
        let task_count = registry
            .list_tools()
            .iter()
            .filter(|tool| {
                tool.name.starts_with("just_")
                    && !tool.name.starts_with("just_admin_")
                    && tool.name.ends_with(&path_suffix)
            })
            .count();

        Ok(task_count)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncResult {
    pub scanned_files: usize,
    pub found_tasks: usize,
    pub errors: Vec<String>,
    pub duration_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_admin_tools_creation() {
        let registry = Arc::new(Mutex::new(ToolRegistry::new()));
        let watcher = Arc::new(JustfileWatcher::new(registry.clone()));
        let admin_tools = AdminTools::new(registry.clone(), watcher, vec![]);

        // Register admin tools
        admin_tools.register_admin_tools().await.unwrap();

        // Check that sync tool was registered
        let reg = registry.lock().await;
        let tools = reg.list_tools();
        assert!(tools.iter().any(|t| t.name == "just_admin_sync"));
    }

    #[tokio::test]
    async fn test_sync_functionality() {
        let temp_dir = TempDir::new().unwrap();
        let justfile_path = temp_dir.path().join("justfile");

        // Create a test justfile
        let content = r#"
# Test task
test:
    echo "test"

# Build task
build:
    cargo build
"#;
        fs::write(&justfile_path, content).unwrap();

        let registry = Arc::new(Mutex::new(ToolRegistry::new()));
        let watcher = Arc::new(JustfileWatcher::new(registry.clone()));
        let admin_tools = AdminTools::new(
            registry.clone(),
            watcher,
            vec![temp_dir.path().to_path_buf()],
        );

        // Perform sync
        let result = admin_tools.sync().await.unwrap();

        // We might find more than one justfile if there are parent directories
        // with justfiles, so just check that we found at least our test justfile
        assert!(result.scanned_files >= 1);
        assert!(result.found_tasks >= 2);
        assert_eq!(result.errors.len(), 0);

        // Check registry has the tools
        let reg = registry.lock().await;
        let tools = reg.list_tools();
        // Should have at least 2 tools from our test justfile
        let our_justfile_tools: Vec<_> = tools
            .iter()
            .filter(|t| {
                !t.name.starts_with("just_admin_") &&
                t.name.contains(&justfile_path.to_string_lossy().to_string())
            })
            .collect();
        assert_eq!(our_justfile_tools.len(), 2);
    }
}
