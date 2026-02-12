/// Integration test for bypass domains
///
/// Verifies that domains in bypass_domains truly bypass the proxy
/// and do not have their HTTPS traffic intercepted by mitmproxy.
///
/// Run with: cargo test --test test_bypass_domains -- --ignored
use assert_cmd::Command;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create a test directory with a .claude-vm.toml file
fn create_test_project(config_content: &str) -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join(".claude-vm.toml");
    fs::write(&config_path, config_content).expect("Failed to write config file");
    temp_dir
}

/// Helper to run setup command in a test directory
fn run_setup(project_dir: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.current_dir(project_dir)
        .args(["setup", "--no-agent-install"]);

    let output = cmd.output()?;
    if !output.status.success() {
        eprintln!("Setup failed:");
        eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        return Err("Setup command failed".into());
    }
    Ok(())
}

/// Helper to run a shell command in the VM and return output
fn run_shell_command(
    project_dir: &PathBuf,
    command: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("claude-vm"));
    cmd.current_dir(project_dir)
        .args(["shell", "bash", "-c", command]);

    let output = cmd.output()?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[test]
#[ignore] // Requires limactl and takes time
fn test_bypass_domains_skip_proxy() {
    let config = r#"
[tools]
network_isolation = true

[security.network]
enabled = true
mode = "allowlist"
# Both domains are allowed, but only one is bypassed
allowed_domains = ["*.github.com", "github.com", "httpbin.org"]
bypass_domains = ["localhost", "127.0.0.1", "*.local", "api.github.com", "*.github.com", "github.com"]
block_tcp_udp = true
block_private_networks = true
block_metadata_services = true
"#;

    let project_dir = create_test_project(config);

    // Run setup
    run_setup(&project_dir.path().to_path_buf()).expect("Setup should succeed");

    // Test that bypass domains skip mitmproxy interception
    let test_script = r#"
        echo "=== Testing Bypass Domains ==="

        # Test GitHub (should be bypassed - direct connection)
        echo ""
        echo "Testing GitHub (bypass domain):"
        GITHUB_CERT=$(curl -v https://api.github.com 2>&1 | grep -i "issuer:" | head -1)
        echo "Certificate issuer: $GITHUB_CERT"
        if echo "$GITHUB_CERT" | grep -iq "mitmproxy"; then
            echo "FAIL: GitHub went through mitmproxy (should be bypassed)"
        else
            echo "PASS: GitHub bypassed proxy (direct connection)"
        fi

        # Test HTTPBin (should go through proxy - intercepted)
        echo ""
        echo "Testing HTTPBin (allowed but not bypassed):"
        HTTPBIN_CERT=$(curl -v https://httpbin.org/get 2>&1 | grep -i "issuer:" | head -1)
        echo "Certificate issuer: $HTTPBIN_CERT"
        if echo "$HTTPBIN_CERT" | grep -iq "mitmproxy"; then
            echo "PASS: HTTPBin went through mitmproxy (intercepted)"
        else
            echo "FAIL: HTTPBin bypassed proxy (should be intercepted)"
        fi

        # Check NO_PROXY environment variable includes bypass domains
        echo ""
        echo "=== NO_PROXY Configuration ==="
        echo "NO_PROXY=$NO_PROXY"
        if echo "$NO_PROXY" | grep -q "github.com"; then
            echo "PASS: NO_PROXY includes github.com"
        else
            echo "FAIL: NO_PROXY should include github.com"
        fi

        # Check mitmproxy stats - GitHub should not appear
        echo ""
        echo "=== Mitmproxy Statistics ==="
        if [ -f /tmp/mitmproxy_stats.json ]; then
            cat /tmp/mitmproxy_stats.json
        fi

        # Check mitmproxy log - GitHub should not appear
        echo ""
        echo "=== Checking Mitmproxy Logs ==="
        if [ -f /tmp/mitmproxy.log ]; then
            if grep -i "github" /tmp/mitmproxy.log | tail -5; then
                echo "WARN: GitHub requests found in mitmproxy log (should be bypassed)"
            else
                echo "PASS: No GitHub requests in mitmproxy log (correctly bypassed)"
            fi
        fi
    "#;

    let output = run_shell_command(&project_dir.path().to_path_buf(), test_script)
        .expect("Command should run");

    // Verify GitHub bypassed the proxy
    assert!(
        output.contains("PASS: GitHub bypassed proxy"),
        "GitHub should bypass mitmproxy (direct connection)\nOutput: {}",
        output
    );

    // Verify HTTPBin went through the proxy
    assert!(
        output.contains("PASS: HTTPBin went through mitmproxy"),
        "HTTPBin should be intercepted by mitmproxy\nOutput: {}",
        output
    );

    // Verify NO_PROXY includes bypass domains
    assert!(
        output.contains("PASS: NO_PROXY includes github.com"),
        "NO_PROXY should include bypass domains\nOutput: {}",
        output
    );

    // No failures
    assert!(
        !output.contains("FAIL:"),
        "All tests should pass\nOutput: {}",
        output
    );
}
