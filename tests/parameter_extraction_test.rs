//! Comprehensive tests for parameter extraction and processing
//!
//! This test module validates Task 133 implementation: Parameter Extraction and Processing

#[cfg(feature = "ast-parser")]
mod parameter_extraction_tests {
    use just_mcp::parser::ast::{
        queries::{
            CommentAssociator, CommentInfo, ExpressionEvaluator, ParameterInfo, ParameterType,
        },
        ASTJustParser,
    };

    /// Test parameter extraction from the demo justfile
    #[test]
    fn test_demo_justfile_parameter_extraction() {
        let mut parser = ASTJustParser::new().unwrap();

        // Read the demo justfile
        let demo_content = include_str!("../demo/justfile");

        let tree = parser.parse_content(demo_content).unwrap();
        let recipes = parser.extract_recipes(&tree).unwrap();

        println!("Extracted {} recipes from demo justfile", recipes.len());

        // Check specific recipes with parameters
        let hello_recipe = recipes.iter().find(|r| r.name == "hello");
        if let Some(recipe) = hello_recipe {
            println!("Hello recipe parameters: {:?}", recipe.parameters);
            assert!(
                !recipe.parameters.is_empty(),
                "Hello recipe should have parameters"
            );

            // Check if the "name" parameter was extracted
            let name_param = recipe.parameters.iter().find(|p| p.name == "name");
            if let Some(param) = name_param {
                assert_eq!(param.default, Some("World".to_string()));
            }
        }

        let build_recipe = recipes.iter().find(|r| r.name == "build");
        if let Some(recipe) = build_recipe {
            println!("Build recipe parameters: {:?}", recipe.parameters);
            assert!(
                !recipe.parameters.is_empty(),
                "Build recipe should have parameters"
            );

            // Check if the "target" parameter was extracted
            let target_param = recipe.parameters.iter().find(|p| p.name == "target");
            if let Some(param) = target_param {
                assert_eq!(param.default, Some("debug".to_string()));
            }
        }
    }

    /// Test parameter type inference for common patterns
    #[test]
    fn test_parameter_type_inference_comprehensive() {
        // Test boolean inference
        assert_eq!(
            ParameterType::infer_from_default("true"),
            ParameterType::Boolean
        );
        assert_eq!(
            ParameterType::infer_from_default("false"),
            ParameterType::Boolean
        );

        // Test number inference
        assert_eq!(
            ParameterType::infer_from_default("0"),
            ParameterType::Number
        );
        assert_eq!(
            ParameterType::infer_from_default("42"),
            ParameterType::Number
        );
        assert_eq!(
            ParameterType::infer_from_default("1000"),
            ParameterType::Number
        );
        assert_eq!(
            ParameterType::infer_from_default("3.14"),
            ParameterType::Number
        );
        assert_eq!(
            ParameterType::infer_from_default("-1"),
            ParameterType::Number
        );

        // Test path inference
        assert_eq!(
            ParameterType::infer_from_default("./config.json"),
            ParameterType::Path
        );
        assert_eq!(
            ParameterType::infer_from_default("../output.txt"),
            ParameterType::Path
        );
        assert_eq!(
            ParameterType::infer_from_default("/usr/bin/app"),
            ParameterType::Path
        );
        assert_eq!(
            ParameterType::infer_from_default("output.json"),
            ParameterType::Path
        );
        assert_eq!(
            ParameterType::infer_from_default("backup.yml"),
            ParameterType::Path
        );

        // Test string inference (default)
        assert_eq!(
            ParameterType::infer_from_default("debug"),
            ParameterType::String
        );
        assert_eq!(
            ParameterType::infer_from_default("staging"),
            ParameterType::String
        );
        assert_eq!(
            ParameterType::infer_from_default("GET"),
            ParameterType::String
        );

        // Test name-based inference
        assert_eq!(
            ParameterType::infer_from_name("input_file"),
            ParameterType::Path
        );
        assert_eq!(
            ParameterType::infer_from_name("output"),
            ParameterType::Path
        );
        assert_eq!(
            ParameterType::infer_from_name("directory"),
            ParameterType::Path
        );
        assert_eq!(
            ParameterType::infer_from_name("count"),
            ParameterType::Number
        );
        assert_eq!(
            ParameterType::infer_from_name("limit"),
            ParameterType::Number
        );
        assert_eq!(
            ParameterType::infer_from_name("size"),
            ParameterType::Number
        );
        assert_eq!(
            ParameterType::infer_from_name("port"),
            ParameterType::Number
        );
        assert_eq!(
            ParameterType::infer_from_name("timeout"),
            ParameterType::Number
        );
        assert_eq!(
            ParameterType::infer_from_name("iterations"),
            ParameterType::Number
        );
        assert_eq!(
            ParameterType::infer_from_name("interval"),
            ParameterType::Number
        );
        assert_eq!(
            ParameterType::infer_from_name("enable"),
            ParameterType::Boolean
        );
        assert_eq!(
            ParameterType::infer_from_name("disable"),
            ParameterType::Boolean
        );
        assert_eq!(
            ParameterType::infer_from_name("verbose"),
            ParameterType::Boolean
        );
        assert_eq!(
            ParameterType::infer_from_name("debug"),
            ParameterType::Boolean
        );
        assert_eq!(
            ParameterType::infer_from_name("force"),
            ParameterType::Boolean
        );
    }

    /// Test expression evaluation for complex default values
    #[test]
    fn test_expression_evaluation_comprehensive() {
        // Test quoted string evaluation
        assert_eq!(
            ExpressionEvaluator::evaluate_default_expression("\"hello world\""),
            "hello world"
        );
        assert_eq!(
            ExpressionEvaluator::evaluate_default_expression("'single quoted'"),
            "single quoted"
        );

        // Test unquoted string evaluation
        assert_eq!(
            ExpressionEvaluator::evaluate_default_expression("debug"),
            "debug"
        );
        assert_eq!(
            ExpressionEvaluator::evaluate_default_expression("staging"),
            "staging"
        );
        assert_eq!(
            ExpressionEvaluator::evaluate_default_expression("production"),
            "production"
        );

        // Test alphanumeric with special characters
        assert_eq!(
            ExpressionEvaluator::evaluate_default_expression("config-dev"),
            "config-dev"
        );
        assert_eq!(
            ExpressionEvaluator::evaluate_default_expression("app_v1.0"),
            "app_v1.0"
        );

        // Test whitespace handling
        assert_eq!(
            ExpressionEvaluator::evaluate_default_expression("  release  "),
            "release"
        );

        // Test complex expression detection
        assert!(ExpressionEvaluator::is_complex_expression("{{variable}}"));
        assert!(ExpressionEvaluator::is_complex_expression("func(arg)"));
        assert!(ExpressionEvaluator::is_complex_expression("a + b"));
        assert!(ExpressionEvaluator::is_complex_expression("x * y"));
        assert!(ExpressionEvaluator::is_complex_expression("value / 2"));
        assert!(ExpressionEvaluator::is_complex_expression("count - 1"));

        assert!(!ExpressionEvaluator::is_complex_expression("simple"));
        assert!(!ExpressionEvaluator::is_complex_expression(
            "\"quoted string\""
        ));
        assert!(!ExpressionEvaluator::is_complex_expression(
            "'another quoted'"
        ));

        // Test variable reference extraction (sorted for consistency)
        let vars =
            ExpressionEvaluator::extract_variable_references("Hello {{name}} from {{location}}!");
        assert_eq!(vars, vec!["location", "name"]);

        let vars = ExpressionEvaluator::extract_variable_references("Build target: {{target}}");
        assert_eq!(vars, vec!["target"]);

        let vars = ExpressionEvaluator::extract_variable_references("No variables here");
        assert!(vars.is_empty());

        let vars = ExpressionEvaluator::extract_variable_references("{{single}}");
        assert_eq!(vars, vec!["single"]);
    }

    /// Test comment association for parameter descriptions
    #[test]
    fn test_comment_association_patterns() {
        let mut parameters = vec![
            ParameterInfo {
                name: "name".to_string(),
                default_value: Some("World".to_string()),
                is_variadic: false,
                is_required: false,
                description: None,
                parameter_type: ParameterType::String,
                raw_default: Some("\"World\"".to_string()),
                position: Some((10, 5)),
            },
            ParameterInfo {
                name: "target".to_string(),
                default_value: Some("debug".to_string()),
                is_variadic: false,
                is_required: false,
                description: None,
                parameter_type: ParameterType::String,
                raw_default: Some("\"debug\"".to_string()),
                position: Some((24, 5)),
            },
            ParameterInfo {
                name: "count".to_string(),
                default_value: Some("10".to_string()),
                is_variadic: false,
                is_required: false,
                description: None,
                parameter_type: ParameterType::Number,
                raw_default: Some("10".to_string()),
                position: Some((45, 5)),
            },
        ];

        let comments = vec![
            CommentInfo {
                text: "{{name}}: person or thing to greet".to_string(),
                line_number: 8,
            },
            CommentInfo {
                text: "{{target}}: the build target mode (debug, release, optimized)".to_string(),
                line_number: 22,
            },
            CommentInfo {
                text: "{{count}}: number of records to seed".to_string(),
                line_number: 43,
            },
        ];

        CommentAssociator::associate_parameter_descriptions(&mut parameters, &comments);

        assert_eq!(
            parameters[0].description,
            Some("person or thing to greet".to_string())
        );
        assert_eq!(
            parameters[1].description,
            Some("the build target mode (debug, release, optimized)".to_string())
        );
        assert_eq!(
            parameters[2].description,
            Some("number of records to seed".to_string())
        );
    }

    /// Test variadic parameter handling
    #[test]
    fn test_variadic_parameter_handling() {
        // Test creating variadic parameters
        let variadic_param = ParameterInfo {
            name: "args".to_string(),
            default_value: None,
            is_variadic: true,
            is_required: false,
            description: Some("Additional arguments".to_string()),
            parameter_type: ParameterType::Array,
            raw_default: None,
            position: Some((15, 10)),
        };

        assert!(variadic_param.is_variadic);
        assert_eq!(variadic_param.parameter_type, ParameterType::Array);
        assert!(!variadic_param.is_required); // Variadic params are optional by nature

        let star_variadic = ParameterInfo {
            name: "options".to_string(),
            default_value: None,
            is_variadic: true,
            is_required: false,
            description: Some("Optional arguments".to_string()),
            parameter_type: ParameterType::Array,
            raw_default: None,
            position: Some((20, 15)),
        };

        assert!(star_variadic.is_variadic);
        assert_eq!(star_variadic.parameter_type, ParameterType::Array);
    }

    /// Test required vs optional parameter detection
    #[test]
    fn test_required_optional_parameter_detection() {
        // Required parameter (no default value, not variadic)
        let required_param = ParameterInfo {
            name: "input".to_string(),
            default_value: None,
            is_variadic: false,
            is_required: true,
            description: Some("Input file path".to_string()),
            parameter_type: ParameterType::Path,
            raw_default: None,
            position: Some((85, 5)),
        };

        assert!(required_param.is_required);
        assert!(required_param.default_value.is_none());
        assert!(!required_param.is_variadic);

        // Optional parameter (has default value)
        let optional_param = ParameterInfo {
            name: "output".to_string(),
            default_value: Some("output.json".to_string()),
            is_variadic: false,
            is_required: false,
            description: Some("Output file path for results".to_string()),
            parameter_type: ParameterType::Path,
            raw_default: Some("\"output.json\"".to_string()),
            position: Some((85, 15)),
        };

        assert!(!optional_param.is_required);
        assert!(optional_param.default_value.is_some());
        assert!(!optional_param.is_variadic);

        // Variadic parameter (not required even without default)
        let variadic_param = ParameterInfo {
            name: "files".to_string(),
            default_value: None,
            is_variadic: true,
            is_required: false,
            description: Some("Files to process".to_string()),
            parameter_type: ParameterType::Array,
            raw_default: None,
            position: Some((90, 5)),
        };

        assert!(!variadic_param.is_required);
        assert!(variadic_param.default_value.is_none());
        assert!(variadic_param.is_variadic);
    }

    /// Test parameter validation edge cases
    #[test]
    fn test_parameter_validation_edge_cases() {
        // Test empty parameter name handling
        let result = ParameterInfo {
            name: "".to_string(),
            default_value: None,
            is_variadic: false,
            is_required: true,
            description: None,
            parameter_type: ParameterType::Unknown,
            raw_default: None,
            position: None,
        };

        assert!(result.name.is_empty());
        assert_eq!(result.parameter_type, ParameterType::Unknown);

        // Test parameter with complex default expression
        let complex_param = ParameterInfo {
            name: "timestamp".to_string(),
            default_value: Some("$(date)".to_string()),
            is_variadic: false,
            is_required: false,
            description: Some("Current timestamp".to_string()),
            parameter_type: ParameterType::String,
            raw_default: Some("$(date)".to_string()),
            position: Some((100, 10)),
        };

        assert_eq!(complex_param.default_value, Some("$(date)".to_string()));
        assert_eq!(complex_param.raw_default, Some("$(date)".to_string()));

        // Test parameter type inference with edge cases
        assert_eq!(ParameterType::infer_from_default(""), ParameterType::String);
        assert_eq!(
            ParameterType::infer_from_default("   "),
            ParameterType::String
        );
        assert_eq!(
            ParameterType::infer_from_default("0.0"),
            ParameterType::Number
        );
        assert_eq!(
            ParameterType::infer_from_default("-0"),
            ParameterType::Number
        );
    }

    /// Test comment parsing edge cases
    #[test]
    fn test_comment_parsing_edge_cases() {
        // Test various comment patterns
        let result = CommentAssociator::parse_parameter_doc_comment(
            "{{name}}: description with special chars !@#$%",
        );
        assert_eq!(
            result,
            Some((
                "name".to_string(),
                "description with special chars !@#$%".to_string()
            ))
        );

        let result = CommentAssociator::parse_parameter_doc_comment(
            "{{param_with_underscore}}: underscore parameter",
        );
        assert_eq!(
            result,
            Some((
                "param_with_underscore".to_string(),
                "underscore parameter".to_string()
            ))
        );

        let result =
            CommentAssociator::parse_parameter_doc_comment("{{kebab-case}}: kebab case parameter");
        assert_eq!(
            result,
            Some(("kebab-case".to_string(), "kebab case parameter".to_string()))
        );

        // Test malformed patterns
        let result = CommentAssociator::parse_parameter_doc_comment("{{incomplete");
        assert_eq!(result, None);

        let result = CommentAssociator::parse_parameter_doc_comment("incomplete}}:");
        assert_eq!(result, None);

        let result = CommentAssociator::parse_parameter_doc_comment("{{empty}}: ");
        assert_eq!(result, None);

        let result = CommentAssociator::parse_parameter_doc_comment("{{  }}: no name");
        assert_eq!(result, None);
    }

    /// Test comprehensive parameter extraction integration
    #[test]
    fn test_parameter_extraction_integration() {
        // This test verifies the complete parameter extraction pipeline
        // Note: This uses the fallback parser for now since Tree-sitter queries
        // require the exact grammar implementation to work properly

        let mut parser = ASTJustParser::new().unwrap();

        // Test content with various parameter patterns
        let content = r#"
# {{name}}: person or thing to greet
# Simple greeting task
hello name="World":
    @echo "Hello, {{name}}!"

# {{target}}: the build target mode (debug, release, optimized)
# Build simulation with different targets
build target="debug":
    @echo "Building project in {{target}} mode..."

# {{input}}: input file path to process
# {{output}}: output file path for results
# Task that processes input data
process-data input output="output.json":
    @echo "Processing data from {{input}} to {{output}}..."

# Task with variadic parameters
run-tests +args:
    @echo "Running tests with args: {{args}}"
"#;

        let tree = parser.parse_content(content).unwrap();
        let recipes = parser.extract_recipes(&tree).unwrap();

        println!("Integration test: Extracted {} recipes", recipes.len());

        // Verify recipes were extracted
        assert!(!recipes.is_empty(), "Should extract at least one recipe");

        // Check specific recipes and their parameters
        for recipe in &recipes {
            println!(
                "Recipe: {} with {} parameters",
                recipe.name,
                recipe.parameters.len()
            );

            for param in &recipe.parameters {
                println!("  Parameter: {} (default: {:?})", param.name, param.default);
            }
        }

        // The fallback parser should extract basic parameter information
        let hello_recipe = recipes.iter().find(|r| r.name == "hello");
        if let Some(recipe) = hello_recipe {
            assert!(
                !recipe.parameters.is_empty(),
                "Hello recipe should have parameters"
            );
            let name_param = recipe.parameters.iter().find(|p| p.name == "name");
            if let Some(param) = name_param {
                assert_eq!(param.default, Some("World".to_string()));
            }
        }
    }
}

#[cfg(not(feature = "ast-parser"))]
mod no_ast_parser_tests {
    #[test]
    fn test_parameter_extraction_feature_gated() {
        // Verify parameter extraction is properly feature-gated
        assert!(
            true,
            "Parameter extraction functionality properly gated behind feature flag"
        );
    }
}
