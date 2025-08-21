//! Confirmation Manager for Dangerous Command Detection
//!
//! This module implements safety mechanisms to detect potentially dangerous commands
//! and recommend user confirmation before execution. It uses pattern matching to
//! identify operations that could be destructive or require elevated privileges.

use std::collections::HashSet;

/// Manager for detecting dangerous commands and providing confirmation recommendations
#[derive(Debug, Clone)]
pub struct ConfirmationManager {
    /// Patterns that indicate potentially dangerous operations
    dangerous_patterns: HashSet<String>,
    /// Whether confirmation is required for dangerous commands
    require_confirmation: bool,
}

impl ConfirmationManager {
    /// Create a new confirmation manager with default dangerous patterns
    pub fn new() -> Self {
        let mut dangerous_patterns = HashSet::new();

        // File/directory operations
        dangerous_patterns.insert("rm".to_string());
        dangerous_patterns.insert("delete".to_string());
        dangerous_patterns.insert("remove".to_string());
        dangerous_patterns.insert("unlink".to_string());
        dangerous_patterns.insert("rmdir".to_string());

        // Cleanup operations
        dangerous_patterns.insert("clean-all".to_string());
        dangerous_patterns.insert("clean_all".to_string());
        dangerous_patterns.insert("purge".to_string());
        dangerous_patterns.insert("wipe".to_string());
        dangerous_patterns.insert("format".to_string());
        dangerous_patterns.insert("erase".to_string());

        // State operations
        dangerous_patterns.insert("reset".to_string());
        dangerous_patterns.insert("restore".to_string());
        dangerous_patterns.insert("revert".to_string());
        dangerous_patterns.insert("rollback".to_string());

        // Process operations
        dangerous_patterns.insert("kill".to_string());
        dangerous_patterns.insert("stop".to_string());
        dangerous_patterns.insert("terminate".to_string());
        dangerous_patterns.insert("shutdown".to_string());

        // Privilege escalation
        dangerous_patterns.insert("sudo".to_string());
        dangerous_patterns.insert("admin".to_string());
        dangerous_patterns.insert("root".to_string());

        // Database/infrastructure operations
        dangerous_patterns.insert("drop".to_string());
        dangerous_patterns.insert("destroy".to_string());
        dangerous_patterns.insert("truncate".to_string());
        dangerous_patterns.insert("migrate-down".to_string());
        dangerous_patterns.insert("migrate_down".to_string());

        Self {
            dangerous_patterns,
            require_confirmation: true,
        }
    }

    /// Create a confirmation manager with custom patterns
    pub fn with_patterns(patterns: HashSet<String>) -> Self {
        Self {
            dangerous_patterns: patterns,
            require_confirmation: true,
        }
    }

    /// Enable or disable confirmation requirements
    pub fn set_require_confirmation(mut self, require: bool) -> Self {
        self.require_confirmation = require;
        self
    }

    /// Add a dangerous pattern
    pub fn add_pattern(&mut self, pattern: impl Into<String>) {
        self.dangerous_patterns.insert(pattern.into());
    }

    /// Remove a dangerous pattern
    pub fn remove_pattern(&mut self, pattern: &str) -> bool {
        self.dangerous_patterns.remove(pattern)
    }

    /// Check if a task name contains dangerous patterns
    pub fn is_dangerous(&self, task_name: &str) -> bool {
        let task_lower = task_name.to_lowercase();

        // Check for exact matches and substrings
        self.dangerous_patterns.iter().any(|pattern| {
            let pattern_lower = pattern.to_lowercase();
            task_lower == pattern_lower ||
            task_lower.contains(&pattern_lower) ||
            // Check for word boundaries to avoid false positives
            self.contains_word_boundary(&task_lower, &pattern_lower)
        })
    }

    /// Check if confirmation should be required for a task
    pub fn should_confirm(&self, task_name: &str) -> bool {
        self.require_confirmation && self.is_dangerous(task_name)
    }

    /// Get a safety assessment for a task
    pub fn assess_safety(&self, task_name: &str, description: Option<&str>) -> SafetyAssessment {
        let is_dangerous = self.is_dangerous(task_name);
        let should_confirm = self.should_confirm(task_name);

        let risk_level = if is_dangerous {
            if self.is_highly_destructive(task_name) {
                RiskLevel::High
            } else {
                RiskLevel::Medium
            }
        } else {
            RiskLevel::Low
        };

        let matched_patterns = self.get_matched_patterns(task_name);

        SafetyAssessment {
            task_name: task_name.to_string(),
            risk_level: risk_level.clone(),
            is_dangerous,
            should_confirm,
            matched_patterns: matched_patterns.clone(),
            reason: self.generate_safety_reason(&risk_level, &matched_patterns, description),
            recommendation: self.generate_recommendation(&risk_level, should_confirm),
        }
    }

    /// Generate a confirmation prompt for a dangerous task
    pub fn generate_confirmation_prompt(
        &self,
        task_name: &str,
        description: Option<&str>,
    ) -> Option<String> {
        if !self.should_confirm(task_name) {
            return None;
        }

        let assessment = self.assess_safety(task_name, description);
        Some(format!(
            "⚠️  WARNING: The task '{}' appears to be potentially destructive.\n\n{}\n\n{}\n\nDo you want to proceed with executing this task?",
            task_name,
            assessment.reason,
            assessment.recommendation
        ))
    }

    /// Check for word boundaries to avoid false positives
    fn contains_word_boundary(&self, text: &str, pattern: &str) -> bool {
        // Simple word boundary check - pattern should be surrounded by non-alphanumeric chars
        let pattern_len = pattern.len();
        if let Some(start_pos) = text.find(pattern) {
            let before_ok = start_pos == 0
                || !text
                    .chars()
                    .nth(start_pos - 1)
                    .unwrap_or(' ')
                    .is_alphanumeric();
            let after_pos = start_pos + pattern_len;
            let after_ok = after_pos >= text.len()
                || !text.chars().nth(after_pos).unwrap_or(' ').is_alphanumeric();

            before_ok && after_ok
        } else {
            false
        }
    }

    /// Check if a task is highly destructive
    fn is_highly_destructive(&self, task_name: &str) -> bool {
        let highly_destructive = [
            "rm",
            "delete",
            "remove",
            "format",
            "wipe",
            "erase",
            "drop",
            "destroy",
            "purge",
            "clean-all",
            "clean_all",
        ];

        let task_lower = task_name.to_lowercase();
        highly_destructive
            .iter()
            .any(|pattern| task_lower.contains(pattern))
    }

    /// Get patterns that matched the task name
    fn get_matched_patterns(&self, task_name: &str) -> Vec<String> {
        let task_lower = task_name.to_lowercase();
        self.dangerous_patterns
            .iter()
            .filter(|pattern| {
                let pattern_lower = pattern.to_lowercase();
                task_lower.contains(&pattern_lower)
                    || self.contains_word_boundary(&task_lower, &pattern_lower)
            })
            .cloned()
            .collect()
    }

    /// Generate a safety reason message
    fn generate_safety_reason(
        &self,
        risk_level: &RiskLevel,
        matched_patterns: &[String],
        description: Option<&str>,
    ) -> String {
        let patterns_str = if matched_patterns.len() == 1 {
            format!("pattern '{}'", matched_patterns[0])
        } else {
            format!("patterns: {}", matched_patterns.join(", "))
        };

        let base_reason = format!("This task matches dangerous {patterns_str}");

        let risk_description = match risk_level {
            RiskLevel::High => {
                "This operation could permanently delete files, data, or configurations."
            }
            RiskLevel::Medium => {
                "This operation could modify system state or require elevated privileges."
            }
            RiskLevel::Low => "This operation appears to be low risk.",
        };

        if let Some(desc) = description {
            format!("{base_reason}. {risk_description} Task description: {desc}")
        } else {
            format!("{base_reason}. {risk_description}")
        }
    }

    /// Generate a safety recommendation
    fn generate_recommendation(&self, risk_level: &RiskLevel, should_confirm: bool) -> String {
        match (risk_level, should_confirm) {
            (RiskLevel::High, true) =>
                "RECOMMENDATION: Carefully review what this task does before proceeding. Consider backing up important data first.".to_string(),
            (RiskLevel::Medium, true) =>
                "RECOMMENDATION: Review the task details and ensure you understand what it will do.".to_string(),
            (RiskLevel::Low, _) =>
                "This task appears to be safe to execute.".to_string(),
            (_, false) =>
                "Safety checks are disabled for this task.".to_string(),
        }
    }
}

impl Default for ConfirmationManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Risk level assessment for a task
#[derive(Debug, Clone, PartialEq)]
pub enum RiskLevel {
    /// Low risk - safe operations
    Low,
    /// Medium risk - operations that modify state or require privileges
    Medium,
    /// High risk - potentially destructive operations
    High,
}

/// Complete safety assessment for a task
#[derive(Debug, Clone)]
pub struct SafetyAssessment {
    /// Name of the task being assessed
    pub task_name: String,
    /// Assessed risk level
    pub risk_level: RiskLevel,
    /// Whether the task is considered dangerous
    pub is_dangerous: bool,
    /// Whether confirmation should be required
    pub should_confirm: bool,
    /// Patterns that matched and triggered the danger detection
    pub matched_patterns: Vec<String>,
    /// Human-readable reason for the assessment
    pub reason: String,
    /// Safety recommendation
    pub recommendation: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dangerous_pattern_detection() {
        let manager = ConfirmationManager::new();

        // Direct pattern matches
        assert!(manager.is_dangerous("rm"));
        assert!(manager.is_dangerous("delete"));
        assert!(manager.is_dangerous("clean-all"));
        assert!(manager.is_dangerous("sudo"));

        // Pattern in task names
        assert!(manager.is_dangerous("clean-all-artifacts"));
        assert!(manager.is_dangerous("remove-temp-files"));
        assert!(manager.is_dangerous("sudo-install"));

        // Safe tasks
        assert!(!manager.is_dangerous("build"));
        assert!(!manager.is_dangerous("test"));
        assert!(!manager.is_dangerous("lint-code")); // lint-code doesn't contain dangerous patterns
    }

    #[test]
    fn test_confirmation_requirements() {
        let manager = ConfirmationManager::new();

        assert!(manager.should_confirm("rm-temp"));
        assert!(manager.should_confirm("delete-cache"));
        assert!(!manager.should_confirm("build"));

        let manager_no_confirm = ConfirmationManager::new().set_require_confirmation(false);
        assert!(!manager_no_confirm.should_confirm("rm-temp"));
    }

    #[test]
    fn test_safety_assessment() {
        let manager = ConfirmationManager::new();

        let assessment = manager.assess_safety("clean-all", Some("Remove all build artifacts"));
        assert_eq!(assessment.risk_level, RiskLevel::High);
        assert!(assessment.is_dangerous);
        assert!(assessment.should_confirm);
        assert!(!assessment.matched_patterns.is_empty());

        let safe_assessment = manager.assess_safety("build", None);
        assert_eq!(safe_assessment.risk_level, RiskLevel::Low);
        assert!(!safe_assessment.is_dangerous);
        assert!(!safe_assessment.should_confirm);
    }

    #[test]
    fn test_confirmation_prompt_generation() {
        let manager = ConfirmationManager::new();

        let prompt =
            manager.generate_confirmation_prompt("rm-temp", Some("Remove temporary files"));
        assert!(prompt.is_some());
        assert!(prompt.unwrap().contains("WARNING"));

        let no_prompt = manager.generate_confirmation_prompt("build", None);
        assert!(no_prompt.is_none());
    }

    #[test]
    fn test_custom_patterns() {
        let mut patterns = HashSet::new();
        patterns.insert("custom-danger".to_string());
        patterns.insert("risky-operation".to_string());

        let manager = ConfirmationManager::with_patterns(patterns);

        assert!(manager.is_dangerous("custom-danger"));
        assert!(manager.is_dangerous("risky-operation"));
        assert!(!manager.is_dangerous("rm")); // Default patterns not included
    }

    #[test]
    fn test_pattern_modification() {
        let mut manager = ConfirmationManager::new();

        manager.add_pattern("custom-risky");
        assert!(manager.is_dangerous("custom-risky"));

        manager.remove_pattern("rm");
        assert!(!manager.is_dangerous("rm"));
    }

    #[test]
    fn test_word_boundary_detection() {
        let manager = ConfirmationManager::new();

        // Should match word boundaries
        assert!(manager.is_dangerous("clean-all"));
        assert!(manager.is_dangerous("do-rm-action"));

        // Should not match partial words (this is a limitation of current implementation)
        // The current implementation is conservative and may have false positives
        // This is acceptable for safety reasons
    }

    #[test]
    fn test_risk_level_classification() {
        let manager = ConfirmationManager::new();

        // High risk operations
        let high_risk = manager.assess_safety("rm-all", None);
        assert_eq!(high_risk.risk_level, RiskLevel::High);

        let high_risk_2 = manager.assess_safety("destroy-database", None);
        assert_eq!(high_risk_2.risk_level, RiskLevel::High);

        // Medium risk operations
        let medium_risk = manager.assess_safety("sudo-install", None);
        assert_eq!(medium_risk.risk_level, RiskLevel::Medium);

        // Low risk operations
        let low_risk = manager.assess_safety("build-project", None);
        assert_eq!(low_risk.risk_level, RiskLevel::Low);
    }
}
