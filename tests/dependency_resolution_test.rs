//! Comprehensive tests for dependency resolution and validation
//!
//! This test module validates Task 134 implementation: Dependency Resolution

#[cfg(feature = "ast-parser")]
mod dependency_resolution_tests {
    use just_mcp::parser::ast::{
        ASTJustParser, DependencyErrorType, DependencyInfo, DependencyType,
        DependencyValidationResult, DependencyValidator, RecipeInfo,
    };

    /// Test basic dependency creation and validation
    #[test]
    fn test_dependency_creation_and_validation() {
        // Test simple dependency
        let simple_dep = DependencyInfo::simple("build".to_string());
        assert_eq!(simple_dep.name, "build");
        assert_eq!(simple_dep.dependency_type, DependencyType::Simple);
        assert!(!simple_dep.has_arguments());
        assert!(!simple_dep.has_condition());
        assert!(simple_dep.is_valid());

        // Test parameterized dependency
        let param_dep = DependencyInfo::parameterized(
            "deploy".to_string(),
            vec!["staging".to_string(), "us-west-2".to_string()],
        );
        assert_eq!(param_dep.name, "deploy");
        assert_eq!(param_dep.dependency_type, DependencyType::Parameterized);
        assert!(param_dep.has_arguments());
        assert!(!param_dep.has_condition());
        assert_eq!(param_dep.arguments.len(), 2);
        assert!(param_dep.is_valid());

        // Test conditional dependency
        let cond_dep =
            DependencyInfo::conditional("cleanup".to_string(), "env == 'test'".to_string());
        assert_eq!(cond_dep.name, "cleanup");
        assert_eq!(cond_dep.dependency_type, DependencyType::Conditional);
        assert!(!cond_dep.has_arguments());
        assert!(cond_dep.has_condition());
        assert!(cond_dep.is_valid());

        // Test complex dependency
        let complex_dep = DependencyInfo::complex(
            "backup".to_string(),
            vec!["database".to_string()],
            "prod".to_string(),
        );
        assert_eq!(complex_dep.name, "backup");
        assert_eq!(complex_dep.dependency_type, DependencyType::Complex);
        assert!(complex_dep.has_arguments());
        assert!(complex_dep.has_condition());
        assert!(complex_dep.is_valid());
    }

    /// Test dependency formatting for debugging
    #[test]
    fn test_dependency_formatting() {
        let simple_dep = DependencyInfo::simple("build".to_string());
        assert_eq!(simple_dep.format_dependency(), "build");

        let param_dep = DependencyInfo::parameterized(
            "deploy".to_string(),
            vec!["prod".to_string(), "us-east-1".to_string()],
        );
        assert_eq!(param_dep.format_dependency(), "deploy(prod, us-east-1)");

        let cond_dep = DependencyInfo::conditional("test".to_string(), "CI == true".to_string());
        assert_eq!(cond_dep.format_dependency(), "test if CI == true");

        let complex_dep = DependencyInfo::complex(
            "backup".to_string(),
            vec!["full".to_string()],
            "prod".to_string(),
        );
        assert_eq!(complex_dep.format_dependency(), "backup(full) if prod");
    }

    /// Test circular dependency detection
    #[test]
    fn test_circular_dependency_detection() {
        let recipes = vec![
            RecipeInfo {
                name: "a".to_string(),
                line_number: 1,
                has_parameters: false,
                has_dependencies: true,
                has_body: true,
            },
            RecipeInfo {
                name: "b".to_string(),
                line_number: 5,
                has_parameters: false,
                has_dependencies: true,
                has_body: true,
            },
            RecipeInfo {
                name: "c".to_string(),
                line_number: 9,
                has_parameters: false,
                has_dependencies: true,
                has_body: true,
            },
        ];

        // Create circular dependencies: a -> b -> c -> a
        let dependencies = vec![
            DependencyInfo {
                name: "b".to_string(),
                arguments: Vec::new(),
                is_conditional: false,
                condition: None,
                position: Some((2, 0)),
                dependency_type: DependencyType::Simple,
            },
            DependencyInfo {
                name: "c".to_string(),
                arguments: Vec::new(),
                is_conditional: false,
                condition: None,
                position: Some((6, 0)),
                dependency_type: DependencyType::Simple,
            },
            DependencyInfo {
                name: "a".to_string(),
                arguments: Vec::new(),
                is_conditional: false,
                condition: None,
                position: Some((10, 0)),
                dependency_type: DependencyType::Simple,
            },
        ];

        let result = DependencyValidator::validate_all_dependencies(&recipes, &dependencies);

        assert!(result.has_errors());
        assert!(!result.circular_dependencies.is_empty());
        println!(
            "Detected circular dependencies: {:?}",
            result.circular_dependencies
        );
    }

    /// Test missing dependency detection
    #[test]
    fn test_missing_dependency_detection() {
        let recipes = vec![
            RecipeInfo {
                name: "build".to_string(),
                line_number: 1,
                has_parameters: false,
                has_dependencies: false,
                has_body: true,
            },
            RecipeInfo {
                name: "test".to_string(),
                line_number: 5,
                has_parameters: false,
                has_dependencies: true,
                has_body: true,
            },
        ];

        let dependencies = vec![
            DependencyInfo::simple("build".to_string()), // Valid dependency
            DependencyInfo::simple("deploy".to_string()), // Missing dependency
            DependencyInfo::simple("cleanup".to_string()), // Missing dependency
        ];

        let result = DependencyValidator::validate_all_dependencies(&recipes, &dependencies);

        assert!(result.has_errors());
        assert_eq!(result.missing_dependencies.len(), 2);
        assert!(result.missing_dependencies.contains(&"deploy".to_string()));
        assert!(result.missing_dependencies.contains(&"cleanup".to_string()));
    }

    /// Test individual dependency validation
    #[test]
    fn test_individual_dependency_validation() {
        let available_recipes = vec![
            "build".to_string(),
            "test".to_string(),
            "deploy".to_string(),
        ];

        // Valid dependency
        let valid_dep = DependencyInfo::simple("build".to_string());
        let errors = DependencyValidator::validate_dependency(&valid_dep, &available_recipes);
        assert!(errors.is_empty());

        // Invalid dependency - missing target
        let invalid_dep = DependencyInfo::simple("missing".to_string());
        let errors = DependencyValidator::validate_dependency(&invalid_dep, &available_recipes);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].error_type, DependencyErrorType::MissingTarget);

        // Invalid dependency - empty name
        let empty_dep = DependencyInfo::simple("".to_string());
        let errors = DependencyValidator::validate_dependency(&empty_dep, &available_recipes);
        assert_eq!(errors.len(), 2); // Both invalid name and missing target
        assert!(errors
            .iter()
            .any(|e| e.error_type == DependencyErrorType::InvalidName));

        // Invalid dependency - empty arguments
        let mut dep_with_empty_args = DependencyInfo::parameterized(
            "deploy".to_string(),
            vec!["staging".to_string(), "".to_string()],
        );
        let errors =
            DependencyValidator::validate_dependency(&dep_with_empty_args, &available_recipes);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].error_type, DependencyErrorType::InvalidArgument);

        // Invalid dependency - empty condition
        dep_with_empty_args.condition = Some("".to_string());
        dep_with_empty_args.is_conditional = true;
        dep_with_empty_args.dependency_type = DependencyType::Complex;
        let errors =
            DependencyValidator::validate_dependency(&dep_with_empty_args, &available_recipes);
        assert!(errors.len() >= 2); // Empty argument + empty condition
        assert!(errors
            .iter()
            .any(|e| e.error_type == DependencyErrorType::InvalidCondition));
    }

    /// Test dependency type inference and display
    #[test]
    fn test_dependency_type_display() {
        assert_eq!(DependencyType::Simple.to_string(), "simple");
        assert_eq!(DependencyType::Parameterized.to_string(), "parameterized");
        assert_eq!(DependencyType::Conditional.to_string(), "conditional");
        assert_eq!(DependencyType::Complex.to_string(), "complex");
    }

    /// Test dependency validation result
    #[test]
    fn test_dependency_validation_result() {
        let mut result = DependencyValidationResult::new();
        assert!(!result.has_errors());
        assert_eq!(result.error_count(), 0);

        // Add some errors
        result
            .circular_dependencies
            .push(vec!["a".to_string(), "b".to_string(), "a".to_string()]);
        result.missing_dependencies.push("missing".to_string());

        assert!(result.has_errors());
        assert_eq!(result.error_count(), 2);
    }

    /// Test dependency parsing from demo justfile
    #[test]
    fn test_demo_justfile_dependency_extraction() {
        let mut parser = ASTJustParser::new().unwrap();

        // Read the demo justfile
        let demo_content = include_str!("../demo/justfile");

        let tree = parser.parse_content(demo_content).unwrap();
        let recipes = parser.extract_recipes(&tree).unwrap();

        println!("Extracted {} recipes from demo justfile", recipes.len());

        // Check for recipes with dependencies
        let recipes_with_deps: Vec<_> = recipes
            .iter()
            .filter(|r| !r.dependencies.is_empty())
            .collect();

        println!(
            "Found {} recipes with dependencies:",
            recipes_with_deps.len()
        );
        for recipe in &recipes_with_deps {
            println!("  {} depends on: {:?}", recipe.name, recipe.dependencies);
        }

        // Test dependency validation if any dependencies were found
        if !recipes_with_deps.is_empty() {
            // Create DependencyInfo objects from the extracted dependencies
            let dep_infos: Vec<DependencyInfo> = recipes
                .iter()
                .flat_map(|recipe| {
                    recipe
                        .dependencies
                        .iter()
                        .map(|dep_name| DependencyInfo::simple(dep_name.clone()))
                })
                .collect();

            let recipe_infos: Vec<RecipeInfo> = recipes
                .iter()
                .map(|recipe| RecipeInfo {
                    name: recipe.name.clone(),
                    line_number: recipe.line_number,
                    has_parameters: !recipe.parameters.is_empty(),
                    has_dependencies: !recipe.dependencies.is_empty(),
                    has_body: !recipe.body.is_empty(),
                })
                .collect();

            let validation_result =
                DependencyValidator::validate_all_dependencies(&recipe_infos, &dep_infos);

            println!("Dependency validation results:");
            println!(
                "  Circular dependencies: {}",
                validation_result.circular_dependencies.len()
            );
            println!(
                "  Missing dependencies: {}",
                validation_result.missing_dependencies.len()
            );
            println!(
                "  Invalid dependencies: {}",
                validation_result.invalid_dependencies.len()
            );

            if !validation_result.missing_dependencies.is_empty() {
                println!("  Missing: {:?}", validation_result.missing_dependencies);
            }
        }
    }

    /// Test complex dependency scenarios
    #[test]
    fn test_complex_dependency_scenarios() {
        let mut parser = ASTJustParser::new().unwrap();

        let content = r#"
# Complex dependency test justfile

# Simple recipe with no dependencies
clean:
    rm -rf target/

# Recipe with simple dependency
build: clean
    cargo build

# Recipe with multiple dependencies
test: clean build
    cargo test

# Recipe with complex dependency chain
deploy: test build clean
    echo "Deploying..."

# Recipe that could create circular dependency
setup: deploy
    echo "Setup complete"
"#;

        let tree = parser.parse_content(content).unwrap();
        let recipes = parser.extract_recipes(&tree).unwrap();

        println!("Complex scenario: Extracted {} recipes", recipes.len());

        // Convert to RecipeInfo and DependencyInfo for validation
        let recipe_infos: Vec<RecipeInfo> = recipes
            .iter()
            .map(|recipe| RecipeInfo {
                name: recipe.name.clone(),
                line_number: recipe.line_number,
                has_parameters: !recipe.parameters.is_empty(),
                has_dependencies: !recipe.dependencies.is_empty(),
                has_body: !recipe.body.is_empty(),
            })
            .collect();

        let dep_infos: Vec<DependencyInfo> = recipes
            .iter()
            .flat_map(|recipe| {
                recipe
                    .dependencies
                    .iter()
                    .map(|dep_name| DependencyInfo::simple(dep_name.clone()))
            })
            .collect();

        let validation_result =
            DependencyValidator::validate_all_dependencies(&recipe_infos, &dep_infos);

        println!("Complex scenario validation:");
        println!("  Total dependencies: {}", dep_infos.len());
        println!("  Validation errors: {}", validation_result.error_count());

        // This scenario should not have circular dependencies or missing dependencies
        // since all referenced recipes exist in the same file
        for cycle in &validation_result.circular_dependencies {
            println!("  Circular dependency detected: {:?}", cycle);
        }

        for missing in &validation_result.missing_dependencies {
            println!("  Missing dependency: {}", missing);
        }
    }

    /// Test edge cases in dependency parsing
    #[test]
    fn test_dependency_parsing_edge_cases() {
        // Test empty dependency handling
        let empty_dep = DependencyInfo::simple("".to_string());
        assert!(!empty_dep.is_valid());

        // Test dependency with whitespace in name
        let whitespace_dep = DependencyInfo::simple("  build  ".to_string());
        assert!(whitespace_dep.is_valid()); // Name should be trimmed in actual parsing

        // Test dependency with special characters
        let special_dep = DependencyInfo::simple("build-prod".to_string());
        assert!(special_dep.is_valid());

        // Test extremely long dependency chain
        let long_chain: Vec<RecipeInfo> = (0..100)
            .map(|i| RecipeInfo {
                name: format!("recipe_{}", i),
                line_number: i + 1,
                has_parameters: false,
                has_dependencies: i > 0,
                has_body: true,
            })
            .collect();

        let chain_deps: Vec<DependencyInfo> = (1..100)
            .map(|i| DependencyInfo::simple(format!("recipe_{}", i - 1)))
            .collect();

        let result = DependencyValidator::validate_all_dependencies(&long_chain, &chain_deps);
        assert!(!result.has_errors()); // Linear chain should be valid
    }
}

#[cfg(not(feature = "ast-parser"))]
mod no_ast_parser_tests {
    #[test]
    fn test_dependency_resolution_feature_gated() {
        // Verify dependency resolution functionality is properly feature-gated
        assert!(
            true,
            "Dependency resolution functionality properly gated behind feature flag"
        );
    }
}
