# test-utilities.just - Test suite for common utilities
# Import the common utilities
import 'just/common.just'

# Test all common utility functions
test-common-all: test-messaging test-validation test-commands test-binaries test-file-ops test-run
    @just _success "All common utility tests passed!"

# Test messaging functions
test-messaging:
    @echo "Testing messaging functions..."
    @just _info "This is an info message"
    @just _warn "This is a warning message"
    @just _success "This is a success message"
    @echo "✅ Messaging functions work correctly"

# Test validation functions
test-validation:
    @echo "Testing validation functions..."
    @just _validate "debug" "debug release"
    @echo "✅ Valid option passed validation"
    @echo "✅ Validation functions work correctly"

# Test command requirement checking
test-commands:
    @echo "Testing command requirement functions..."
    @just _require-command "echo"
    @just _require-command "bash"
    @echo "✅ Command requirement functions work correctly"

# Test binary parsing
test-binaries:
    @echo "Testing binary parsing functions..."
    @echo "Detected binaries: $(just _get-binaries)"
    @echo "✅ Binary parsing functions work correctly"

# Test file operations
test-file-ops:
    @echo "Testing file operation utilities..."
    @echo "Cargo bin directory: $(just _get-cargo-bin-dir)"
    @echo "Current timestamp: $(just _timestamp)"
    @echo "✅ File operation utilities work correctly"

# Test run function
test-run:
    @echo "Testing _run function..."
    @just _run "Testing echo command" "echo 'Hello from _run function'"
    @echo "✅ Run function works correctly"

# Test error handling (will exit with error)
test-error-example:
    @echo "Testing error handling (this will exit with error)..."
    @just _error "test" "This is a test error message"

# Test validation failure (will exit with error)
test-validation-failure:
    @echo "Testing validation failure (this will exit with error)..."
    @just _validate "invalid" "valid options only"