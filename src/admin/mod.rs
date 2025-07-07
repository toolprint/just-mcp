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
    watch_configs: Vec<(PathBuf, Option<String>)>,
}

impl AdminTools {
    pub fn new(
        registry: Arc<Mutex<ToolRegistry>>,
        watcher: Arc<JustfileWatcher>,
        watch_paths: Vec<PathBuf>,
        watch_configs: Vec<(PathBuf, Option<String>)>,
    ) -> Self {
        Self {
            registry,
            watcher,
            watch_paths,
            watch_configs,
        }
    }

    pub async fn register_admin_tools(&self) -> Result<()> {
        let mut registry = self.registry.lock().await;

        // Register sync() tool
        let sync_tool = ToolDefinition {
            name: "admin_sync".to_string(),
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
            internal_name: None,
        };

        registry.add_tool(sync_tool)?;

        // Register create_task() tool
        let create_task_tool = ToolDefinition {
            name: "admin_create_task".to_string(),
            description: "Create a new task in a justfile with AI assistance".to_string(),
            input_schema: json!({
                "$schema": "http://json-schema.org/draft-07/schema#",
                "type": "object",
                "properties": {
                    "watch_name": {
                        "type": "string",
                        "description": "Name of the watch directory to create task in (e.g., 'frontend', 'backend'). If omitted, uses the main/default justfile"
                    },
                    "task_name": {
                        "type": "string",
                        "description": "Name of the new task"
                    },
                    "description": {
                        "type": "string",
                        "description": "Description/comment for the task"
                    },
                    "recipe": {
                        "type": "string",
                        "description": "The command(s) to execute"
                    },
                    "parameters": {
                        "type": "array",
                        "description": "Task parameters",
                        "items": {
                            "type": "object",
                            "properties": {
                                "name": {"type": "string"},
                                "default": {"type": "string"}
                            },
                            "required": ["name"]
                        }
                    },
                    "dependencies": {
                        "type": "array",
                        "description": "Task dependencies",
                        "items": {"type": "string"}
                    }
                },
                "required": ["task_name", "recipe"],
                "additionalProperties": false
            }),
            dependencies: vec![],
            source_hash: "admin_tool_create_task_v1".to_string(),
            last_modified: std::time::SystemTime::now(),
            internal_name: None,
        };

        registry.add_tool(create_task_tool)?;

        // TODO: Add modify_task, remove_task tools in future subtasks

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
                .filter(|tool| !tool.name.starts_with("admin_"))
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

        // Send a single notification after all tools are registered
        self.watcher.send_tools_changed_notification();

        Ok(SyncResult {
            scanned_files,
            found_tasks,
            errors,
            duration_ms: duration.as_millis() as u64,
        })
    }

    async fn scan_justfile(&self, path: &std::path::Path) -> Result<usize> {
        info!("Scanning justfile: {}", path.display());

        // Use the watcher's parse method without sending notifications
        let task_count = self.watcher.parse_and_update_justfile_without_notification(path).await?;

        Ok(task_count)
    }

    pub async fn create_task(&self, params: CreateTaskParams) -> Result<CreateTaskResult> {
        info!(
            "Creating new task: {} in {}",
            params.task_name,
            params
                .watch_name
                .as_deref()
                .unwrap_or("default justfile")
        );

        // Determine which justfile to use
        let justfile_path = if let Some(watch_name) = params.watch_name {
            // Find the watch directory by name
            let mut found_path = None;
            
            for (path, name) in &self.watch_configs {
                if let Some(n) = name {
                    if n == &watch_name {
                        // Found by name
                        if path.is_dir() {
                            let justfile = path.join("justfile");
                            if justfile.exists() {
                                found_path = Some(justfile);
                                break;
                            }
                            let justfile_cap = path.join("Justfile");
                            if justfile_cap.exists() {
                                found_path = Some(justfile_cap);
                                break;
                            }
                        } else {
                            found_path = Some(path.clone());
                            break;
                        }
                    }
                }
            }
            
            found_path.ok_or_else(|| crate::error::Error::Other(
                format!("Watch directory '{}' not found. Available: {}", 
                    watch_name,
                    self.watch_configs.iter()
                        .filter_map(|(_, name)| name.as_ref())
                        .map(|n| format!("'{}'", n))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            ))?
        } else {
            // No name specified - use the main/first justfile
            let (path, _) = &self.watch_configs.get(0)
                .ok_or_else(|| crate::error::Error::Other("No watch directories configured".to_string()))?;
            
            if path.is_dir() {
                let justfile = path.join("justfile");
                if justfile.exists() {
                    justfile
                } else {
                    let justfile_cap = path.join("Justfile");
                    if justfile_cap.exists() {
                        justfile_cap
                    } else {
                        return Err(crate::error::Error::Other("No justfile found in main watch directory".to_string()));
                    }
                }
            } else {
                path.clone()
            }
        };

        // Validate task name doesn't conflict with existing tasks
        {
            let registry = self.registry.lock().await;
            
            // Check for any tool that starts with "just_{task_name}"
            // This handles both single directory (just_taskname) and multi-directory (just_taskname@name) cases
            let task_prefix = format!("just_{}", params.task_name);
            let existing_task = registry.list_tools().iter().any(|tool| {
                tool.name == task_prefix || tool.name.starts_with(&format!("{}@", task_prefix))
            });
            
            if existing_task {
                return Err(crate::error::Error::Other(format!(
                    "Task '{}' already exists in {}",
                    params.task_name,
                    justfile_path.display()
                )));
            }

            // Check for admin tool conflicts
            if params.task_name.starts_with("admin_") {
                return Err(crate::error::Error::Other(
                    "Task names starting with 'admin_' are reserved".to_string(),
                ));
            }
        }

        // Create backup with dotfile naming
        let backup_path = justfile_path.parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join(format!(".{}.bak", justfile_path.file_name().unwrap().to_string_lossy()));
        std::fs::copy(&justfile_path, &backup_path)?;

        // Read existing content
        let existing_content = std::fs::read_to_string(&justfile_path)?;

        // Build the new task content
        let mut task_content = String::new();

        // Ensure proper spacing: always add a blank line before the new task
        if !existing_content.is_empty() {
            // If file doesn't end with newline, add one
            if !existing_content.ends_with('\n') {
                task_content.push('\n');
            }
            // Always add a blank line for visual separation
            task_content.push('\n');
        }

        // Add description as comment
        if let Some(desc) = &params.description {
            task_content.push_str(&format!("# {desc}\n"));
        }

        // Add task signature
        task_content.push_str(&params.task_name);

        // Add parameters
        if let Some(parameters) = &params.parameters {
            for param in parameters {
                task_content.push(' ');
                task_content.push_str(&param.name);
                if let Some(default) = &param.default {
                    task_content.push_str(&format!("=\"{default}\""));
                }
            }
        }

        // Add dependencies
        if let Some(deps) = &params.dependencies {
            if !deps.is_empty() {
                task_content.push_str(": ");
                task_content.push_str(&deps.join(" "));
            }
        }

        task_content.push_str(":\n");

        // Add recipe body with proper indentation
        for line in params.recipe.lines() {
            task_content.push_str("    ");
            task_content.push_str(line);
            task_content.push('\n');
        }

        // Write updated content
        let new_content = existing_content + &task_content;
        std::fs::write(&justfile_path, &new_content)?;

        // Re-scan the justfile to update registry
        self.scan_justfile(&justfile_path).await?;

        info!(
            "Successfully created task '{}' in {}",
            params.task_name,
            justfile_path.display()
        );

        Ok(CreateTaskResult {
            task_name: params.task_name,
            justfile_path: justfile_path.to_string_lossy().to_string(),
            backup_path: backup_path.to_string_lossy().to_string(),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncResult {
    pub scanned_files: usize,
    pub found_tasks: usize,
    pub errors: Vec<String>,
    pub duration_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTaskParams {
    pub watch_name: Option<String>,
    pub task_name: String,
    pub description: Option<String>,
    pub recipe: String,
    pub parameters: Option<Vec<TaskParameter>>,
    pub dependencies: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskParameter {
    pub name: String,
    pub default: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTaskResult {
    pub task_name: String,
    pub justfile_path: String,
    pub backup_path: String,
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
        let admin_tools = AdminTools::new(registry.clone(), watcher, vec![], vec![]);

        // Register admin tools
        admin_tools.register_admin_tools().await.unwrap();

        // Check that sync tool was registered
        let reg = registry.lock().await;
        let tools = reg.list_tools();
        assert!(tools.iter().any(|t| t.name == "admin_sync"));
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
            vec![(temp_dir.path().to_path_buf(), None)],
        );

        // Perform sync
        let result = admin_tools.sync().await.unwrap();

        // We might find more than one justfile if there are parent directories
        // with justfiles, so just check that we found at least our test justfile
        assert!(result.scanned_files >= 1);
        assert!(result.found_tasks >= 2, "Expected at least 2 tasks, found {}", result.found_tasks);
        assert_eq!(result.errors.len(), 0);

        // Check registry has the tools
        let reg = registry.lock().await;
        let tools = reg.list_tools();
        // Should have at least 2 tools from our test justfile
        let our_justfile_tools: Vec<_> = tools
            .iter()
            .filter(|t| {
                t.name.starts_with("just_") && !t.name.starts_with("admin_")
            })
            .collect();
        assert!(our_justfile_tools.len() >= 2, "Expected at least 2 tools, found {}", our_justfile_tools.len());
    }

    #[tokio::test]
    async fn test_create_task() {
        let temp_dir = TempDir::new().unwrap();
        let justfile_path = temp_dir.path().join("justfile");

        // Create an initial justfile
        let content = r#"
# Existing task
existing:
    echo "existing"
"#;
        fs::write(&justfile_path, content).unwrap();

        let registry = Arc::new(Mutex::new(ToolRegistry::new()));
        let watcher = Arc::new(JustfileWatcher::new(registry.clone()));
        let admin_tools = AdminTools::new(
            registry.clone(),
            watcher,
            vec![temp_dir.path().to_path_buf()],
            vec![(temp_dir.path().to_path_buf(), None)],
        );

        // Create a new task
        let params = CreateTaskParams {
            watch_name: None,  // Use default
            task_name: "new_task".to_string(),
            description: Some("A new test task".to_string()),
            recipe: "echo \"hello world\"\necho \"second line\"".to_string(),
            parameters: Some(vec![TaskParameter {
                name: "name".to_string(),
                default: Some("world".to_string()),
            }]),
            dependencies: Some(vec!["existing".to_string()]),
        };

        let result = admin_tools.create_task(params).await.unwrap();

        assert_eq!(result.task_name, "new_task");
        assert!(result.backup_path.ends_with(".justfile.bak"));

        // Verify the task was added to the file
        let new_content = fs::read_to_string(&justfile_path).unwrap();
        assert!(new_content.contains("# A new test task"));
        assert!(new_content.contains("new_task name=\"world\": existing"));
        assert!(new_content.contains("    echo \"hello world\""));
        assert!(new_content.contains("    echo \"second line\""));

        // Verify backup was created
        let backup_path = justfile_path.parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join(format!(".{}.bak", justfile_path.file_name().unwrap().to_string_lossy()));
        assert!(backup_path.exists());

        // Verify registry was updated
        let reg = registry.lock().await;
        let tools = reg.list_tools();
        let new_task_tool = tools
            .iter()
            .find(|t| t.name.contains("new_task"))
            .expect("New task should be in registry");
        assert_eq!(new_task_tool.description, "A new test task");
    }

    #[tokio::test]
    async fn test_create_task_validation() {
        let temp_dir = TempDir::new().unwrap();
        let justfile_path = temp_dir.path().join("justfile");

        // Create an initial justfile
        let content = r#"
# Existing task
existing:
    echo "existing"
"#;
        fs::write(&justfile_path, content).unwrap();

        let registry = Arc::new(Mutex::new(ToolRegistry::new()));
        let watcher = Arc::new(JustfileWatcher::new(registry.clone()));
        let admin_tools = AdminTools::new(
            registry.clone(),
            watcher.clone(),
            vec![temp_dir.path().to_path_buf()],
            vec![(temp_dir.path().to_path_buf(), None)],
        );

        // Parse initial justfile to populate registry
        watcher
            .parse_and_update_justfile(&justfile_path)
            .await
            .unwrap();

        // Try to create a task with existing name
        let params = CreateTaskParams {
            watch_name: None,  // Use default
            task_name: "existing".to_string(),
            description: None,
            recipe: "echo \"duplicate\"".to_string(),
            parameters: None,
            dependencies: None,
        };

        let result = admin_tools.create_task(params).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));

        // Try to create a task with admin_ prefix
        let params = CreateTaskParams {
            watch_name: None,  // Use default
            task_name: "admin_task".to_string(),
            description: None,
            recipe: "echo \"admin\"".to_string(),
            parameters: None,
            dependencies: None,
        };

        let result = admin_tools.create_task(params).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("reserved"));
    }

    #[tokio::test]
    async fn test_create_task_with_named_dirs() {
        let temp_dir1 = TempDir::new().unwrap();
        let temp_dir2 = TempDir::new().unwrap();
        let justfile_path1 = temp_dir1.path().join("justfile");
        let justfile_path2 = temp_dir2.path().join("justfile");

        // Create justfiles
        fs::write(&justfile_path1, "# Frontend tasks\n").unwrap();
        fs::write(&justfile_path2, "# Backend tasks\n").unwrap();

        let registry = Arc::new(Mutex::new(ToolRegistry::new()));
        let watcher = Arc::new(JustfileWatcher::new(registry.clone()));
        let admin_tools = AdminTools::new(
            registry.clone(),
            watcher,
            vec![temp_dir1.path().to_path_buf(), temp_dir2.path().to_path_buf()],
            vec![
                (temp_dir1.path().to_path_buf(), Some("frontend".to_string())),
                (temp_dir2.path().to_path_buf(), Some("backend".to_string())),
            ],
        );

        // Test creating task with name
        let params = CreateTaskParams {
            watch_name: Some("frontend".to_string()),
            task_name: "build".to_string(),
            description: Some("Build frontend".to_string()),
            recipe: "npm run build".to_string(),
            parameters: None,
            dependencies: None,
        };

        let result = admin_tools.create_task(params).await.unwrap();
        assert_eq!(result.task_name, "build");
        assert!(result.justfile_path.contains("justfile"));

        // Verify the task was added
        let content = fs::read_to_string(&justfile_path1).unwrap();
        assert!(content.contains("# Build frontend"));
        assert!(content.contains("build:"));
        assert!(content.contains("npm run build"));
    }
}
