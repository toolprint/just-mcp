#[test]
fn test_project_structure() {
    // Ensure all modules are accessible
    use just_mcp::{PKG_NAME, VERSION};

    assert_eq!(PKG_NAME, "just-mcp");
    assert!(!VERSION.is_empty());
}

#[test]
fn test_error_types() {
    use just_mcp::Error;

    let io_error = Error::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "test"));
    assert!(matches!(io_error, Error::Io(_)));

    let parse_error = Error::Parse {
        message: "test parse error".to_string(),
        line: 1,
        column: 0,
    };
    assert!(matches!(parse_error, Error::Parse { .. }));
}
