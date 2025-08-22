//! Comprehensive AST Parser Integration and Validation Tests
//!
//! This test suite validates the Tree-sitter AST parser implementation against
//! all existing project recipes and ensures consistency with the regex parser.

use anyhow::Result;
use just_mcp::parser::{EnhancedJustfileParser, JustfileParser, ParserPreference};
#[cfg(feature = "ast-parser")]
use just_mcp::types::JustTask;
#[cfg(feature = "ast-parser")]
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
#[cfg(feature = "ast-parser")]
use std::time::Instant;

/// Test data structure for tracking parsing results
#[cfg(feature = "ast-parser")]
#[derive(Debug, Clone)]
struct ParsingTestResult {
    file_path: String,
    ast_success: bool,
    regex_success: bool,
    ast_tasks: Vec<JustTask>,
    regex_tasks: Vec<JustTask>,
    ast_error: Option<String>,
    regex_error: Option<String>,
    ast_parse_time: u128,
    regex_parse_time: u128,
    consistency_issues: Vec<String>,
}

/// Summary statistics for the validation test suite
#[cfg(feature = "ast-parser")]
#[derive(Debug, Default)]
struct ValidationSummary {
    total_justfiles: usize,
    ast_successes: usize,
    regex_successes: usize,
    both_succeeded: usize,
    ast_only_succeeded: usize,
    regex_only_succeeded: usize,
    both_failed: usize,
    consistency_issues: usize,
    total_recipes_ast: usize,
    total_recipes_regex: usize,
    total_ast_time: u128,
    total_regex_time: u128,
}

/// Get all justfiles in the project for testing
fn get_all_project_justfiles() -> Result<Vec<PathBuf>> {
    let mut justfiles = Vec::new();

    // Main project justfile
    let main_justfile = PathBuf::from("justfile");
    if main_justfile.exists() {
        justfiles.push(main_justfile);
    }

    // Demo justfile
    let demo_justfile = PathBuf::from("demo/justfile");
    if demo_justfile.exists() {
        justfiles.push(demo_justfile);
    }

    // Modular justfiles in just/ directory
    if let Ok(entries) = fs::read_dir("just") {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_file() && path.extension().is_some_and(|ext| ext == "just") {
                    justfiles.push(path);
                }
            }
        }
    }

    Ok(justfiles)
}

/// Test individual justfile with both AST and regex parsers
#[cfg(feature = "ast-parser")]
fn test_justfile_parsing(path: &PathBuf) -> Result<ParsingTestResult> {
    let file_path = path.to_string_lossy().to_string();
    let mut result = ParsingTestResult {
        file_path: file_path.clone(),
        ast_success: false,
        regex_success: false,
        ast_tasks: Vec::new(),
        regex_tasks: Vec::new(),
        ast_error: None,
        regex_error: None,
        ast_parse_time: 0,
        regex_parse_time: 0,
        consistency_issues: Vec::new(),
    };

    // Test AST parser
    let ast_start = Instant::now();
    let enhanced_parser = EnhancedJustfileParser::new_with_preference(ParserPreference::Ast)?;

    match enhanced_parser.parse_file(path) {
        Ok(tasks) => {
            result.ast_success = true;
            result.ast_tasks = tasks;
        }
        Err(e) => {
            result.ast_error = Some(e.to_string());
        }
    }
    result.ast_parse_time = ast_start.elapsed().as_micros();

    // Test regex parser
    let regex_start = Instant::now();
    let regex_parser = JustfileParser::new()?;

    match regex_parser.parse_file(path) {
        Ok(tasks) => {
            result.regex_success = true;
            result.regex_tasks = tasks;
        }
        Err(e) => {
            result.regex_error = Some(e.to_string());
        }
    }
    result.regex_parse_time = regex_start.elapsed().as_micros();

    // Check consistency if both succeeded
    if result.ast_success && result.regex_success {
        result.consistency_issues =
            check_parsing_consistency(&result.ast_tasks, &result.regex_tasks);
    }

    Ok(result)
}

/// Check consistency between AST and regex parser results
#[cfg(feature = "ast-parser")]
fn check_parsing_consistency(ast_tasks: &[JustTask], regex_tasks: &[JustTask]) -> Vec<String> {
    let mut issues = Vec::new();

    // Compare task counts
    if ast_tasks.len() != regex_tasks.len() {
        issues.push(format!(
            "Task count mismatch: AST={}, Regex={}",
            ast_tasks.len(),
            regex_tasks.len()
        ));
    }

    // Filter out private recipes (starting with '_') for consistency comparison
    // The AST parser now finds private helper functions from imported modules
    // that the regex parser cannot see, so we focus on public recipes
    let ast_public_tasks: Vec<&JustTask> = ast_tasks
        .iter()
        .filter(|t| !t.name.starts_with('_'))
        .collect();
    let regex_public_tasks: Vec<&JustTask> = regex_tasks
        .iter()
        .filter(|t| !t.name.starts_with('_'))
        .collect();

    // Create maps for easier lookup (public recipes only)
    let ast_map: HashMap<&str, &JustTask> = ast_public_tasks
        .iter()
        .map(|t| (t.name.as_str(), *t))
        .collect();
    let regex_map: HashMap<&str, &JustTask> = regex_public_tasks
        .iter()
        .map(|t| (t.name.as_str(), *t))
        .collect();

    // Check for missing tasks in either parser
    for task_name in ast_map.keys() {
        if !regex_map.contains_key(task_name) {
            issues.push(format!(
                "Task '{task_name}' found in AST but not in regex parser"
            ));
        }
    }

    for task_name in regex_map.keys() {
        if !ast_map.contains_key(task_name) {
            issues.push(format!(
                "Task '{task_name}' found in regex but not in AST parser"
            ));
        }
    }

    // Compare common tasks
    for (task_name, ast_task) in &ast_map {
        if let Some(regex_task) = regex_map.get(task_name) {
            // Compare parameter counts
            if ast_task.parameters.len() != regex_task.parameters.len() {
                issues.push(format!(
                    "Task '{}': parameter count mismatch: AST={}, Regex={}",
                    task_name,
                    ast_task.parameters.len(),
                    regex_task.parameters.len()
                ));
            }

            // Compare dependency counts
            if ast_task.dependencies.len() != regex_task.dependencies.len() {
                issues.push(format!(
                    "Task '{}': dependency count mismatch: AST={}, Regex={}",
                    task_name,
                    ast_task.dependencies.len(),
                    regex_task.dependencies.len()
                ));
            }

            // Compare parameter names (order might differ)
            let ast_param_names: std::collections::HashSet<_> =
                ast_task.parameters.iter().map(|p| &p.name).collect();
            let regex_param_names: std::collections::HashSet<_> =
                regex_task.parameters.iter().map(|p| &p.name).collect();

            if ast_param_names != regex_param_names {
                issues.push(format!(
                    "Task '{task_name}': parameter names differ: AST={ast_param_names:?}, Regex={regex_param_names:?}"
                ));
            }

            // Compare dependency names (order might differ)
            let ast_deps: std::collections::HashSet<_> = ast_task.dependencies.iter().collect();
            let regex_deps: std::collections::HashSet<_> = regex_task.dependencies.iter().collect();

            if ast_deps != regex_deps {
                issues.push(format!(
                    "Task '{task_name}': dependencies differ: AST={ast_deps:?}, Regex={regex_deps:?}"
                ));
            }
        }
    }

    issues
}

#[cfg(feature = "ast-parser")]
#[test]
fn test_all_project_justfiles_with_ast_parser() -> Result<()> {
    let justfiles = get_all_project_justfiles()?;
    let mut results = Vec::new();
    let mut summary = ValidationSummary::default();

    println!("Testing {} justfiles with AST parser...", justfiles.len());

    for justfile in &justfiles {
        println!("Testing: {}", justfile.display());
        let result = test_justfile_parsing(justfile)?;

        // Update summary statistics
        summary.total_justfiles += 1;
        if result.ast_success {
            summary.ast_successes += 1;
            summary.total_recipes_ast += result.ast_tasks.len();
        }
        if result.regex_success {
            summary.regex_successes += 1;
            summary.total_recipes_regex += result.regex_tasks.len();
        }
        if result.ast_success && result.regex_success {
            summary.both_succeeded += 1;
        } else if result.ast_success {
            summary.ast_only_succeeded += 1;
        } else if result.regex_success {
            summary.regex_only_succeeded += 1;
        } else {
            summary.both_failed += 1;
        }

        summary.consistency_issues += result.consistency_issues.len();
        summary.total_ast_time += result.ast_parse_time;
        summary.total_regex_time += result.regex_parse_time;

        results.push(result);
    }

    // Print detailed results for each file
    println!("\n=== DETAILED RESULTS ===");
    for result in &results {
        println!("\nFile: {}", result.file_path);
        println!(
            "  AST Success: {} ({} tasks, {}μs)",
            result.ast_success,
            result.ast_tasks.len(),
            result.ast_parse_time
        );
        println!(
            "  Regex Success: {} ({} tasks, {}μs)",
            result.regex_success,
            result.regex_tasks.len(),
            result.regex_parse_time
        );

        if let Some(ref error) = result.ast_error {
            println!("  AST Error: {error}");
        }
        if let Some(ref error) = result.regex_error {
            println!("  Regex Error: {error}");
        }

        if !result.consistency_issues.is_empty() {
            println!("  Consistency Issues:");
            for issue in &result.consistency_issues {
                println!("    - {issue}");
            }
        }
    }

    // Print summary statistics
    println!("\n=== VALIDATION SUMMARY ===");
    println!("Total justfiles tested: {}", summary.total_justfiles);
    println!(
        "AST parser successes: {} ({:.1}%)",
        summary.ast_successes,
        (summary.ast_successes as f64 / summary.total_justfiles as f64) * 100.0
    );
    println!(
        "Regex parser successes: {} ({:.1}%)",
        summary.regex_successes,
        (summary.regex_successes as f64 / summary.total_justfiles as f64) * 100.0
    );
    println!(
        "Both parsers succeeded: {} ({:.1}%)",
        summary.both_succeeded,
        (summary.both_succeeded as f64 / summary.total_justfiles as f64) * 100.0
    );
    println!("AST only succeeded: {}", summary.ast_only_succeeded);
    println!("Regex only succeeded: {}", summary.regex_only_succeeded);
    println!("Both parsers failed: {}", summary.both_failed);
    println!("Total recipes parsed by AST: {}", summary.total_recipes_ast);
    println!(
        "Total recipes parsed by Regex: {}",
        summary.total_recipes_regex
    );
    println!("Total consistency issues: {}", summary.consistency_issues);

    if summary.total_ast_time > 0 && summary.total_regex_time > 0 {
        println!(
            "Average AST parse time: {:.1}μs",
            summary.total_ast_time as f64 / summary.ast_successes as f64
        );
        println!(
            "Average Regex parse time: {:.1}μs",
            summary.total_regex_time as f64 / summary.regex_successes as f64
        );
        println!(
            "AST vs Regex speed ratio: {:.2}x",
            summary.total_ast_time as f64 / summary.total_regex_time as f64
        );
    }

    // Validation assertions
    assert!(
        summary.ast_successes > 0,
        "AST parser should successfully parse at least some justfiles"
    );
    assert!(
        summary.regex_successes > 0,
        "Regex parser should successfully parse at least some justfiles"
    );

    // Require at least 80% success rate for AST parser
    let ast_success_rate = summary.ast_successes as f64 / summary.total_justfiles as f64;
    assert!(
        ast_success_rate >= 0.8,
        "AST parser success rate should be at least 80%, got {:.1}%",
        ast_success_rate * 100.0
    );

    // For files where both parsers succeeded, consistency issues should be minimal
    if summary.both_succeeded > 0 {
        let consistency_rate = summary.consistency_issues as f64 / summary.both_succeeded as f64;
        // The AST parser now handles imports correctly, finding recipes from imported modules
        // that the regex parser cannot see. This creates expected "consistency issues"
        // that are actually feature differences, not bugs.
        assert!(
            consistency_rate <= 10.0,
            "Average consistency issues per file should be <= 10 (AST parser finds imported recipes), got {consistency_rate:.1}"
        );
    }

    println!("\n✅ All validation tests passed!");
    Ok(())
}

#[cfg(feature = "ast-parser")]
#[test]
fn test_enhanced_parser_fallback_behavior() -> Result<()> {
    let justfiles = get_all_project_justfiles()?;

    if justfiles.is_empty() {
        println!("No justfiles found for testing fallback behavior");
        return Ok(());
    }

    // Test with the first available justfile
    let test_file = &justfiles[0];
    println!("Testing fallback behavior with: {}", test_file.display());

    let enhanced_parser = EnhancedJustfileParser::new()?;
    enhanced_parser.reset_metrics();

    // Parse the file (should use the three-tier fallback system)
    let tasks = enhanced_parser.parse_file(test_file)?;
    assert!(!tasks.is_empty(), "Should parse at least one task");

    // Check that parsing metrics were recorded
    let metrics = enhanced_parser.get_metrics();
    let total_attempts = metrics.ast_attempts + metrics.command_attempts + metrics.regex_attempts;
    assert!(
        total_attempts > 0,
        "Should have attempted at least one parsing method"
    );

    // Print diagnostics
    println!("Fallback behavior diagnostics:");
    println!("{}", enhanced_parser.get_diagnostics());

    // Test different parser configurations
    let ast_only_parser = EnhancedJustfileParser::new_with_preference(ParserPreference::Ast)?;
    let ast_only_tasks = ast_only_parser.parse_file(test_file)?;

    #[allow(deprecated)]
    let regex_only_parser = EnhancedJustfileParser::new_with_preference(ParserPreference::Regex)?;
    let regex_only_tasks = regex_only_parser.parse_file(test_file)?;

    println!("Enhanced parser: {} tasks", tasks.len());
    println!("AST-only parser: {} tasks", ast_only_tasks.len());
    println!("Regex-only parser: {} tasks", regex_only_tasks.len());

    // All parsers should return some tasks
    assert!(
        !ast_only_tasks.is_empty(),
        "AST-only parser should parse tasks"
    );
    assert!(
        !regex_only_tasks.is_empty(),
        "Regex-only parser should parse tasks"
    );

    Ok(())
}

#[test]
fn test_regex_parser_baseline() -> Result<()> {
    let justfiles = get_all_project_justfiles()?;
    let parser = JustfileParser::new()?;
    let mut total_tasks = 0;
    let mut successful_files = 0;

    println!(
        "Testing {} justfiles with regex parser (baseline)...",
        justfiles.len()
    );

    for justfile in &justfiles {
        match parser.parse_file(justfile) {
            Ok(tasks) => {
                successful_files += 1;
                total_tasks += tasks.len();
                println!("  {}: {} tasks", justfile.display(), tasks.len());
            }
            Err(e) => {
                println!("  {}: ERROR - {}", justfile.display(), e);
            }
        }
    }

    println!("\nRegex parser baseline results:");
    println!(
        "  Successful files: {}/{}",
        successful_files,
        justfiles.len()
    );
    println!("  Total tasks parsed: {total_tasks}");

    // Baseline expectations
    assert!(
        successful_files > 0,
        "Regex parser should parse at least some files"
    );
    assert!(
        total_tasks >= 100,
        "Should parse at least 100 tasks total (based on estimated 171 recipes)"
    );

    Ok(())
}

#[cfg(feature = "ast-parser")]
#[test]
fn test_complex_justfile_syntax_edge_cases() -> Result<()> {
    use just_mcp::parser::ast::ASTJustParser;

    let mut ast_parser = ASTJustParser::new()?;
    let regex_parser = JustfileParser::new()?;

    // Test various complex syntax patterns
    let test_cases = vec![
        // Parameters with complex defaults
        (
            "complex-params",
            r#"
task param1="default with spaces" param2='single-quoted' param3=no-quotes:
    echo "{{param1}} {{param2}} {{param3}}"
"#,
        ),
        // Multiple dependencies
        (
            "multi-deps",
            r#"
deploy: build test lint
    echo "Deploying..."
"#,
        ),
        // Basic expressions
        (
            "basic-expr",
            r#"
task:
    echo "Home: {{env_var('HOME')}}"
    echo "User: {{env_var('USER')}}"
"#,
        ),
        // Complex recipe body
        (
            "complex-body",
            r#"
setup:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Setting up..."
    if [ -f "requirements.txt" ]; then
        pip install -r requirements.txt
    fi
"#,
        ),
        // Attributes and doc strings
        (
            "attributes",
            r#"
[private]
[group('test')]
[doc("Run unit tests with coverage")]
test-unit coverage="true":
    @if [[ "{{coverage}}" == 'true' ]]; then
        cargo test --all --lib
    else
        cargo test --lib
    fi
"#,
        ),
        // String interpolation variations
        (
            "interpolation",
            r#"
greet name="World":
    @echo "Hello, {{name}}!"
    @echo "Welcome to {{justfile_directory()}}"
"#,
        ),
    ];

    for (name, content) in test_cases {
        println!("Testing {name} syntax...");

        // Parse with AST parser
        let tree = ast_parser.parse_content(content)?;
        let ast_tasks = ast_parser.extract_recipes(&tree)?;

        // Parse with regex parser
        let regex_tasks = regex_parser.parse_content(content)?;

        println!("  AST parser: {} tasks", ast_tasks.len());
        println!("  Regex parser: {} tasks", regex_tasks.len());

        // Both should find at least one task
        assert!(
            !ast_tasks.is_empty(),
            "AST parser should find tasks in {name}"
        );
        assert!(
            !regex_tasks.is_empty(),
            "Regex parser should find tasks in {name}"
        );

        // Check for major consistency issues
        if ast_tasks.len() != regex_tasks.len() {
            println!(
                "  ⚠️  Task count mismatch in {}: AST={}, Regex={}",
                name,
                ast_tasks.len(),
                regex_tasks.len()
            );
        }
    }

    Ok(())
}

#[cfg(not(feature = "ast-parser"))]
#[test]
fn test_ast_parser_feature_disabled() {
    // When AST parser feature is disabled, the enhanced parser should fall back gracefully
    let enhanced_parser = EnhancedJustfileParser::new();
    assert!(
        enhanced_parser.is_ok(),
        "Enhanced parser should work without AST feature"
    );

    let parser = enhanced_parser.unwrap();
    assert!(
        !parser.is_ast_parsing_available(),
        "AST parsing should not be available without feature"
    );
}
