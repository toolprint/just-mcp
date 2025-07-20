# GitHub Actions Workflows

This directory contains GitHub Actions workflows that use Dagger for CI/CD automation.

## Workflows

### Dagger CI (`dagger-ci.yml`)

Runs on every push and pull request using Dagger for all CI operations.

- **Single Command**: Runs `dagger call ci` which executes:
  - Format checking with `cargo fmt`
  - Linting with `cargo clippy`
  - Tests on multiple platforms
  - Code coverage generation
- **Artifacts**: Coverage report uploaded as workflow artifact

### Dagger Release (`dagger-release.yml`)

Triggers on version tags (e.g., `v1.0.0`) to create GitHub releases with pre-built binaries.

- **Linux Builds**: Uses Dagger on Ubuntu for Linux x86_64 and ARM64
- **Platform-Specific Builds**: Uses native runners for macOS and Windows
- **Artifacts**: Compressed binaries (.tar.gz for Unix, .zip for Windows)
- **Automatic Release**: Creates and publishes GitHub release with all artifacts

## Benefits of Dagger-based CI/CD

1. **Local Testing**: Run the exact same CI pipeline locally with `just dagger-ci`
2. **Reproducible Builds**: Containerized builds ensure consistency
3. **Better Caching**: Dagger automatically caches dependencies and build artifacts
4. **Simpler Workflows**: GitHub Actions files are minimal - just install Dagger and run
5. **Platform Agnostic**: The same Dagger module works with any CI system

## Local Testing

### Running CI Checks Locally

```bash
# Run complete CI pipeline
just dagger-ci

# Or use Dagger directly
dagger call ci --source .
```

### Building Releases Locally

```bash
# Build for a specific platform
dagger call package --source . --platform linux/amd64 --version v1.0.0

# Build all Linux releases
dagger call release --source . --version v1.0.0
```

## Creating a Release

1. Update version in `Cargo.toml`
2. Commit changes
3. Create and push a tag:

   ```bash
   git tag v1.0.0
   git push origin v1.0.0
   ```

4. The release workflow will automatically:
   - Build binaries for all platforms
   - Create a GitHub release
   - Upload all artifacts

## Troubleshooting

### Dagger Installation

If Dagger is not installed in CI:

```bash
curl -L https://dl.dagger.io/dagger/install.sh | sh
```

### Platform Limitations

- macOS builds require a macOS runner
- Windows builds require a Windows runner
- Linux ARM64 can be cross-compiled from x86_64

## Dependabot

Dependabot is configured to:

- Check for Rust dependency updates weekly
- Check for GitHub Actions updates weekly
- Group minor and patch updates together
- Create PRs with appropriate labels
