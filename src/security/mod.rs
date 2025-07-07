use crate::error::{Error, Result};
use regex::Regex;
use std::path::{Path, PathBuf};
use tracing::warn;

/// Security configuration for the just-mcp server
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Allowed directories for justfile access
    pub allowed_paths: Vec<PathBuf>,
    /// Maximum parameter length to prevent buffer overflow attacks
    pub max_parameter_length: usize,
    /// Forbidden command patterns
    pub forbidden_patterns: Vec<Regex>,
    /// Maximum number of parameters
    pub max_parameters: usize,
    /// Enable strict mode (more restrictive validation)
    pub strict_mode: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            allowed_paths: vec![PathBuf::from(".")], // Default to current directory only
            max_parameter_length: 1024,
            forbidden_patterns: vec![
                // Prevent shell injection patterns
                Regex::new(r"[;&|]|\$\(|\`").unwrap(),
                // Prevent path traversal
                Regex::new(r"\.\.[\\/]").unwrap(),
                // Prevent command substitution
                Regex::new(r"\$\{.*\}").unwrap(),
            ],
            max_parameters: 50,
            strict_mode: true,
        }
    }
}

/// Security validator for command execution
pub struct SecurityValidator {
    config: SecurityConfig,
}

impl SecurityValidator {
    pub fn new(config: SecurityConfig) -> Self {
        Self { config }
    }

    pub fn with_default() -> Self {
        Self::new(SecurityConfig::default())
    }

    /// Validate a justfile path is within allowed directories
    pub fn validate_path(&self, path: &Path) -> Result<()> {
        // For non-existent files, check the parent directory
        let path_to_check = if path.exists() {
            path.canonicalize()
                .map_err(|e| Error::Other(format!("Invalid path {}: {}", path.display(), e)))?
        } else if let Some(parent) = path.parent() {
            // For files that don't exist yet, validate the parent directory
            let canonical_parent = parent.canonicalize().map_err(|e| {
                Error::Other(format!("Invalid parent path {}: {}", parent.display(), e))
            })?;
            // Reconstruct the full path with canonical parent
            canonical_parent.join(path.file_name().unwrap_or_default())
        } else {
            return Err(Error::Other(format!(
                "Cannot validate path without parent: {}",
                path.display()
            )));
        };

        // Check if path is within any allowed directory
        let is_allowed = self.config.allowed_paths.iter().any(|allowed| {
            if let Ok(canonical_allowed) = allowed.canonicalize() {
                path_to_check.starts_with(&canonical_allowed)
            } else {
                false
            }
        });

        if !is_allowed {
            return Err(Error::Other(format!(
                "Access denied: Path {} is outside allowed directories",
                path.display()
            )));
        }

        // Additional checks for suspicious patterns
        let path_str = path.to_string_lossy();
        if path_str.contains("..") || path_str.contains("~") {
            return Err(Error::Other(format!(
                "Suspicious path pattern detected: {}",
                path.display()
            )));
        }

        Ok(())
    }

    /// Validate task name to prevent injection
    pub fn validate_task_name(&self, name: &str) -> Result<()> {
        // Check length
        if name.is_empty() || name.len() > 100 {
            return Err(Error::InvalidParameter(
                "Task name must be between 1 and 100 characters".to_string(),
            ));
        }

        // Only allow alphanumeric, underscore, and hyphen
        if !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            return Err(Error::InvalidParameter(
                "Task name can only contain alphanumeric characters, underscores, and hyphens"
                    .to_string(),
            ));
        }

        // Check for forbidden patterns
        for pattern in &self.config.forbidden_patterns {
            if pattern.is_match(name) {
                return Err(Error::InvalidParameter(format!(
                    "Task name contains forbidden pattern: {name}"
                )));
            }
        }

        Ok(())
    }

    /// Validate parameter value to prevent injection
    pub fn validate_parameter(&self, name: &str, value: &str) -> Result<()> {
        // Check parameter name
        if name.is_empty() || name.len() > 50 {
            return Err(Error::InvalidParameter(
                "Parameter name must be between 1 and 50 characters".to_string(),
            ));
        }

        // Check parameter value length
        if value.len() > self.config.max_parameter_length {
            return Err(Error::InvalidParameter(format!(
                "Parameter value exceeds maximum length of {} characters",
                self.config.max_parameter_length
            )));
        }

        // In strict mode, check for potentially dangerous patterns
        if self.config.strict_mode {
            for pattern in &self.config.forbidden_patterns {
                if pattern.is_match(value) {
                    return Err(Error::InvalidParameter(format!(
                        "Parameter '{name}' contains forbidden pattern"
                    )));
                }
            }

            // Check for null bytes
            if value.contains('\0') {
                return Err(Error::InvalidParameter(
                    "Parameter contains null byte".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Validate all parameters for a task execution
    pub fn validate_parameters(
        &self,
        parameters: &std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        // Check parameter count
        if parameters.len() > self.config.max_parameters {
            return Err(Error::InvalidParameter(format!(
                "Too many parameters: {} (max: {})",
                parameters.len(),
                self.config.max_parameters
            )));
        }

        // Validate each parameter
        for (name, value) in parameters {
            // Convert value to string for validation
            let value_str = match value {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Bool(b) => b.to_string(),
                _ => {
                    return Err(Error::InvalidParameter(format!(
                        "Parameter '{name}' must be a string, number, or boolean"
                    )));
                }
            };

            self.validate_parameter(name, &value_str)?;
        }

        Ok(())
    }

    /// Sanitize a parameter value for safe shell execution
    pub fn sanitize_parameter(&self, value: &str) -> String {
        // For now, we'll use shell escaping
        // In a production system, you might want to use a proper shell escaping library
        shell_escape::escape(value.into()).to_string()
    }

    /// Check if a command should be allowed to execute
    pub fn validate_command(&self, command: &str) -> Result<()> {
        // Check for obvious shell injection attempts
        let dangerous_patterns = &[
            "eval", "exec", "source", "bash", "sh", "zsh", "python", "perl", "ruby",
        ];

        let command_lower = command.to_lowercase();
        for pattern in dangerous_patterns {
            if command_lower.contains(pattern) {
                warn!(
                    "Potentially dangerous command pattern detected: {}",
                    pattern
                );
                if self.config.strict_mode {
                    return Err(Error::Other(format!(
                        "Command contains potentially dangerous pattern: {pattern}"
                    )));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::TempDir;

    #[test]
    fn test_path_validation() {
        let temp_dir = TempDir::new().unwrap();
        let allowed_path = temp_dir.path().to_path_buf();

        let config = SecurityConfig {
            allowed_paths: vec![allowed_path.clone()],
            ..Default::default()
        };
        let validator = SecurityValidator::new(config);

        // Test allowed path
        let valid_path = allowed_path.join("justfile");
        assert!(validator.validate_path(&valid_path).is_ok());

        // Test path outside allowed directory
        let invalid_path = PathBuf::from("/etc/passwd");
        assert!(validator.validate_path(&invalid_path).is_err());
    }

    #[test]
    fn test_task_name_validation() {
        let validator = SecurityValidator::with_default();

        // Valid task names
        assert!(validator.validate_task_name("test").is_ok());
        assert!(validator.validate_task_name("test_task").is_ok());
        assert!(validator.validate_task_name("test-task").is_ok());
        assert!(validator.validate_task_name("test123").is_ok());

        // Invalid task names
        assert!(validator.validate_task_name("").is_err());
        assert!(validator.validate_task_name("test;rm -rf /").is_err());
        assert!(validator.validate_task_name("test|cat").is_err());
        assert!(validator.validate_task_name("test$(whoami)").is_err());
        assert!(validator.validate_task_name("test`date`").is_err());
        assert!(validator.validate_task_name("../../../etc/passwd").is_err());
    }

    #[test]
    fn test_parameter_validation() {
        let validator = SecurityValidator::with_default();

        // Valid parameters
        assert!(validator.validate_parameter("name", "value").is_ok());
        assert!(validator.validate_parameter("count", "123").is_ok());
        assert!(validator.validate_parameter("flag", "true").is_ok());

        // Invalid parameters
        assert!(validator.validate_parameter("", "value").is_err());
        assert!(validator
            .validate_parameter("name", "value; rm -rf /")
            .is_err());
        assert!(validator
            .validate_parameter("name", "value | cat /etc/passwd")
            .is_err());
        assert!(validator.validate_parameter("name", "value\0null").is_err());

        // Test max length
        let long_value = "a".repeat(2000);
        assert!(validator.validate_parameter("name", &long_value).is_err());
    }

    #[test]
    fn test_parameters_validation() {
        let validator = SecurityValidator::with_default();

        let mut params = HashMap::new();
        params.insert("name".to_string(), serde_json::json!("John"));
        params.insert("count".to_string(), serde_json::json!(42));
        params.insert("enabled".to_string(), serde_json::json!(true));

        assert!(validator.validate_parameters(&params).is_ok());

        // Test with invalid parameter
        params.insert("evil".to_string(), serde_json::json!("value; rm -rf /"));
        assert!(validator.validate_parameters(&params).is_err());

        // Test with too many parameters
        for i in 0..100 {
            params.insert(format!("param{}", i), serde_json::json!("value"));
        }
        assert!(validator.validate_parameters(&params).is_err());
    }

    #[test]
    fn test_command_validation() {
        let validator = SecurityValidator::with_default();

        // Safe commands
        assert!(validator.validate_command("echo hello").is_ok());
        assert!(validator.validate_command("cargo build").is_ok());
        assert!(validator.validate_command("npm test").is_ok());

        // Dangerous commands in strict mode
        assert!(validator.validate_command("eval $CODE").is_err());
        assert!(validator.validate_command("exec bash").is_err());
        assert!(validator.validate_command("python script.py").is_err());
    }

    #[test]
    fn test_parameter_sanitization() {
        let validator = SecurityValidator::with_default();

        // Test sanitization
        assert_eq!(validator.sanitize_parameter("hello"), "hello");
        assert_eq!(validator.sanitize_parameter("hello world"), "'hello world'");
        assert_eq!(
            validator.sanitize_parameter("hello; rm -rf /"),
            "'hello; rm -rf /'"
        );
        assert_eq!(validator.sanitize_parameter("$(whoami)"), "'$(whoami)'");
    }
}
