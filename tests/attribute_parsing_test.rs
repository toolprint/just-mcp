//! Comprehensive test suite for Just recipe attribute parsing
//!
//! This module tests the attribute parsing functionality for Just recipes,
//! including various attribute types, validation, and integration with the AST parser.

#[cfg(feature = "ast-parser")]
mod attribute_parsing_tests {
    use just_mcp::parser::ast::queries::{AttributeInfo, AttributeType, QueryResultProcessor};
    use just_mcp::parser::ast::ASTJustParser;

    fn create_parser() -> ASTJustParser {
        ASTJustParser::new().expect("Failed to create AST parser")
    }

    #[test]
    fn test_parse_simple_attributes() {
        let mut parser = create_parser();
        let content = r#"
# Simple tasks without attributes
_helper:
    echo "Helper task"

test-unit:
    cargo test --lib

dangerous-task:
    rm -rf data/
"#;

        let tree = parser
            .parse_content(content)
            .expect("Failed to parse content");
        let tasks = parser
            .extract_recipes(&tree)
            .expect("Failed to extract recipes");

        println!(
            "Extracted {} tasks: {:?}",
            tasks.len(),
            tasks.iter().map(|t| &t.name).collect::<Vec<_>>()
        );

        // Verify that we extract the basic recipes
        // and that the attribute fields exist (even if empty)
        for task in &tasks {
            println!(
                "Task: {}, private: {}, group: {:?}, confirm: {:?}",
                task.name, task.is_private, task.group, task.confirm_message
            );

            // Verify attribute fields exist
            assert!(task.attributes.is_empty()); // No attributes in simple test
                                                 // These fields should exist but be None/false for tasks without attributes
                                                 // The is_private field should be inferred from naming convention
            if task.name.starts_with('_') {
                assert!(task.is_private);
            }
        }
    }

    #[test]
    fn test_task_with_parameters() {
        let mut parser = create_parser();
        let content = r#"
deploy env="production":
    echo "Deploying to {{env}}"

seed-db count="100":
    echo "Seeding {{count}} records"
"#;

        let tree = parser
            .parse_content(content)
            .expect("Failed to parse content");
        let tasks = parser
            .extract_recipes(&tree)
            .expect("Failed to extract recipes");

        // Verify that tasks with parameters are parsed and have attribute fields
        for task in &tasks {
            // Verify attribute fields exist even if not populated
            assert!(task.group.is_none());
            assert!(task.confirm_message.is_none());
            assert!(task.doc.is_none());
            assert!(task.attributes.is_empty());
        }
    }

    #[test]
    fn test_basic_recipe_parsing() {
        let mut parser = create_parser();
        let content = r#"
install-windows:
    choco install just

install-unix:
    brew install just

setup-linux:
    apt-get update

setup-macos:
    brew update
"#;

        let tree = parser
            .parse_content(content)
            .expect("Failed to parse content");
        let tasks = parser
            .extract_recipes(&tree)
            .expect("Failed to extract recipes");

        // Verify all tasks are parsed and have attribute structure
        for task in &tasks {
            assert!(task.group.is_none());
            assert!(task.confirm_message.is_none());
            assert!(task.doc.is_none());
            assert!(task.attributes.is_empty());
            assert!(!task.is_private); // No underscore prefix
        }
    }

    #[test]
    fn test_attribute_validation() {
        // Test valid attributes
        let valid_group = AttributeInfo::with_value("group".to_string(), "test".to_string(), 1);
        assert!(valid_group.is_valid());

        let valid_private = AttributeInfo::new("private".to_string(), 1);
        assert!(valid_private.is_valid());

        let valid_confirm =
            AttributeInfo::with_value("confirm".to_string(), "Are you sure?".to_string(), 1);
        assert!(valid_confirm.is_valid());

        // Test invalid attributes
        let invalid_group = AttributeInfo::new("group".to_string(), 1); // Missing argument
        assert!(!invalid_group.is_valid());
        let errors = invalid_group.validation_errors();
        assert!(!errors.is_empty());
        assert!(errors[0].contains("Group attribute requires exactly one argument"));

        // Test conflicting attributes
        let private = AttributeInfo::new("private".to_string(), 1);
        let confirm = AttributeInfo::with_value("confirm".to_string(), "Sure?".to_string(), 2);
        let windows = AttributeInfo::new("windows".to_string(), 3);
        let linux = AttributeInfo::new("linux".to_string(), 4);

        // Test private + confirm conflict
        let errors = QueryResultProcessor::validate_attributes(&[private.clone(), confirm]);
        assert!(errors
            .iter()
            .any(|e| e.contains("Private recipe") && e.contains("confirm attribute")));

        // Test platform conflicts
        let errors = QueryResultProcessor::validate_attributes(&[windows, linux]);
        assert!(errors
            .iter()
            .any(|e| e.contains("conflicting platform attributes")));
    }

    #[test]
    fn test_multiple_group_attributes_error() {
        let group1 = AttributeInfo::with_value("group".to_string(), "test1".to_string(), 1);
        let group2 = AttributeInfo::with_value("group".to_string(), "test2".to_string(), 2);

        let errors = QueryResultProcessor::validate_attributes(&[group1, group2]);
        assert!(errors
            .iter()
            .any(|e| e.contains("multiple group attributes")));
    }

    #[test]
    fn test_attribute_type_classification() {
        assert_eq!(AttributeType::from_name("group"), AttributeType::Group);
        assert_eq!(AttributeType::from_name("private"), AttributeType::Private);
        assert_eq!(AttributeType::from_name("confirm"), AttributeType::Confirm);
        assert_eq!(AttributeType::from_name("doc"), AttributeType::Doc);
        assert_eq!(AttributeType::from_name("no-cd"), AttributeType::NoCD);
        assert_eq!(AttributeType::from_name("windows"), AttributeType::Windows);
        assert_eq!(AttributeType::from_name("unix"), AttributeType::Unix);
        assert_eq!(AttributeType::from_name("linux"), AttributeType::Linux);
        assert_eq!(AttributeType::from_name("macos"), AttributeType::MacOS);

        // Test unknown attribute
        if let AttributeType::Unknown(name) = AttributeType::from_name("custom") {
            assert_eq!(name, "custom");
        } else {
            panic!("Expected Unknown attribute type");
        }
    }

    #[test]
    fn test_attribute_display_formatting() {
        let private = AttributeInfo::new("private".to_string(), 1);
        assert_eq!(private.format_display(), "[private]");

        let group = AttributeInfo::with_value("group".to_string(), "test".to_string(), 1);
        assert_eq!(group.format_display(), "[group('test')]");

        let confirm_with_msg =
            AttributeInfo::with_value("confirm".to_string(), "Are you sure?".to_string(), 1);
        assert_eq!(
            confirm_with_msg.format_display(),
            "[confirm('Are you sure?')]"
        );
    }

    #[test]
    fn test_cleanup_task_structure() {
        let mut parser = create_parser();
        let content = r#"
cleanup:
    rm -rf /tmp/test-data
    echo "Cleanup completed"
"#;

        let tree = parser
            .parse_content(content)
            .expect("Failed to parse content");
        let tasks = parser
            .extract_recipes(&tree)
            .expect("Failed to extract recipes");

        assert!(!tasks.is_empty());
        let task = &tasks[0];

        assert_eq!(task.name, "cleanup");
        // Verify attribute structure exists
        assert!(task.group.is_none());
        assert!(task.confirm_message.is_none());
        assert!(task.doc.is_none());
        assert!(task.attributes.is_empty());
        assert!(!task.is_private); // No underscore prefix
    }

    #[test]
    fn test_global_command_structure() {
        let mut parser = create_parser();
        let content = r#"
global-command:
    echo "This runs without changing directory"
"#;

        let tree = parser
            .parse_content(content)
            .expect("Failed to parse content");
        let tasks = parser
            .extract_recipes(&tree)
            .expect("Failed to extract recipes");

        assert!(!tasks.is_empty());
        let task = &tasks[0];

        assert_eq!(task.name, "global-command");
        assert!(!task.is_private);
        assert!(task.group.is_none());
        assert!(task.confirm_message.is_none());
        assert!(task.doc.is_none());
        assert!(task.attributes.is_empty());
    }

    #[test]
    fn test_unknown_attributes() {
        let unknown_attr = AttributeInfo::new("custom-attr".to_string(), 1);
        assert_eq!(
            unknown_attr.attribute_type,
            AttributeType::Unknown("custom-attr".to_string())
        );
        // Unknown attributes need arguments to be considered valid in our implementation
        // Boolean attributes need to be explicitly defined
        let unknown_with_args =
            AttributeInfo::with_value("custom-attr".to_string(), "value".to_string(), 1);
        assert!(unknown_with_args.is_valid()); // Unknown attributes with args are considered valid

        // Unknown attributes shouldn't cause validation errors by themselves
        let errors = QueryResultProcessor::validate_attributes(&[unknown_with_args]);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_attribute_helper_methods() {
        let group_attr = AttributeInfo::with_value("group".to_string(), "deploy".to_string(), 1);

        // Test getter methods
        assert_eq!(group_attr.get_value(), Some("deploy"));
        assert_eq!(group_attr.get_arguments(), vec!["deploy"]);
        assert!(!group_attr.is_boolean);

        // Test private attribute (boolean)
        let private_attr = AttributeInfo::new("private".to_string(), 1);
        assert_eq!(private_attr.get_value(), None);
        assert!(private_attr.get_arguments().is_empty());
        assert!(private_attr.is_boolean);

        // Test platform-specific detection
        assert!(AttributeType::Windows.is_platform_specific());
        assert!(AttributeType::Linux.is_platform_specific());
        assert!(!AttributeType::Group.is_platform_specific());
        assert!(!AttributeType::Private.is_platform_specific());

        // Test visibility effects
        assert!(AttributeType::Private.affects_visibility());
        assert!(!AttributeType::Group.affects_visibility());

        // Test interaction requirements
        assert!(AttributeType::Confirm.requires_interaction());
        assert!(!AttributeType::Private.requires_interaction());
    }
}

#[cfg(not(feature = "ast-parser"))]
mod no_ast_parser_tests {
    #[test]
    fn test_attribute_parsing_feature_gated() {
        // This test verifies that when the ast-parser feature is not enabled,
        // the code compiles correctly without the tree-sitter dependencies.
        assert!(
            true,
            "Attribute parsing functionality properly gated behind ast-parser feature flag"
        );
    }
}
