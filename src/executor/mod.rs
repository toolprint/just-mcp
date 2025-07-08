use crate::error::{Error, Result};
use crate::parser::JustfileParser;
use crate::resource_limits::{ResourceLimits, ResourceManager};
use crate::security::{SecurityConfig, SecurityValidator};
use crate::types::{ExecutionContext, ExecutionRequest, ExecutionResult, JustTask};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::Command;
use tokio::time::{timeout, Duration};
use tracing::{error, info, warn};

// Re-export for tests
pub use crate::security::{SecurityConfig as SecConfig, SecurityValidator as SecValidator};

pub struct TaskExecutor {
    default_timeout: Duration,
    parser: JustfileParser,
    justfile_cache: HashMap<PathBuf, Vec<JustTask>>,
    security_validator: SecurityValidator,
    resource_manager: Arc<ResourceManager>,
}

impl TaskExecutor {
    pub fn new() -> Self {
        let resource_manager = Arc::new(ResourceManager::with_default());
        Self {
            default_timeout: resource_manager.get_timeout(),
            parser: JustfileParser::new().expect("Failed to create parser"),
            justfile_cache: HashMap::new(),
            security_validator: SecurityValidator::with_default(),
            resource_manager,
        }
    }

    pub fn with_security_config(mut self, config: SecurityConfig) -> Self {
        self.security_validator = SecurityValidator::new(config);
        self
    }

    pub fn with_resource_limits(mut self, limits: ResourceLimits) -> Self {
        self.resource_manager = Arc::new(ResourceManager::new(limits));
        self.default_timeout = self.resource_manager.get_timeout();
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.default_timeout = timeout;
        self
    }

    pub async fn execute(&mut self, request: ExecutionRequest) -> Result<ExecutionResult> {
        info!("Executing task: {}", request.tool_name);

        // Check resource limits before starting
        self.resource_manager.can_execute()?;

        // Extract task name and justfile path from tool name
        // Tool names are in format: just_taskname_/path/to/justfile
        let (task_name, justfile_path) = self.parse_tool_name(&request.tool_name)?;
        info!(
            "Parsed task name: {}, justfile path: {}",
            task_name, justfile_path
        );

        // Validate task name
        self.security_validator.validate_task_name(&task_name)?;

        // Validate justfile path
        let justfile_path_buf = PathBuf::from(&justfile_path);
        self.security_validator.validate_path(&justfile_path_buf)?;

        // Validate parameters
        self.security_validator
            .validate_parameters(&request.parameters)?;

        // Verify task exists
        {
            let tasks = self.get_or_parse_justfile(&justfile_path)?;
            let _task = tasks
                .iter()
                .find(|t| t.name == task_name)
                .ok_or_else(|| Error::TaskNotFound(task_name.clone()))?;
        }

        // Determine working directory
        let working_dir = request
            .context
            .working_directory
            .as_ref()
            .map(PathBuf::from)
            .or_else(|| Path::new(&justfile_path).parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."));

        // Execute the command
        let context = ExecutionContext {
            working_directory: Some(working_dir.to_string_lossy().to_string()),
            environment: request.context.environment,
            timeout: request.context.timeout,
        };

        // Start tracking this execution
        let _execution_guard = self.resource_manager.start_execution();

        self.execute_just_command(&task_name, &request.parameters, &context)
            .await
    }

    fn parse_tool_name(&self, tool_name: &str) -> Result<(String, String)> {
        // Tool names are in format: just_taskname_/path/to/justfile
        if !tool_name.starts_with("just_") {
            return Err(Error::InvalidParameter(format!(
                "Invalid tool name: {tool_name}"
            )));
        }

        let without_prefix = &tool_name[5..]; // Remove "just_"

        // Find the underscore that separates task name from path
        // We need to find the first underscore followed by a '/'
        let mut found_pos = None;
        let chars: Vec<char> = without_prefix.chars().collect();

        for i in 0..chars.len() {
            if chars[i] == '_' && i + 1 < chars.len() && chars[i + 1] == '/' {
                found_pos = Some(i);
                break;
            }
        }

        if let Some(pos) = found_pos {
            let task_name = without_prefix[..pos].to_string();
            let justfile_path = without_prefix[pos + 1..].to_string();
            Ok((task_name, justfile_path))
        } else {
            Err(Error::InvalidParameter(format!(
                "Invalid tool name format: {tool_name}"
            )))
        }
    }

    fn get_or_parse_justfile(&mut self, path: &str) -> Result<&Vec<JustTask>> {
        let path_buf = PathBuf::from(path);

        info!("Getting or parsing justfile at: {}", path_buf.display());

        // Check cache first
        if self.justfile_cache.contains_key(&path_buf) {
            info!("Found in cache");
            return Ok(self.justfile_cache.get(&path_buf).unwrap());
        }

        // Check if file exists
        if !path_buf.exists() {
            error!("Justfile does not exist at: {}", path_buf.display());
            return Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Justfile not found: {}", path_buf.display()),
            )));
        }

        // Parse the justfile
        info!("Parsing justfile...");
        let tasks = self.parser.parse_file(&path_buf)?;
        info!("Parsed {} tasks", tasks.len());
        self.justfile_cache.insert(path_buf.clone(), tasks);
        Ok(self.justfile_cache.get(&path_buf).unwrap())
    }

    async fn execute_just_command(
        &self,
        task_name: &str,
        parameters: &HashMap<String, serde_json::Value>,
        context: &ExecutionContext,
    ) -> Result<ExecutionResult> {
        info!(
            "Executing just command: task={}, context={:?}",
            task_name, context
        );

        // Use just to execute the command
        let mut cmd = Command::new("just");

        // Set working directory if provided
        if let Some(ref wd) = context.working_directory {
            cmd.current_dir(wd);
            // The justfile is in the working directory
            cmd.arg("--justfile");
            cmd.arg("justfile");
        }

        // Add the task name
        cmd.arg(task_name);

        // Get task definition to know parameter order
        let tasks = self.parser.parse_file(&PathBuf::from(format!(
            "{}/justfile",
            context.working_directory.as_ref().unwrap()
        )))?;
        let task = tasks.iter().find(|t| t.name == task_name);

        if let Some(task) = task {
            // Add parameters in the order they're defined in the task
            for param in &task.parameters {
                if let Some(value) = parameters.get(&param.name) {
                    let value_str = match value {
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    // Sanitize parameter value before passing to command
                    let sanitized_value = self.security_validator.sanitize_parameter(&value_str);
                    cmd.arg(sanitized_value);
                } else if let Some(default) = &param.default {
                    // Sanitize default value as well
                    let sanitized_default = self.security_validator.sanitize_parameter(default);
                    cmd.arg(sanitized_default);
                }
            }
        }

        cmd.stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null());

        // Set environment variables
        for (key, value) in &context.environment {
            cmd.env(key, value);
        }

        let timeout_duration = context
            .timeout
            .map(Duration::from_secs)
            .unwrap_or(self.default_timeout);

        // Execute with timeout
        match timeout(timeout_duration, cmd.output()).await {
            Ok(Ok(output)) => {
                // Check output size limits
                self.resource_manager
                    .check_output_size(output.stdout.len(), output.stderr.len())?;

                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code();
                let success = output.status.success();

                if !success {
                    warn!("Command failed with exit code {:?}: {}", exit_code, stderr);
                }

                Ok(ExecutionResult {
                    success,
                    exit_code,
                    stdout,
                    stderr,
                    error: if success {
                        None
                    } else {
                        Some(format!("Command failed with exit code {exit_code:?}"))
                    },
                })
            }
            Ok(Err(e)) => {
                error!("Failed to execute command: {}", e);
                Ok(ExecutionResult {
                    success: false,
                    exit_code: None,
                    stdout: String::new(),
                    stderr: String::new(),
                    error: Some(format!("Failed to execute command: {e}")),
                })
            }
            Err(_) => {
                error!("Command timed out after {:?}", timeout_duration);
                Ok(ExecutionResult {
                    success: false,
                    exit_code: None,
                    stdout: String::new(),
                    stderr: String::new(),
                    error: Some(format!("Command timed out after {timeout_duration:?}")),
                })
            }
        }
    }

    pub async fn execute_command(
        &self,
        command: &str,
        args: &[String],
        context: &ExecutionContext,
    ) -> Result<ExecutionResult> {
        let mut cmd = Command::new(command);
        cmd.args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null());

        // Set working directory if provided
        if let Some(ref wd) = context.working_directory {
            cmd.current_dir(wd);
        }

        // Set environment variables
        for (key, value) in &context.environment {
            cmd.env(key, value);
        }

        let timeout_duration = context
            .timeout
            .map(Duration::from_secs)
            .unwrap_or(self.default_timeout);

        // Execute with timeout
        match timeout(timeout_duration, cmd.output()).await {
            Ok(Ok(output)) => {
                // Check output size limits
                self.resource_manager
                    .check_output_size(output.stdout.len(), output.stderr.len())?;

                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code();
                let success = output.status.success();

                Ok(ExecutionResult {
                    success,
                    exit_code,
                    stdout,
                    stderr,
                    error: if success {
                        None
                    } else {
                        Some(format!("Command failed with exit code {exit_code:?}"))
                    },
                })
            }
            Ok(Err(e)) => Ok(ExecutionResult {
                success: false,
                exit_code: None,
                stdout: String::new(),
                stderr: String::new(),
                error: Some(format!("Failed to execute command: {e}")),
            }),
            Err(_) => Ok(ExecutionResult {
                success: false,
                exit_code: None,
                stdout: String::new(),
                stderr: String::new(),
                error: Some(format!("Command timed out after {timeout_duration:?}")),
            }),
        }
    }
}

impl Default for TaskExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_executor_creation() {
        let executor = TaskExecutor::new();
        assert_eq!(executor.default_timeout, Duration::from_secs(300));
    }

    #[test]
    fn test_parse_tool_name() {
        let executor = TaskExecutor::new();

        // Valid tool name
        let result = executor.parse_tool_name("just_build_/home/user/project/justfile");
        assert!(result.is_ok());
        let (task, path) = result.unwrap();
        assert_eq!(task, "build");
        assert_eq!(path, "/home/user/project/justfile");

        // Valid tool name with underscore in path
        let result = executor.parse_tool_name("just_test_/home/user_name/test_project/justfile");
        assert!(result.is_ok());
        let (task, path) = result.unwrap();
        assert_eq!(task, "test");
        assert_eq!(path, "/home/user_name/test_project/justfile");

        // Invalid tool name (no prefix)
        let result = executor.parse_tool_name("build_/home/user/project/justfile");
        assert!(result.is_err());

        // Invalid tool name (no separator)
        let result = executor.parse_tool_name("just_build");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_execute_simple_command() {
        let executor = TaskExecutor::new();
        let context = ExecutionContext {
            working_directory: None,
            environment: HashMap::new(),
            timeout: Some(5),
        };

        let result = executor
            .execute_command("echo", &["hello".to_string()], &context)
            .await;
        assert!(result.is_ok());

        let exec_result = result.unwrap();
        assert!(exec_result.success);
        assert_eq!(exec_result.exit_code, Some(0));
        assert!(exec_result.stdout.contains("hello"));
        assert!(exec_result.stderr.is_empty());
        assert!(exec_result.error.is_none());
    }

    #[tokio::test]
    async fn test_execute_with_timeout() {
        let executor = TaskExecutor::new();
        let context = ExecutionContext {
            working_directory: None,
            environment: HashMap::new(),
            timeout: Some(1), // 1 second timeout
        };

        // Command that takes longer than timeout
        let result = executor
            .execute_command("sleep", &["2".to_string()], &context)
            .await;
        assert!(result.is_ok());

        let exec_result = result.unwrap();
        assert!(!exec_result.success);
        assert!(exec_result.error.is_some());
        assert!(exec_result.error.unwrap().contains("timed out"));
    }

    #[tokio::test]
    async fn test_execute_with_working_directory() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "test content").unwrap();

        let executor = TaskExecutor::new();
        let context = ExecutionContext {
            working_directory: Some(temp_dir.path().to_string_lossy().to_string()),
            environment: HashMap::new(),
            timeout: None,
        };

        #[cfg(target_os = "windows")]
        let (cmd, args) = (
            "cmd",
            vec!["/C".to_string(), "type".to_string(), "test.txt".to_string()],
        );
        #[cfg(not(target_os = "windows"))]
        let (cmd, args) = ("cat", vec!["test.txt".to_string()]);

        let result = executor.execute_command(cmd, &args, &context).await;
        assert!(result.is_ok());

        let exec_result = result.unwrap();
        assert!(exec_result.success);
        assert!(exec_result.stdout.contains("test content"));
    }
}
