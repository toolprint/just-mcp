//! Error Adapter for Framework Integration
//!
//! This module provides error handling alignment between just-mcp's internal
//! error types and the ultrafast-mcp framework's expected error formats.
//!
//! The key requirement is to preserve the quality and meaningfulness of error
//! messages while ensuring compatibility with MCP protocol standards and
//! framework expectations.

use crate::error::{Error as JustMcpError, Result as JustMcpResult};
use crate::types::ExecutionResult;

#[cfg(feature = "ultrafast-framework")]
use ultrafast_mcp::{MCPError, MCPResult};

/// Framework-compatible error adapter
///
/// This adapter converts between just-mcp's internal error types and the
/// framework's expected error formats while preserving error message quality.
pub struct ErrorAdapter;

impl ErrorAdapter {
    /// Convert just-mcp Error to framework-compatible MCPError
    ///
    /// This method preserves the semantic meaning and user-friendly messages
    /// of our internal errors while converting them to framework-compatible
    /// formats that work with the MCP protocol.
    #[cfg(feature = "ultrafast-framework")]
    pub fn to_mcp_error(error: JustMcpError) -> MCPError {
        match error {
            // Tool and task-related errors
            JustMcpError::TaskNotFound(task_name) => {
                MCPError::invalid_request(format!(
                    "Task '{}' not found. Use 'just --list' to see available tasks.",
                    task_name
                ))
            }
            JustMcpError::ToolNotFound(tool_name) => {
                MCPError::invalid_request(format!(
                    "Tool '{}' is not available. Check tool registration or refresh tool list.",
                    tool_name
                ))
            }
            JustMcpError::InvalidToolName(tool_name) => {
                MCPError::invalid_request(format!(
                    "Invalid tool name '{}'. Tool names must follow the format 'task_/path/to/justfile'.",
                    tool_name
                ))
            }

            // Parameter validation errors
            JustMcpError::InvalidParameter(msg) => {
                MCPError::invalid_params(format!(
                    "Invalid parameter: {}. Please check parameter types and constraints.",
                    msg
                ))
            }

            // Execution errors with context preservation
            JustMcpError::Execution { command, exit_code, stderr } => {
                let error_msg = if stderr.is_empty() {
                    format!(
                        "Command '{}' failed with exit code {:?}. No additional error information available.",
                        command, exit_code
                    )
                } else {
                    format!(
                        "Command '{}' failed with exit code {:?}: {}",
                        command, exit_code, stderr
                    )
                };
                MCPError::internal_error(error_msg)
            }

            // Just command errors
            JustMcpError::JustCommand(msg) => {
                MCPError::internal_error(format!(
                    "Just command error: {}. Verify justfile syntax and task definitions.",
                    msg
                ))
            }

            // Parse errors with location information
            JustMcpError::Parse { message, line, column } => {
                MCPError::invalid_request(format!(
                    "Justfile parse error at line {}, column {}: {}. Check justfile syntax.",
                    line, column, message
                ))
            }

            // Registry errors
            JustMcpError::Registry(msg) => {
                MCPError::internal_error(format!(
                    "Tool registry error: {}. This may indicate a configuration issue.",
                    msg
                ))
            }

            // Server errors
            JustMcpError::Server(msg) => {
                MCPError::internal_error(format!(
                    "Server error: {}. Check server configuration and logs.",
                    msg
                ))
            }

            // Timeout errors
            JustMcpError::Timeout(msg) => {
                MCPError::internal_error(format!(
                    "Operation timed out: {}. Consider increasing timeout limits or checking system performance.",
                    msg
                ))
            }

            // IO errors with context
            JustMcpError::Io(io_error) => {
                MCPError::internal_error(format!(
                    "File system error: {}. Check file permissions and disk space.",
                    io_error
                ))
            }

            // JSON errors
            JustMcpError::Json(json_error) => {
                MCPError::invalid_request(format!(
                    "JSON parsing error: {}. Check input format and structure.",
                    json_error
                ))
            }

            // File watching errors
            JustMcpError::Watch(watch_error) => {
                MCPError::internal_error(format!(
                    "File watching error: {}. File monitoring may be temporarily unavailable.",
                    watch_error
                ))
            }

            // Regex errors
            JustMcpError::Regex(regex_error) => {
                MCPError::internal_error(format!(
                    "Pattern matching error: {}. This indicates a configuration issue.",
                    regex_error
                ))
            }

            // Internal errors
            JustMcpError::Internal(msg) => {
                MCPError::internal_error(format!(
                    "Internal error: {}. Please report this issue with context.",
                    msg
                ))
            }

            // Generic other errors
            JustMcpError::Other(msg) => {
                MCPError::internal_error(format!(
                    "Unexpected error: {}. Please check logs for additional details.",
                    msg
                ))
            }
        }
    }

    /// Convert just-mcp Result to framework-compatible MCPResult
    ///
    /// This is a convenience method that applies error conversion to Result types.
    #[cfg(feature = "ultrafast-framework")]
    pub fn to_mcp_result<T>(result: JustMcpResult<T>) -> MCPResult<T> {
        result.map_err(Self::to_mcp_error)
    }

    /// Convert ExecutionResult to framework-compatible error representation
    ///
    /// This method handles the special case where ExecutionResult indicates
    /// failure but should be converted to an appropriate MCPError rather
    /// than being passed through as a successful result.
    #[cfg(feature = "ultrafast-framework")]
    pub fn execution_result_to_mcp_result(result: ExecutionResult) -> MCPResult<ExecutionResult> {
        if result.success {
            // Successful execution - return as-is
            Ok(result)
        } else {
            // Failed execution - convert to appropriate error
            let error_msg = if let Some(ref error) = result.error {
                format!("Tool execution failed: {}", error)
            } else if !result.stderr.is_empty() {
                format!("Tool execution failed with stderr: {}", result.stderr)
            } else {
                format!(
                    "Tool execution failed with exit code {:?}",
                    result.exit_code
                )
            };

            // Include execution details in error for debugging
            let detailed_error = if result.stderr.is_empty() && result.error.is_none() {
                error_msg
            } else {
                format!(
                    "{}. Execution details: exit_code={:?}, stderr='{}', error='{}'",
                    error_msg,
                    result.exit_code,
                    result.stderr,
                    result.error.as_deref().unwrap_or("none")
                )
            };

            Err(MCPError::internal_error(detailed_error))
        }
    }

    /// Extract error information suitable for logging and debugging
    ///
    /// This method provides structured error information that can be used
    /// for logging, monitoring, and debugging while preserving sensitive
    /// information handling.
    pub fn extract_error_info(error: &JustMcpError) -> ErrorInfo {
        match error {
            JustMcpError::TaskNotFound(task_name) => ErrorInfo {
                error_type: "task_not_found".to_string(),
                user_message: format!("Task '{}' not found", task_name),
                technical_details: format!("TaskNotFound: {}", task_name),
                is_user_error: true,
                is_retryable: false,
            },
            JustMcpError::ToolNotFound(tool_name) => ErrorInfo {
                error_type: "tool_not_found".to_string(),
                user_message: format!("Tool '{}' is not available", tool_name),
                technical_details: format!("ToolNotFound: {}", tool_name),
                is_user_error: true,
                is_retryable: true, // User might retry after tool refresh
            },
            JustMcpError::InvalidParameter(msg) => ErrorInfo {
                error_type: "invalid_parameter".to_string(),
                user_message: format!("Invalid parameter: {}", msg),
                technical_details: format!("InvalidParameter: {}", msg),
                is_user_error: true,
                is_retryable: false,
            },
            JustMcpError::Execution {
                command,
                exit_code,
                stderr,
            } => ErrorInfo {
                error_type: "execution_failure".to_string(),
                user_message: if stderr.is_empty() {
                    format!("Command failed with exit code {:?}", exit_code)
                } else {
                    format!("Command failed: {}", stderr)
                },
                technical_details: format!(
                    "Execution: command='{}', exit_code={:?}, stderr='{}'",
                    command, exit_code, stderr
                ),
                is_user_error: false, // Could be system or justfile issue
                is_retryable: true,
            },
            JustMcpError::Parse {
                message,
                line,
                column,
            } => ErrorInfo {
                error_type: "parse_error".to_string(),
                user_message: format!(
                    "Syntax error at line {}, column {}: {}",
                    line, column, message
                ),
                technical_details: format!("Parse error: {}:{}:{}", line, column, message),
                is_user_error: true, // User needs to fix justfile
                is_retryable: false,
            },
            JustMcpError::Timeout(msg) => ErrorInfo {
                error_type: "timeout".to_string(),
                user_message: "Operation timed out".to_string(),
                technical_details: format!("Timeout: {}", msg),
                is_user_error: false,
                is_retryable: true,
            },
            JustMcpError::Io(io_error) => ErrorInfo {
                error_type: "io_error".to_string(),
                user_message: "File system error occurred".to_string(),
                technical_details: format!("IO: {}", io_error),
                is_user_error: false,
                is_retryable: true,
            },
            _ => ErrorInfo {
                error_type: "internal_error".to_string(),
                user_message: "An internal error occurred".to_string(),
                technical_details: format!("{:?}", error),
                is_user_error: false,
                is_retryable: false,
            },
        }
    }

    /// Check if an error indicates a user-correctable issue
    ///
    /// This helps determine whether error messages should suggest user action
    /// or indicate system/configuration issues.
    pub fn is_user_correctable(error: &JustMcpError) -> bool {
        match error {
            JustMcpError::TaskNotFound(_)
            | JustMcpError::InvalidParameter(_)
            | JustMcpError::Parse { .. }
            | JustMcpError::InvalidToolName(_) => true,
            _ => false,
        }
    }

    /// Check if an error is likely transient and worth retrying
    ///
    /// This helps clients determine retry strategies.
    pub fn is_retryable(error: &JustMcpError) -> bool {
        match error {
            JustMcpError::Timeout(_)
            | JustMcpError::Io(_)
            | JustMcpError::Watch(_)
            | JustMcpError::Execution { .. }
            | JustMcpError::ToolNotFound(_) => true,
            _ => false,
        }
    }
}

/// Structured error information for logging and debugging
#[derive(Debug, Clone)]
pub struct ErrorInfo {
    /// Error type classification
    pub error_type: String,
    /// User-friendly error message
    pub user_message: String,
    /// Technical details for debugging
    pub technical_details: String,
    /// Whether this is likely a user error vs system error
    pub is_user_error: bool,
    /// Whether retrying might succeed
    pub is_retryable: bool,
}

/// Error categories for monitoring and alerting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// User input or configuration errors
    UserError,
    /// System or infrastructure errors
    SystemError,
    /// Internal application errors
    InternalError,
    /// External dependency errors
    ExternalError,
}

impl ErrorAdapter {
    /// Categorize errors for monitoring and alerting
    pub fn categorize_error(error: &JustMcpError) -> ErrorCategory {
        match error {
            JustMcpError::TaskNotFound(_)
            | JustMcpError::InvalidParameter(_)
            | JustMcpError::Parse { .. }
            | JustMcpError::InvalidToolName(_) => ErrorCategory::UserError,

            JustMcpError::Io(_) | JustMcpError::Watch(_) | JustMcpError::Timeout(_) => {
                ErrorCategory::SystemError
            }

            JustMcpError::Registry(_)
            | JustMcpError::Server(_)
            | JustMcpError::Internal(_)
            | JustMcpError::Json(_)
            | JustMcpError::Regex(_) => ErrorCategory::InternalError,

            JustMcpError::Execution { .. }
            | JustMcpError::JustCommand(_)
            | JustMcpError::ToolNotFound(_)
            | JustMcpError::Other(_) => ErrorCategory::ExternalError,
        }
    }
}

/// Trait for framework-compatible error conversion
///
/// This trait allows other modules to easily convert their errors to
/// framework-compatible formats.
#[cfg(feature = "ultrafast-framework")]
pub trait ToMcpError {
    fn to_mcp_error(self) -> MCPError;
}

#[cfg(feature = "ultrafast-framework")]
impl ToMcpError for JustMcpError {
    fn to_mcp_error(self) -> MCPError {
        ErrorAdapter::to_mcp_error(self)
    }
}

#[cfg(feature = "ultrafast-framework")]
impl<T> ToMcpError for JustMcpResult<T> {
    fn to_mcp_error(self) -> MCPError {
        match self {
            Ok(_) => MCPError::internal_error(
                "Attempted to convert successful result to error".to_string(),
            ),
            Err(e) => ErrorAdapter::to_mcp_error(e),
        }
    }
}

/// Extension trait for Result types to provide framework conversion methods
#[cfg(feature = "ultrafast-framework")]
pub trait ResultExt<T> {
    fn to_mcp_result(self) -> MCPResult<T>;
}

#[cfg(feature = "ultrafast-framework")]
impl<T> ResultExt<T> for JustMcpResult<T> {
    fn to_mcp_result(self) -> MCPResult<T> {
        ErrorAdapter::to_mcp_result(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error as JustMcpError;

    #[test]
    fn test_error_info_extraction() {
        let error = JustMcpError::TaskNotFound("build".to_string());
        let info = ErrorAdapter::extract_error_info(&error);

        assert_eq!(info.error_type, "task_not_found");
        assert!(info.user_message.contains("build"));
        assert!(info.is_user_error);
        assert!(!info.is_retryable);
    }

    #[test]
    fn test_error_categorization() {
        assert_eq!(
            ErrorAdapter::categorize_error(&JustMcpError::TaskNotFound("test".to_string())),
            ErrorCategory::UserError
        );

        assert_eq!(
            ErrorAdapter::categorize_error(&JustMcpError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "test"
            ))),
            ErrorCategory::SystemError
        );

        assert_eq!(
            ErrorAdapter::categorize_error(&JustMcpError::Internal("test".to_string())),
            ErrorCategory::InternalError
        );
    }

    #[test]
    fn test_user_correctable_detection() {
        assert!(ErrorAdapter::is_user_correctable(
            &JustMcpError::TaskNotFound("build".to_string())
        ));
        assert!(ErrorAdapter::is_user_correctable(
            &JustMcpError::InvalidParameter("bad param".to_string())
        ));
        assert!(!ErrorAdapter::is_user_correctable(&JustMcpError::Internal(
            "internal".to_string()
        )));
    }

    #[test]
    fn test_retryable_detection() {
        assert!(ErrorAdapter::is_retryable(&JustMcpError::Timeout(
            "timeout".to_string()
        )));
        assert!(ErrorAdapter::is_retryable(&JustMcpError::Io(
            std::io::Error::new(std::io::ErrorKind::PermissionDenied, "permission")
        )));
        assert!(!ErrorAdapter::is_retryable(&JustMcpError::Parse {
            message: "syntax error".to_string(),
            line: 1,
            column: 1,
        }));
    }

    #[cfg(feature = "ultrafast-framework")]
    #[test]
    fn test_mcp_error_conversion() {
        let error = JustMcpError::TaskNotFound("build".to_string());
        let mcp_error = ErrorAdapter::to_mcp_error(error);

        // Verify the error message contains helpful information
        let error_msg = format!("{}", mcp_error);
        assert!(error_msg.contains("build"));
        assert!(error_msg.contains("not found"));
    }

    #[cfg(feature = "ultrafast-framework")]
    #[test]
    fn test_execution_result_conversion() {
        // Test successful execution result
        let success_result = ExecutionResult {
            success: true,
            exit_code: Some(0),
            stdout: "Task completed".to_string(),
            stderr: String::new(),
            error: None,
        };

        let mcp_result = ErrorAdapter::execution_result_to_mcp_result(success_result.clone());
        assert!(mcp_result.is_ok());
        assert_eq!(mcp_result.unwrap().success, true);

        // Test failed execution result
        let error_result = ExecutionResult {
            success: false,
            exit_code: Some(1),
            stdout: String::new(),
            stderr: "Command failed".to_string(),
            error: Some("Task execution failed".to_string()),
        };

        let mcp_error_result = ErrorAdapter::execution_result_to_mcp_result(error_result);
        assert!(mcp_error_result.is_err());

        let error_msg = format!("{}", mcp_error_result.unwrap_err());
        assert!(error_msg.contains("execution failed"));
        assert!(error_msg.contains("Command failed"));
    }

    #[test]
    fn test_error_info_structure() {
        let errors = vec![
            JustMcpError::TaskNotFound("test".to_string()),
            JustMcpError::InvalidParameter("bad".to_string()),
            JustMcpError::Execution {
                command: "just test".to_string(),
                exit_code: Some(1),
                stderr: "error output".to_string(),
            },
            JustMcpError::Parse {
                message: "syntax error".to_string(),
                line: 10,
                column: 5,
            },
        ];

        for error in errors {
            let info = ErrorAdapter::extract_error_info(&error);

            // Verify all fields are populated
            assert!(!info.error_type.is_empty());
            assert!(!info.user_message.is_empty());
            assert!(!info.technical_details.is_empty());

            // Verify user message is more friendly than technical details
            assert!(info.user_message.len() <= info.technical_details.len() * 2);
            // Rough heuristic
        }
    }

    #[cfg(feature = "ultrafast-framework")]
    #[test]
    fn test_comprehensive_error_conversion_quality() {
        // Test various error scenarios to ensure message quality is preserved
        let test_cases = vec![
            (
                JustMcpError::TaskNotFound("build".to_string()),
                "invalid_request", // Expected error type in framework
                vec!["build", "not found", "just --list"], // Expected content
            ),
            (
                JustMcpError::Execution {
                    command: "cargo build".to_string(),
                    exit_code: Some(1),
                    stderr: "compilation failed".to_string(),
                },
                "internal_error",
                vec!["cargo build", "exit code", "compilation failed"],
            ),
            (
                JustMcpError::Parse {
                    message: "unexpected token".to_string(),
                    line: 5,
                    column: 10,
                },
                "invalid_request",
                vec!["line 5", "column 10", "unexpected token", "syntax"],
            ),
            (
                JustMcpError::InvalidParameter("missing required field 'name'".to_string()),
                "invalid_params",
                vec!["parameter", "missing required field"],
            ),
        ];

        for (error, _expected_type, expected_content) in test_cases {
            let mcp_error = ErrorAdapter::to_mcp_error(error);
            let error_msg = format!("{}", mcp_error);

            // Verify error message contains expected content
            for content in expected_content {
                assert!(
                    error_msg.contains(content),
                    "Error message '{}' should contain '{}'",
                    error_msg,
                    content
                );
            }

            // Verify error message is helpful and actionable
            assert!(
                error_msg.len() > 20,
                "Error message should be substantial: '{}'",
                error_msg
            );
        }
    }

    #[test]
    fn test_error_categorization_consistency() {
        // Test that error categorization is consistent and logical
        let user_errors = vec![
            JustMcpError::TaskNotFound("test".to_string()),
            JustMcpError::InvalidParameter("bad".to_string()),
            JustMcpError::Parse {
                message: "syntax error".to_string(),
                line: 1,
                column: 1,
            },
        ];

        for error in user_errors {
            assert_eq!(
                ErrorAdapter::categorize_error(&error),
                ErrorCategory::UserError
            );
            assert!(ErrorAdapter::is_user_correctable(&error));
        }

        let system_errors = vec![
            JustMcpError::Io(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "permission denied",
            )),
            JustMcpError::Timeout("operation timed out".to_string()),
        ];

        for error in system_errors {
            assert_eq!(
                ErrorAdapter::categorize_error(&error),
                ErrorCategory::SystemError
            );
            assert!(!ErrorAdapter::is_user_correctable(&error));
            assert!(ErrorAdapter::is_retryable(&error));
        }
    }

    #[test]
    fn test_error_information_preservation() {
        // Ensure that important error information is never lost in conversion
        let original_error = JustMcpError::Execution {
            command: "just important-task --flag value".to_string(),
            exit_code: Some(127),
            stderr: "command not found: important-task".to_string(),
        };

        let error_info = ErrorAdapter::extract_error_info(&original_error);

        // Verify all important information is preserved
        assert!(error_info.technical_details.contains("important-task"));
        assert!(error_info.technical_details.contains("--flag value"));
        assert!(error_info.technical_details.contains("127"));
        assert!(error_info.technical_details.contains("command not found"));

        // Verify user message is more digestible but still informative
        assert!(
            error_info.user_message.contains("failed") || error_info.user_message.contains("error")
        );
    }
}
