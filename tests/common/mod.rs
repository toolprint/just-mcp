use std::fs;
use std::path::{Path, PathBuf};

/// Base directory for test assets and fixtures
pub const TEST_BASE_DIR: &str = "test-fixtures";

/// Creates a test directory under the base test directory
///
/// # Arguments
/// * `test_name` - Name of the test subdirectory to create
///
/// # Returns
/// * `PathBuf` - Path to the created test directory
///
/// # Panics
/// * If directory creation fails
pub fn create_test_dir(test_name: &str) -> PathBuf {
    let test_dir = Path::new(TEST_BASE_DIR).join(test_name);
    fs::create_dir_all(&test_dir).expect(&format!(
        "Failed to create test directory: {}",
        test_dir.display()
    ));
    test_dir
}

/// Creates a test directory and returns both the directory path and a justfile path within it
///
/// # Arguments
/// * `test_name` - Name of the test subdirectory to create
///
/// # Returns
/// * `(PathBuf, PathBuf)` - Tuple of (test_dir, justfile_path)
pub fn create_test_dir_with_justfile(test_name: &str) -> (PathBuf, PathBuf) {
    let test_dir = create_test_dir(test_name);
    let justfile_path = test_dir.join("justfile");
    (test_dir, justfile_path)
}

/// Cleans up a test directory (removes it and all contents)
///
/// # Arguments
/// * `test_name` - Name of the test subdirectory to remove
pub fn cleanup_test_dir(test_name: &str) {
    let test_dir = Path::new(TEST_BASE_DIR).join(test_name);
    if test_dir.exists() {
        fs::remove_dir_all(&test_dir).expect(&format!(
            "Failed to remove test directory: {}",
            test_dir.display()
        ));
    }
}

/// Gets the path to a test directory without creating it
///
/// # Arguments
/// * `test_name` - Name of the test subdirectory
///
/// # Returns
/// * `PathBuf` - Path to the test directory
#[allow(dead_code)]
pub fn get_test_dir_path(test_name: &str) -> PathBuf {
    Path::new(TEST_BASE_DIR).join(test_name)
}

/// Creates a test justfile with the given content
///
/// # Arguments
/// * `test_name` - Name of the test subdirectory
/// * `content` - Content to write to the justfile
///
/// # Returns
/// * `PathBuf` - Path to the created justfile
#[allow(dead_code)]
pub fn create_test_justfile(test_name: &str, content: &str) -> PathBuf {
    let (_, justfile_path) = create_test_dir_with_justfile(test_name);
    fs::write(&justfile_path, content).expect(&format!(
        "Failed to write justfile: {}",
        justfile_path.display()
    ));
    justfile_path
}
