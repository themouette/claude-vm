use std::env;
use std::process::Command;

fn main() {
    let version = env::var("CARGO_PKG_VERSION").unwrap();
    let profile = env::var("PROFILE").unwrap();

    let full_version = if profile == "debug" {
        // Dev build - add git hash
        let git_hash = get_git_hash().unwrap_or_else(|| "unknown".to_string());
        let dirty = is_git_dirty();

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
}

fn get_git_hash() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--short=8", "HEAD"])
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
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
