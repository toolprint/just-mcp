// CI/CD pipeline for just-mcp Rust project
//
// This module provides a complete CI/CD pipeline for the just-mcp project,
// including formatting checks, linting, testing, coverage, and release builds
// for multiple platforms.

package main

import (
	"context"
	"dagger/just-mcp/internal/dagger"
	"fmt"
)

type JustMcp struct{}

// rustContainer creates a base Rust container with common tools
func (m *JustMcp) rustContainer(source *dagger.Directory) *dagger.Container {
	return dag.Container().
		From("rust:1.88.0").
		WithDirectory("/src", source).
		WithWorkdir("/src").
		WithExec([]string{"rustup", "component", "add", "rustfmt", "clippy"}).
		// Install just for tests
		WithExec([]string{"sh", "-c", "curl -qsSf https://just.systems/install.sh | bash -s -- --to /usr/local/bin"})
}

// Format checks Rust code formatting
func (m *JustMcp) Format(ctx context.Context, source *dagger.Directory) (string, error) {
	return m.rustContainer(source).
		WithExec([]string{"cargo", "fmt", "--", "--check"}).
		Stdout(ctx)
}

// Lint runs clippy on the Rust code
func (m *JustMcp) Lint(ctx context.Context, source *dagger.Directory) (string, error) {
	return m.rustContainer(source).
		WithExec([]string{"cargo", "clippy", "--", "-D", "warnings"}).
		Stdout(ctx)
}

// Test runs all tests for a specific platform
func (m *JustMcp) Test(
	ctx context.Context,
	source *dagger.Directory,
	// +optional
	// +default="linux/amd64"
	platform string,
) (string, error) {
	container := dag.Container(dagger.ContainerOpts{Platform: dagger.Platform(platform)}).
		From("rust:1.88.0").
		WithDirectory("/src", source).
		WithWorkdir("/src").
		WithExec([]string{"rustup", "component", "add", "rustfmt", "clippy"}).
		// Install just for tests
		WithExec([]string{"sh", "-c", "curl -qsSf https://just.systems/install.sh | bash -s -- --to /usr/local/bin"})
	
	return container.
		WithExec([]string{"cargo", "test", "--verbose"}).
		Stdout(ctx)
}

// Coverage generates code coverage report using tarpaulin
func (m *JustMcp) Coverage(ctx context.Context, source *dagger.Directory) (*dagger.File, error) {
	container := dag.Container().
		From("rust:1.88.0").
		WithDirectory("/src", source).
		WithWorkdir("/src").
		WithExec([]string{"rustup", "component", "add", "rustfmt", "clippy"}).
		// Install just for tests
		WithExec([]string{"sh", "-c", "curl -qsSf https://just.systems/install.sh | bash -s -- --to /usr/local/bin"}).
		// Install tarpaulin
		WithExec([]string{"cargo", "install", "cargo-tarpaulin"})
	
	return container.
		// Generate coverage
		WithExec([]string{"cargo", "tarpaulin", "--out", "Html", "--output-dir", "/coverage"}).
		File("/coverage/tarpaulin-report.html"), nil
}

// Build creates a debug build
func (m *JustMcp) Build(
	ctx context.Context,
	source *dagger.Directory,
	// +optional
	// +default="linux/amd64"
	platform string,
) (*dagger.File, error) {
	target := platformToTarget(platform)
	
	// Always use linux/amd64 container for cross-compilation
	container := dag.Container().
		From("rust:1.88.0").
		WithDirectory("/src", source).
		WithWorkdir("/src")

	// For native x86_64 Linux, don't specify target to avoid issues
	if platform == "linux/amd64" {
		return container.
			WithExec([]string{"cargo", "build"}).
			File("/src/target/debug/just-mcp"), nil
	}
	
	// Setup cross-compilation for other targets
	container = setupCrossCompilation(container, target)

	return container.
		WithExec([]string{"cargo", "build", "--target", target}).
		File(fmt.Sprintf("/src/target/%s/debug/just-mcp", target)), nil
}

// BuildRelease creates an optimized release build
func (m *JustMcp) BuildRelease(
	ctx context.Context,
	source *dagger.Directory,
	// +optional
	// +default="linux/amd64"
	platform string,
) (*dagger.File, error) {
	target := platformToTarget(platform)
	
	// Always use linux/amd64 container for cross-compilation
	container := dag.Container().
		From("rust:1.88.0").
		WithDirectory("/src", source).
		WithWorkdir("/src")

	binaryName := "just-mcp"
	
	// For native x86_64 Linux, don't specify target to avoid issues
	if platform == "linux/amd64" {
		return container.
			WithExec([]string{"cargo", "build", "--release"}).
			File("/src/target/release/" + binaryName), nil
	}
	
	// Setup cross-compilation for other targets
	container = setupCrossCompilation(container, target)

	return container.
		WithExec([]string{"cargo", "build", "--release", "--target", target}).
		File(fmt.Sprintf("/src/target/%s/release/%s", target, binaryName)), nil
}

// Package creates a release archive with binary, README, and LICENSE
func (m *JustMcp) Package(
	ctx context.Context,
	source *dagger.Directory,
	// +optional
	// +default="linux/amd64"
	platform string,
	// +optional
	// +default="v0.1.0"
	version string,
) (*dagger.File, error) {
	binary, err := m.BuildRelease(ctx, source, platform)
	if err != nil {
		return nil, err
	}

	archiveName := fmt.Sprintf("just-mcp-%s-%s", version, platformToArchiveName(platform))
	
	container := dag.Container().
		From("alpine:latest").
		WithExec([]string{"apk", "add", "--no-cache", "tar", "gzip", "zip"}).
		WithDirectory("/archive", dag.Directory().
			WithFile("just-mcp", binary).
			WithFile("README.md", source.File("README.md")).
			WithFile("LICENSE", source.File("LICENSE")))


	return container.
		WithWorkdir("/archive").
		WithExec([]string{"tar", "czf", fmt.Sprintf("/%s.tar.gz", archiveName), "."}).
		File(fmt.Sprintf("/%s.tar.gz", archiveName)), nil
}

// CI runs the complete CI pipeline (format, lint, test)
func (m *JustMcp) CI(ctx context.Context, source *dagger.Directory) (string, error) {
	// Run format check
	fmt.Println("🔍 Checking code formatting...")
	if _, err := m.Format(ctx, source); err != nil {
		return "", fmt.Errorf("format check failed: %w", err)
	}
	
	// Run clippy
	fmt.Println("📋 Running clippy linter...")
	if _, err := m.Lint(ctx, source); err != nil {
		return "", fmt.Errorf("clippy failed: %w", err)
	}
	
	// Run tests on Linux platforms only (cross-platform testing requires native runners)
	platforms := []string{"linux/amd64"}
	for _, platform := range platforms {
		fmt.Printf("🧪 Running tests on %s...\n", platform)
		if _, err := m.Test(ctx, source, platform); err != nil {
			return "", fmt.Errorf("tests failed on %s: %w", platform, err)
		}
	}
	
	// Generate coverage on Linux
	fmt.Println("📊 Generating code coverage...")
	if _, err := m.Coverage(ctx, source); err != nil {
		fmt.Println("⚠️  Coverage generation failed (non-critical)")
	}
	
	return "✅ CI pipeline completed successfully!", nil
}

// Release builds releases for Linux platforms only
// macOS builds require native macOS environment due to framework dependencies
func (m *JustMcp) Release(
	ctx context.Context,
	source *dagger.Directory,
	// +optional
	// +default="v0.1.0"
	version string,
) ([]*dagger.File, error) {
	platforms := []struct {
		platform string
		name     string
	}{
		{"linux/amd64", "x86_64-unknown-linux-gnu"},
		{"linux/arm64", "aarch64-unknown-linux-gnu"},
	}

	var releases []*dagger.File
	
	for _, p := range platforms {
		fmt.Printf("📦 Building release for %s...\n", p.name)
		
		archive, err := m.Package(ctx, source, p.platform, version)
		if err != nil {
			return nil, fmt.Errorf("failed to package %s: %w", p.name, err)
		}
		
		releases = append(releases, archive)
	}
	
	return releases, nil
}


// Helper functions

func platformToTarget(platform string) string {
	targets := map[string]string{
		"linux/amd64":   "x86_64-unknown-linux-gnu",
		"linux/arm64":   "aarch64-unknown-linux-gnu",
		"darwin/amd64":  "x86_64-apple-darwin",
		"darwin/arm64":  "aarch64-apple-darwin",
	}
	
	if target, ok := targets[platform]; ok {
		return target
	}
	return "x86_64-unknown-linux-gnu"
}

func platformToArchiveName(platform string) string {
	return platformToTarget(platform)
}

// setupCrossCompilation configures the container for cross-compilation
func setupCrossCompilation(container *dagger.Container, target string) *dagger.Container {
	// Always add the target
	container = container.WithExec([]string{"rustup", "target", "add", target})
	
	// Install cross-compilation tools based on target
	switch target {
	case "aarch64-unknown-linux-gnu":
		// ARM64 Linux
		return container.
			WithExec([]string{"apt-get", "update"}).
			WithExec([]string{"apt-get", "install", "-y", "gcc-aarch64-linux-gnu"}).
			WithEnvVariable("CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER", "aarch64-linux-gnu-gcc")
		
	case "x86_64-apple-darwin", "aarch64-apple-darwin":
		// For now, we'll skip macOS cross-compilation as it requires more complex setup
		// We'll document this limitation and handle macOS builds separately
		return container
		
	default:
		// x86_64-unknown-linux-gnu - no additional tools needed
		return container
	}
}

// ZigbuildSingle builds a release for a single platform using cargo-zigbuild
// This provides cross-compilation support for macOS from Linux
func (m *JustMcp) ZigbuildSingle(
	ctx context.Context,
	source *dagger.Directory,
	target string,
	// +optional
	// +default="v0.1.0"
	version string,
) (*dagger.File, error) {
	// Use the official cargo-zigbuild Docker image which includes macOS SDK
	container := dag.Container().
		From("ghcr.io/rust-cross/cargo-zigbuild:latest").
		WithDirectory("/src", source).
		WithWorkdir("/src")
	
	// Handle universal2-apple-darwin specially - it needs both Apple targets
	if target == "universal2-apple-darwin" {
		fmt.Println("📦 Adding Apple targets for universal2 binary...")
		container = container.
			WithExec([]string{"rustup", "target", "add", "x86_64-apple-darwin", "aarch64-apple-darwin"})
	} else {
		fmt.Printf("📦 Adding Rust target %s...\n", target)
		container = container.
			WithExec([]string{"rustup", "target", "add", target})
	}
	
	fmt.Printf("📦 Building release for %s...\n", target)
	// Build with cargo-zigbuild
	container = container.
		WithExec([]string{"cargo", "zigbuild", "--release", "--target", target})
	
	// Get the binary path
	binaryPath := fmt.Sprintf("/src/target/%s/release/just-mcp", target)
	
	// Extract the binary from the built container
	binary := container.File(binaryPath)
	
	// Create archive with binary, README, and LICENSE
	archiveName := fmt.Sprintf("just-mcp-%s-%s", version, target)
	
	archiveContainer := dag.Container().
		From("alpine:latest").
		WithExec([]string{"apk", "add", "--no-cache", "tar", "gzip"}).
		WithDirectory("/archive", dag.Directory().
			WithFile("just-mcp", binary).
			WithFile("README.md", source.File("README.md")).
			WithFile("LICENSE", source.File("LICENSE")))
	
	archive := archiveContainer.
		WithWorkdir("/archive").
		WithExec([]string{"tar", "czf", fmt.Sprintf("/%s.tar.gz", archiveName), "."}).
		File(fmt.Sprintf("/%s.tar.gz", archiveName))
	
	return archive, nil
}

// ReleaseZigbuild builds releases for all platforms using cargo-zigbuild
// This provides cross-compilation support for macOS from Linux
func (m *JustMcp) ReleaseZigbuild(
	ctx context.Context,
	source *dagger.Directory,
	// +optional
	// +default="v0.1.0"
	version string,
) ([]*dagger.File, error) {
	platforms := []string{
		"x86_64-unknown-linux-gnu",
		"aarch64-unknown-linux-gnu",
		"x86_64-apple-darwin",
		"aarch64-apple-darwin",
		"universal2-apple-darwin",
	}
	
	var releases []*dagger.File
	
	// Build each platform using ZigbuildSingle
	for _, target := range platforms {
		archive, err := m.ZigbuildSingle(ctx, source, target, version)
		if err != nil {
			return nil, fmt.Errorf("failed to build %s: %w", target, err)
		}
		releases = append(releases, archive)
	}
	
	return releases, nil
}
