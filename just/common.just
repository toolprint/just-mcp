# common.just - Shared utilities and helpers
#
# This module provides reusable functions to eliminate duplication and ensure consistency
# across all justfile modules. All functions are prefixed with underscore (_) to indicate
# they are internal utilities.
#
# 🚨 ERROR HANDLING:
#   • _error <context> <message>     - Display error and exit with code 1
#   • _success <message>             - Display success message
#   • _info <message>                - Display info message
#   • _warn <message>                - Display warning message
#
# ✅ VALIDATION:
#   • _validate <choice> <options>   - Validate parameter against allowed values
#   • _require-command <command>     - Check if command exists, exit if not
#   • _require-file <path> <context> - Check if file exists, exit if not
#
# 🔧 EXECUTION:
#   • _run <description> <command>   - Execute command with status messages
#   • _get-binaries                  - Parse Cargo.toml to get binary names
#   • _check-binary <binary>         - Verify binary exists in release target
#   • _get-cargo-bin-dir             - Get cargo bin directory path
#
# 🛠️  UTILITIES:
#   • _validate-env <env>            - Validate environment name
#   • _timestamp                     - Generate timestamp for backups/releases
#
# USAGE PATTERN:
#   All common utilities should be used consistently across modules to ensure
#   uniform error handling, validation, and user experience.
#
# EXAMPLES:
#   just _validate "debug" "debug release"  # Validate build mode
#   just _require-command "docker"          # Check Docker availability
#   just _run "Building project" "cargo build"  # Execute with status
#
# DESIGN PRINCIPLES:
#   • Consistent error messages and exit codes
#   • Clear success/info/warning indicators with emojis
#   • Proper validation before expensive operations
#   • Helpful context in all error messages

# Error handling with context
_error context message:
    #!/usr/bin/env bash
    echo "❌ {{context}}: {{message}}" >&2
    exit 1

# Success messages
_success message:
    @echo "✅ {{message}}"

# Info messages  
_info message:
    @echo "ℹ️  {{message}}"

# Warning messages
_warn message:
    @echo "⚠️  {{message}}"

# Parameter validation
_validate choice valid_options:
    #!/usr/bin/env bash
    if [[ ! " {{valid_options}} " =~ " {{choice}} " ]]; then
        echo "❌ Invalid option: '{{choice}}'"
        echo "   Valid options: {{valid_options}}"
        exit 1
    fi

# Check if command exists
_require-command command:
    #!/usr/bin/env bash
    if ! command -v {{command}} &> /dev/null; then
        echo "❌ {{command}} is required but not installed"
        exit 1
    fi

# Consistent command execution with status
_run description command:
    @echo "🔄 {{description}}..."
    @{{command}}
    @echo "✅ {{description}} complete"

# Binary parsing utility (eliminates duplication from 3+ recipes)
_get-binaries:
    #!/usr/bin/env bash
    # Find Cargo.toml in current or parent directory
    CARGO_TOML="Cargo.toml"
    if [ ! -f "$CARGO_TOML" ] && [ -f "../Cargo.toml" ]; then
        CARGO_TOML="../Cargo.toml"
    fi
    
    if [ ! -f "$CARGO_TOML" ]; then
        echo "❌ Cargo.toml not found" >&2
        return 1
    fi
    
    if command -v tq >/dev/null 2>&1 && command -v jq >/dev/null 2>&1; then
        tq -o json -f "$CARGO_TOML" 'bin' 2>/dev/null | jq -r '.[].name' 2>/dev/null | tr '\n' ' '
    elif command -v tq >/dev/null 2>&1; then
        bin_json=$(tq -o json -f "$CARGO_TOML" 'bin' 2>/dev/null)
        echo "$bin_json" | sed 's/.*"name":"\([^"]*\)".*/\1/g' | tr '\n' ' '
    else
        awk '/^\[\[bin\]\]/ { in_bin=1; next } /^\[/ { in_bin=0 } in_bin && /^name = / { gsub(/^name = "|"$/, ""); print }' "$CARGO_TOML" | tr '\n' ' '
    fi

# Check if binary exists in release target
_check-binary binary:
    #!/usr/bin/env bash
    if [ -f "target/release/{{binary}}" ]; then
        echo "✅ Binary {{binary}} found"
        return 0
    else
        echo "❌ Binary {{binary}} not found in target/release/"
        return 1
    fi

# Get cargo bin directory
_get-cargo-bin-dir:
    #!/usr/bin/env bash
    if [ -n "$CARGO_HOME" ]; then
        echo "$CARGO_HOME/bin"
    else
        echo "$HOME/.cargo/bin"
    fi

# Validate environment choices
_validate-env env:
    @just _validate "{{env}}" "dev development staging production"

# Check if file exists with helpful error
_require-file filepath context:
    #!/usr/bin/env bash
    if [ ! -f "{{filepath}}" ]; then
        just _error "{{context}}" "Required file not found: {{filepath}}"
    fi

# Generate timestamp for backups/releases
_timestamp:
    @date +%Y%m%d_%H%M%S