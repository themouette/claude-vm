/// Integration tests for network isolation
///
/// These tests require limactl to be installed and take significant time.
/// Run with: cargo test --test test_network_isolation_integration -- --ignored --test-threads=1
///
/// Tests are run sequentially (test-threads=1) because they share VM templates.
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
fn test_network_isolation_denylist_mode() {
    let config = r#"
[tools]
network_isolation = true

[security.network]
enabled = true
mode = "denylist"
blocked_domains = ["example.com", "httpbin.org"]
bypass_domains = ["localhost", "127.0.0.1", "*.local"]
block_tcp_udp = true
block_private_networks = true
block_metadata_services = true
"#;

    let project_dir = create_test_project(config);

    // Run setup
    run_setup(&project_dir.path().to_path_buf()).expect("Setup should succeed");

    // Run test script inline - tests both HTTP and HTTPS filtering
    let test_script = r#"
        test_url() {
            url="$1"
            expected="$2"
            label="$3"
            status=$(curl -s -o /dev/null -w "%{http_code}" "$url" --connect-timeout 10 2>&1)
            if echo "$status" | grep -q "$expected"; then
                echo "PASS: $label (got $status)"
                return 0
            else
                echo "FAIL: $label (expected $expected, got $status)"
                return 1
            fi
        }

        # Test HTTP filtering
        test_url "http://example.com" "403" "Example.com HTTP (blocked)"
        test_url "http://httpbin.org" "403" "HTTPBin HTTP (blocked)"

        # Test HTTPS filtering - verify mitmproxy is intercepting
        test_url "https://api.github.com" "200" "GitHub HTTPS (allowed)"

        # Verify HTTPS interception is working by checking certificate issuer
        echo ""
        echo "=== Verifying HTTPS Interception ==="
        CERT_ISSUER=$(curl -v https://api.github.com 2>&1 | grep -i "issuer:" | head -1)
        if echo "$CERT_ISSUER" | grep -iq "mitmproxy"; then
            echo "PASS: HTTPS interception working (cert from mitmproxy)"
        else
            echo "FAIL: HTTPS not intercepted (cert issuer: $CERT_ISSUER)"
        fi

        # Check mitmproxy stats to verify requests were intercepted
        echo ""
        echo "=== Mitmproxy Statistics ==="
        if [ -f /tmp/mitmproxy_stats.json ]; then
            cat /tmp/mitmproxy_stats.json
        else
            echo "FAIL: Stats file not found"
        fi
    "#;
    let output = run_shell_command(&project_dir.path().to_path_buf(), test_script)
        .expect("Command should run");

    // Check all tests passed
    assert!(
        output.contains("PASS: GitHub HTTPS"),
        "GitHub should be allowed in denylist mode\nOutput: {}",
        output
    );

    assert!(
        output.contains("PASS: Example.com HTTP"),
        "example.com should be blocked\nOutput: {}",
        output
    );

    assert!(
        output.contains("PASS: HTTPBin HTTP"),
        "httpbin.org should be blocked\nOutput: {}",
        output
    );

    // Verify HTTPS interception is working
    assert!(
        output.contains("PASS: HTTPS interception working"),
        "HTTPS must be intercepted by mitmproxy\nOutput: {}",
        output
    );

    // Ensure no failures
    assert!(
        !output.contains("FAIL:"),
        "All tests should pass\nOutput: {}",
        output
    );
}

#[test]
#[ignore] // Requires limactl and takes time
fn test_network_isolation_allowlist_mode() {
    let config = r#"
[tools]
network_isolation = true

[security.network]
enabled = true
mode = "allowlist"
allowed_domains = ["*.github.com", "github.com"]
bypass_domains = ["localhost", "127.0.0.1", "*.local"]
block_tcp_udp = true
block_private_networks = true
block_metadata_services = true
"#;

    let project_dir = create_test_project(config);

    // Run setup
    run_setup(&project_dir.path().to_path_buf()).expect("Setup should succeed");

    // Run test script inline - verify allowlist blocks non-allowed domains
    let test_script = r#"
        test_url() {
            url="$1"
            expected="$2"
            label="$3"
            status=$(curl -s -o /dev/null -w "%{http_code}" "$url" --connect-timeout 10 2>&1)
            if echo "$status" | grep -q "$expected"; then
                echo "PASS: $label (got $status)"
                return 0
            else
                echo "FAIL: $label (expected $expected, got $status)"
                return 1
            fi
        }

        # Test allowed domain
        test_url "https://api.github.com" "200" "GitHub HTTPS (allowed)"

        # Test blocked domains (not in allowlist)
        test_url "http://example.com" "403" "Example.com HTTP (blocked)"
        test_url "http://httpbin.org" "403" "HTTPBin HTTP (blocked)"
        test_url "https://httpbin.org/get" "403" "HTTPBin HTTPS (blocked)"

        # Verify HTTPS interception is working
        echo ""
        echo "=== Verifying HTTPS Interception ==="
        CERT_ISSUER=$(curl -v https://api.github.com 2>&1 | grep -i "issuer:" | head -1)
        if echo "$CERT_ISSUER" | grep -iq "mitmproxy"; then
            echo "PASS: HTTPS interception working (cert from mitmproxy)"
        else
            echo "FAIL: HTTPS not intercepted (cert issuer: $CERT_ISSUER)"
        fi
    "#;
    let output = run_shell_command(&project_dir.path().to_path_buf(), test_script)
        .expect("Command should run");

    // Check all tests passed
    assert!(
        output.contains("PASS: GitHub HTTPS"),
        "GitHub should be allowed in allowlist mode\nOutput: {}",
        output
    );

    assert!(
        output.contains("PASS: Example.com HTTP"),
        "example.com should be blocked in allowlist mode\nOutput: {}",
        output
    );

    assert!(
        output.contains("PASS: HTTPBin HTTP"),
        "httpbin.org HTTP should be blocked in allowlist mode\nOutput: {}",
        output
    );

    assert!(
        output.contains("PASS: HTTPBin HTTPS"),
        "httpbin.org HTTPS should be blocked in allowlist mode\nOutput: {}",
        output
    );

    // Verify HTTPS interception is working
    assert!(
        output.contains("PASS: HTTPS interception working"),
        "HTTPS must be intercepted by mitmproxy\nOutput: {}",
        output
    );

    assert!(
        !output.contains("FAIL:"),
        "All tests should pass\nOutput: {}",
        output
    );
}

#[test]
#[ignore] // Requires limactl and takes time
fn test_network_isolation_proxy_running() {
    let config = r#"
[tools]
network_isolation = true

[security.network]
enabled = true
mode = "denylist"
blocked_domains = ["example.com"]
bypass_domains = ["localhost", "127.0.0.1", "*.local"]
block_tcp_udp = true
block_private_networks = true
block_metadata_services = true
"#;

    let project_dir = create_test_project(config);

    // Run setup
    run_setup(&project_dir.path().to_path_buf()).expect("Setup should succeed");

    // Check that mitmproxy is running
    let output = run_shell_command(
        &project_dir.path().to_path_buf(),
        "ps aux | grep -v grep | grep mitmdump",
    )
    .expect("Command should run");

    assert!(
        output.contains("mitmdump"),
        "mitmproxy should be running, got: {}",
        output
    );

    // Check that proxy environment variables are set
    let output = run_shell_command(&project_dir.path().to_path_buf(), "env | grep -i proxy")
        .expect("Command should run");

    assert!(
        output.contains("http_proxy=http://localhost:8080"),
        "http_proxy should be set, got: {}",
        output
    );
    assert!(
        output.contains("https_proxy=http://localhost:8080"),
        "https_proxy should be set, got: {}",
        output
    );
}

#[test]
#[ignore] // Requires limactl and takes time
fn test_network_isolation_iptables_configured() {
    let config = r#"
[tools]
network_isolation = true

[security.network]
enabled = true
mode = "denylist"
blocked_domains = []
bypass_domains = ["localhost", "127.0.0.1", "*.local"]
block_tcp_udp = true
block_private_networks = true
block_metadata_services = true
"#;

    let project_dir = create_test_project(config);

    // Run setup
    run_setup(&project_dir.path().to_path_buf()).expect("Setup should succeed");

    // Check iptables rules are configured
    let output = run_shell_command(
        &project_dir.path().to_path_buf(),
        "sudo iptables -L OUTPUT -n",
    )
    .expect("Command should run");

    // Should have user allow rules
    assert!(
        output.contains("owner UID match"),
        "iptables should have user allow rules, got: {}",
        output
    );

    // Should have private network blocks
    assert!(
        output.contains("192.168.0.0/16") || output.contains("REJECT"),
        "iptables should have reject rules, got: {}",
        output
    );
}

#[test]
#[ignore] // Requires limactl and takes time
fn test_network_isolation_https_interception() {
    let config = r#"
[tools]
network_isolation = true

[security.network]
enabled = true
mode = "allowlist"
allowed_domains = ["*.github.com", "github.com"]
bypass_domains = ["localhost", "127.0.0.1", "*.local"]
block_tcp_udp = true
block_private_networks = true
block_metadata_services = true
"#;

    let project_dir = create_test_project(config);

    // Run setup
    run_setup(&project_dir.path().to_path_buf()).expect("Setup should succeed");

    // Run test script inline - verify HTTPS interception and filtering
    let test_script = r#"
        test_url() {
            url="$1"
            expected="$2"
            label="$3"
            status=$(curl -s -o /dev/null -w "%{http_code}" "$url" --connect-timeout 10 2>&1)
            if echo "$status" | grep -q "$expected"; then
                echo "PASS: $label (got $status)"
                return 0
            else
                echo "FAIL: $label (expected $expected, got $status)"
                return 1
            fi
        }

        # Test HTTPS filtering
        test_url "https://api.github.com" "200" "HTTPS allowed domain"
        test_url "https://httpbin.org/get" "403" "HTTPS blocked domain"

        # Verify certificate is from mitmproxy (proves interception)
        echo ""
        echo "=== Certificate Verification ==="
        CERT_ISSUER=$(curl -v https://api.github.com 2>&1 | grep -i "issuer:" | head -1)
        echo "Certificate issuer: $CERT_ISSUER"

        if echo "$CERT_ISSUER" | grep -iq "mitmproxy"; then
            echo "PASS: Certificate from mitmproxy (HTTPS is being intercepted)"
        else
            echo "FAIL: Certificate not from mitmproxy (HTTPS passthrough, not intercepted)"
        fi

        # Check mitmproxy request stats
        echo ""
        echo "=== Mitmproxy Statistics ==="
        if [ -f /tmp/mitmproxy_stats.json ]; then
            cat /tmp/mitmproxy_stats.json
            REQUESTS=$(cat /tmp/mitmproxy_stats.json | python3 -c "import sys, json; print(json.load(sys.stdin).get('requests_total', 0))" 2>/dev/null || echo "0")
            echo "Total requests intercepted: $REQUESTS"
            if [ "$REQUESTS" -gt "0" ] 2>/dev/null; then
                echo "PASS: Mitmproxy is intercepting requests"
            else
                echo "FAIL: No requests intercepted by mitmproxy"
            fi
        else
            echo "FAIL: Stats file not found"
        fi
    "#;
    let output = run_shell_command(&project_dir.path().to_path_buf(), test_script)
        .expect("Command should run");

    // Check both tests passed
    assert!(
        output.contains("PASS: HTTPS allowed domain"),
        "HTTPS to allowed domain should work\nOutput: {}",
        output
    );

    assert!(
        output.contains("PASS: HTTPS blocked domain"),
        "HTTPS to blocked domain should be rejected\nOutput: {}",
        output
    );

    // Verify interception is actually happening
    assert!(
        output.contains("PASS: Certificate from mitmproxy"),
        "Certificate must be from mitmproxy (proves HTTPS interception)\nOutput: {}",
        output
    );

    assert!(
        output.contains("PASS: Mitmproxy is intercepting requests"),
        "Mitmproxy stats should show intercepted requests\nOutput: {}",
        output
    );

    assert!(
        !output.contains("FAIL:"),
        "All tests should pass\nOutput: {}",
        output
    );
}
