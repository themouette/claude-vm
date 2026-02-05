use std::env;
use std::process::Command;

fn main() {
    let version = env::var("CARGO_PKG_VERSION").unwrap();
    let profile = env::var("PROFILE").unwrap();

    let full_version = if profile == "debug" {
        // Dev build - add git hash
        let git_hash = get_git_hash().unwrap_or_else(|| "unknown".to_string());
        // Skip dirty check in CI environments (often have build artifacts)
        let dirty = !is_ci_environment() && is_git_dirty();

        if dirty {
            format!("{}-dev+{}.dirty", version, git_hash)
        } else {
            format!("{}-dev+{}", version, git_hash)
        }
    } else {
        // Release build - just the version
        version
    };

    println!("cargo:rustc-env=CLAUDE_VM_VERSION={}", full_version);

    // Re-run if git state changes
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads/");
    println!("cargo:rerun-if-changed=.git/index"); // Detects staging changes
}

fn get_git_hash() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--short=8", "HEAD"])
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        // Print to stderr so developers know why git hash is missing
        eprintln!("cargo:warning=Failed to get git commit hash (not in a git repository or git not available)");
        None
    }
}

fn is_git_dirty() -> bool {
    // Check unstaged changes
    let unstaged = Command::new("git")
        .args(["diff", "--quiet"])
        .status()
        .map(|status| !status.success())
        .unwrap_or(false);

    // Check staged changes
    let staged = Command::new("git")
        .args(["diff", "--cached", "--quiet"])
        .status()
        .map(|status| !status.success())
        .unwrap_or(false);

    unstaged || staged
}

fn is_ci_environment() -> bool {
    // Check common CI environment variables
    env::var("CI").is_ok()
        || env::var("GITHUB_ACTIONS").is_ok()
        || env::var("GITLAB_CI").is_ok()
        || env::var("CIRCLECI").is_ok()
        || env::var("TRAVIS").is_ok()
        || env::var("JENKINS_URL").is_ok()
}
