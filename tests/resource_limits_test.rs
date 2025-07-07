use just_mcp::executor::TaskExecutor;
use just_mcp::resource_limits::{ResourceLimits, ResourceManager};
use just_mcp::security::SecurityConfig;
use just_mcp::types::{ExecutionContext, ExecutionRequest};
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;

#[tokio::test]
async fn test_concurrent_execution_limits() {
    // Test that a single executor enforces concurrent execution limits
    let resource_manager = Arc::new(ResourceManager::new(ResourceLimits {
        max_concurrent_executions: 2,
        ..Default::default()
    }));

    // Can start first execution
    assert!(resource_manager.can_execute().is_ok());
    let guard1 = resource_manager.start_execution();
    assert_eq!(resource_manager.current_execution_count(), 1);

    // Can start second execution
    assert!(resource_manager.can_execute().is_ok());
    let guard2 = resource_manager.start_execution();
    assert_eq!(resource_manager.current_execution_count(), 2);

    // Cannot start third execution
    assert!(resource_manager.can_execute().is_err());

    // Drop one guard
    drop(guard1);
    assert_eq!(resource_manager.current_execution_count(), 1);

    // Now can start another
    assert!(resource_manager.can_execute().is_ok());
    let _guard3 = resource_manager.start_execution();
    assert_eq!(resource_manager.current_execution_count(), 2);

    // Cleanup
    drop(guard2);
}

#[tokio::test]
async fn test_timeout_enforcement() {
    let temp_dir = TempDir::new().unwrap();
    let justfile_path = temp_dir.path().join("justfile");

    // Create a test justfile with a task that takes too long
    let content = r#"
timeout_test:
    sleep 3
    echo "Should not see this"
"#;
    fs::write(&justfile_path, content).unwrap();

    // Create resource limits with a short timeout
    let resource_limits = ResourceLimits {
        max_execution_time: Duration::from_secs(1),
        ..Default::default()
    };

    let security_config = SecurityConfig {
        allowed_paths: vec![temp_dir.path().to_path_buf()],
        ..Default::default()
    };

    let mut executor = TaskExecutor::new()
        .with_security_config(security_config)
        .with_resource_limits(resource_limits);

    let request = ExecutionRequest {
        tool_name: format!("just_timeout_test_{}", justfile_path.display()),
        parameters: HashMap::new(),
        context: ExecutionContext {
            working_directory: Some(temp_dir.path().to_string_lossy().to_string()),
            environment: HashMap::new(),
            timeout: None, // Use default from resource limits
        },
    };

    let result = executor.execute(request).await.unwrap();

    // Task should fail due to timeout
    assert!(!result.success);
    assert!(result.error.as_ref().unwrap().contains("timed out"));
    assert!(!result.stdout.contains("Should not see this"));
}

#[tokio::test]
async fn test_output_size_limits() {
    let temp_dir = TempDir::new().unwrap();
    let justfile_path = temp_dir.path().join("justfile");

    // Create a test justfile that generates large output
    let content = r#"
large_output:
    @for i in {1..1000}; do echo "Line $i: This is a test line with some content to make it longer"; done
"#;
    fs::write(&justfile_path, content).unwrap();

    // Create resource limits with small output size
    let resource_limits = ResourceLimits {
        max_output_size: 1024, // 1KB limit
        enforce_hard_limits: true,
        ..Default::default()
    };

    let security_config = SecurityConfig {
        allowed_paths: vec![temp_dir.path().to_path_buf()],
        ..Default::default()
    };

    let mut executor = TaskExecutor::new()
        .with_security_config(security_config)
        .with_resource_limits(resource_limits);

    let request = ExecutionRequest {
        tool_name: format!("just_large_output_{}", justfile_path.display()),
        parameters: HashMap::new(),
        context: ExecutionContext {
            working_directory: Some(temp_dir.path().to_string_lossy().to_string()),
            environment: HashMap::new(),
            timeout: Some(5),
        },
    };

    let result = executor.execute(request).await;

    // Should fail due to output size limit
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Output size"));
}

#[test]
fn test_resource_manager_guards() {
    let limits = ResourceLimits {
        max_concurrent_executions: 3,
        ..Default::default()
    };
    let manager = ResourceManager::new(limits);

    assert_eq!(manager.current_execution_count(), 0);

    {
        let _guard1 = manager.start_execution();
        assert_eq!(manager.current_execution_count(), 1);

        {
            let _guard2 = manager.start_execution();
            assert_eq!(manager.current_execution_count(), 2);

            let _guard3 = manager.start_execution();
            assert_eq!(manager.current_execution_count(), 3);

            // Should be at limit
            assert!(manager.can_execute().is_err());
        }
        // guard3 and guard2 dropped
        assert_eq!(manager.current_execution_count(), 1);
    }
    // guard1 dropped
    assert_eq!(manager.current_execution_count(), 0);

    // Can execute again
    assert!(manager.can_execute().is_ok());
}
