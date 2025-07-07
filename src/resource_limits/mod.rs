use crate::error::{Error, Result};
use std::time::Duration;
use tracing::{info, warn};

/// Resource limits configuration for task execution
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// Maximum execution time for a task
    pub max_execution_time: Duration,
    /// Maximum memory usage in bytes (if enforceable)
    pub max_memory_bytes: Option<usize>,
    /// Maximum CPU percentage (0-100)
    pub max_cpu_percent: Option<u8>,
    /// Maximum number of concurrent executions
    pub max_concurrent_executions: usize,
    /// Maximum output size in bytes (stdout + stderr)
    pub max_output_size: usize,
    /// Kill tasks that exceed limits (vs just warning)
    pub enforce_hard_limits: bool,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_execution_time: Duration::from_secs(300), // 5 minutes
            max_memory_bytes: None, // Not enforced by default
            max_cpu_percent: None, // Not enforced by default
            max_concurrent_executions: 10,
            max_output_size: 10 * 1024 * 1024, // 10MB
            enforce_hard_limits: true,
        }
    }
}

/// Manages resource limits and tracks usage
pub struct ResourceManager {
    limits: ResourceLimits,
    current_executions: std::sync::atomic::AtomicUsize,
}

impl ResourceManager {
    pub fn new(limits: ResourceLimits) -> Self {
        Self {
            limits,
            current_executions: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    pub fn with_default() -> Self {
        Self::new(ResourceLimits::default())
    }

    /// Check if we can start a new execution
    pub fn can_execute(&self) -> Result<()> {
        let current = self.current_executions.load(std::sync::atomic::Ordering::Relaxed);
        
        if current >= self.limits.max_concurrent_executions {
            return Err(Error::Other(format!(
                "Maximum concurrent executions ({}) reached",
                self.limits.max_concurrent_executions
            )));
        }
        
        Ok(())
    }

    /// Register the start of an execution
    pub fn start_execution(&self) -> ExecutionGuard {
        self.current_executions.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        info!("Started execution. Current count: {}", 
              self.current_executions.load(std::sync::atomic::Ordering::Relaxed));
        
        ExecutionGuard {
            manager: self,
        }
    }

    /// Get the configured timeout for executions
    pub fn get_timeout(&self) -> Duration {
        self.limits.max_execution_time
    }

    /// Check if output size is within limits
    pub fn check_output_size(&self, stdout_len: usize, stderr_len: usize) -> Result<()> {
        let total_size = stdout_len + stderr_len;
        
        if total_size > self.limits.max_output_size {
            let msg = format!(
                "Output size ({} bytes) exceeds limit ({} bytes)",
                total_size, self.limits.max_output_size
            );
            
            if self.limits.enforce_hard_limits {
                return Err(Error::Other(msg));
            } else {
                warn!("{}", msg);
            }
        }
        
        Ok(())
    }

    /// Get current execution count
    pub fn current_execution_count(&self) -> usize {
        self.current_executions.load(std::sync::atomic::Ordering::Relaxed)
    }
}

/// RAII guard for tracking execution lifecycle
pub struct ExecutionGuard<'a> {
    manager: &'a ResourceManager,
}

impl<'a> Drop for ExecutionGuard<'a> {
    fn drop(&mut self) {
        self.manager.current_executions.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
        info!("Finished execution. Current count: {}",
              self.manager.current_executions.load(std::sync::atomic::Ordering::Relaxed));
    }
}

/// Platform-specific resource limit enforcement
#[cfg(unix)]
pub mod platform {
    use super::*;
    use std::process::Command;
    
    /// Apply resource limits to a command (Unix-specific)
    pub fn apply_limits(cmd: &mut Command, limits: &ResourceLimits) {
        // On Unix, we can use ulimit-style limits
        // This is a simplified version - in production, consider using cgroups
        
        if let Some(memory_bytes) = limits.max_memory_bytes {
            // Set memory limit using ulimit (in KB)
            let memory_kb = memory_bytes / 1024;
            cmd.env("RLIMIT_AS", memory_kb.to_string());
        }
        
        // CPU limits are harder to enforce directly
        // In production, consider using nice/renice or cgroups
        if let Some(cpu_percent) = limits.max_cpu_percent {
            if cpu_percent < 100 {
                // Use nice to lower priority
                let nice_value = 19 - (cpu_percent as i32 * 19 / 100);
                cmd.arg("-n").arg(nice_value.to_string());
            }
        }
    }
}

#[cfg(windows)]
pub mod platform {
    use super::*;
    use std::process::Command;
    
    /// Apply resource limits to a command (Windows-specific)
    pub fn apply_limits(_cmd: &mut Command, _limits: &ResourceLimits) {
        // Windows resource limiting is more complex
        // Would need to use Job Objects API
        // For now, we just log a warning
        if _limits.max_memory_bytes.is_some() || _limits.max_cpu_percent.is_some() {
            warn!("Memory and CPU limits are not enforced on Windows");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_resource_limits_default() {
        let limits = ResourceLimits::default();
        assert_eq!(limits.max_execution_time, Duration::from_secs(300));
        assert_eq!(limits.max_concurrent_executions, 10);
        assert_eq!(limits.max_output_size, 10 * 1024 * 1024);
        assert!(limits.enforce_hard_limits);
    }
    
    #[test]
    fn test_resource_manager_concurrent_limit() {
        let limits = ResourceLimits {
            max_concurrent_executions: 2,
            ..Default::default()
        };
        let manager = ResourceManager::new(limits);
        
        // First two should succeed
        assert!(manager.can_execute().is_ok());
        let _guard1 = manager.start_execution();
        assert_eq!(manager.current_execution_count(), 1);
        
        assert!(manager.can_execute().is_ok());
        let _guard2 = manager.start_execution();
        assert_eq!(manager.current_execution_count(), 2);
        
        // Third should fail
        assert!(manager.can_execute().is_err());
        
        // Drop one guard
        drop(_guard1);
        assert_eq!(manager.current_execution_count(), 1);
        
        // Now we can execute again
        assert!(manager.can_execute().is_ok());
    }
    
    #[test]
    fn test_output_size_limits() {
        let limits = ResourceLimits {
            max_output_size: 1024,
            enforce_hard_limits: true,
            ..Default::default()
        };
        let manager = ResourceManager::new(limits);
        
        // Within limits
        assert!(manager.check_output_size(500, 500).is_ok());
        
        // Exceeds limits
        assert!(manager.check_output_size(600, 600).is_err());
    }
    
    #[test]
    fn test_soft_limits() {
        let limits = ResourceLimits {
            max_output_size: 1024,
            enforce_hard_limits: false,
            ..Default::default()
        };
        let manager = ResourceManager::new(limits);
        
        // Exceeds limits but only warns
        assert!(manager.check_output_size(600, 600).is_ok());
    }
}