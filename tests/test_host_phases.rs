use claude_vm::config::Config;

/// Test that host.before_setup phases are properly parsed from TOML
#[test]
fn test_host_before_setup_parsing() {
    let toml = r#"
        [[phase.host.before_setup]]
        name = "host-phase"
        script = "echo 'before setup'"
    "#;

    let config: Config = toml::from_str(toml).expect("Failed to parse TOML");
    assert_eq!(config.phase.host.before_setup.len(), 1);
    assert_eq!(config.phase.host.before_setup[0].name, "host-phase");
    assert_eq!(
        config.phase.host.before_setup[0].script,
        Some("echo 'before setup'".to_string())
    );
}

/// Test that host.after_setup phases are properly parsed from TOML
#[test]
fn test_host_after_setup_parsing() {
    let toml = r#"
        [[phase.host.after_setup]]
        name = "cleanup-phase"
        script = "echo 'after setup'"
    "#;

    let config: Config = toml::from_str(toml).expect("Failed to parse TOML");
    assert_eq!(config.phase.host.after_setup.len(), 1);
    assert_eq!(config.phase.host.after_setup[0].name, "cleanup-phase");
}

/// Test that host.before_runtime phases are properly parsed from TOML
#[test]
fn test_host_before_runtime_parsing() {
    let toml = r#"
        [[phase.host.before_runtime]]
        name = "pre-runtime"
        script = "echo 'before runtime'"
    "#;

    let config: Config = toml::from_str(toml).expect("Failed to parse TOML");
    assert_eq!(config.phase.host.before_runtime.len(), 1);
    assert_eq!(config.phase.host.before_runtime[0].name, "pre-runtime");
}

/// Test that host.after_runtime phases are properly parsed from TOML
#[test]
fn test_host_after_runtime_parsing() {
    let toml = r#"
        [[phase.host.after_runtime]]
        name = "post-runtime"
        script = "echo 'after runtime'"
    "#;

    let config: Config = toml::from_str(toml).expect("Failed to parse TOML");
    assert_eq!(config.phase.host.after_runtime.len(), 1);
    assert_eq!(config.phase.host.after_runtime[0].name, "post-runtime");
}

/// Test that host.teardown phases are properly parsed from TOML
#[test]
fn test_host_teardown_parsing() {
    let toml = r#"
        [[phase.host.teardown]]
        name = "teardown-phase"
        script = "echo 'teardown'"
    "#;

    let config: Config = toml::from_str(toml).expect("Failed to parse TOML");
    assert_eq!(config.phase.host.teardown.len(), 1);
    assert_eq!(config.phase.host.teardown[0].name, "teardown-phase");
}

/// Test that all host phase types can be defined together
#[test]
fn test_all_host_phases_together() {
    let toml = r#"
        [[phase.host.before_setup]]
        name = "before-setup"
        script = "echo '1'"

        [[phase.host.after_setup]]
        name = "after-setup"
        script = "echo '2'"

        [[phase.host.before_runtime]]
        name = "before-runtime"
        script = "echo '3'"

        [[phase.host.after_runtime]]
        name = "after-runtime"
        script = "echo '4'"

        [[phase.host.teardown]]
        name = "teardown"
        script = "echo '5'"
    "#;

    let config: Config = toml::from_str(toml).expect("Failed to parse TOML");
    assert_eq!(config.phase.host.before_setup.len(), 1);
    assert_eq!(config.phase.host.after_setup.len(), 1);
    assert_eq!(config.phase.host.before_runtime.len(), 1);
    assert_eq!(config.phase.host.after_runtime.len(), 1);
    assert_eq!(config.phase.host.teardown.len(), 1);
}

/// Test that multiple phases of the same host type are preserved in order
#[test]
fn test_multiple_host_phases_same_type() {
    let toml = r#"
        [[phase.host.before_setup]]
        name = "first"
        script = "echo 'first'"

        [[phase.host.before_setup]]
        name = "second"
        script = "echo 'second'"

        [[phase.host.before_setup]]
        name = "third"
        script = "echo 'third'"
    "#;

    let config: Config = toml::from_str(toml).expect("Failed to parse TOML");
    assert_eq!(config.phase.host.before_setup.len(), 3);
    assert_eq!(config.phase.host.before_setup[0].name, "first");
    assert_eq!(config.phase.host.before_setup[1].name, "second");
    assert_eq!(config.phase.host.before_setup[2].name, "third");
}

/// Test that host phases with env vars are properly parsed
#[test]
fn test_host_phase_with_env_vars() {
    let toml = r#"
        [[phase.host.before_setup]]
        name = "with-env"
        env = { HOST_VAR = "value", DEBUG = "true" }
        script = "echo $HOST_VAR"
    "#;

    let config: Config = toml::from_str(toml).expect("Failed to parse TOML");
    let phase = &config.phase.host.before_setup[0];
    assert_eq!(phase.env.get("HOST_VAR"), Some(&"value".to_string()));
    assert_eq!(phase.env.get("DEBUG"), Some(&"true".to_string()));
}

/// Test that host phases with conditional execution are properly parsed
#[test]
fn test_host_phase_conditional() {
    let toml = r#"
        [[phase.host.before_runtime]]
        name = "conditional"
        when = "test -f ~/.ssh/id_rsa"
        script = "echo 'ssh key exists'"
    "#;

    let config: Config = toml::from_str(toml).expect("Failed to parse TOML");
    let phase = &config.phase.host.before_runtime[0];
    assert_eq!(phase.when, Some("test -f ~/.ssh/id_rsa".to_string()));
}

/// Test that host phases with continue_on_error are properly parsed
#[test]
fn test_host_phase_continue_on_error() {
    let toml = r#"
        [[phase.host.teardown]]
        name = "optional-cleanup"
        continue_on_error = true
        script = "rm -f /tmp/cleanup"
    "#;

    let config: Config = toml::from_str(toml).expect("Failed to parse TOML");
    let phase = &config.phase.host.teardown[0];
    assert!(phase.continue_on_error);
}

/// Test that flatten_host_phases merges host.before_setup into before_setup
#[test]
fn test_flatten_host_before_setup() {
    let toml = r#"
        [[phase.host.before_setup]]
        name = "host-phase"
        script = "echo 'from host'"

        [[phase.before_setup]]
        name = "direct-phase"
        script = "echo 'direct'"
    "#;

    let mut config: Config = toml::from_str(toml).expect("Failed to parse TOML");

    // Before flattening
    assert_eq!(config.phase.host.before_setup.len(), 1);
    assert_eq!(config.phase.before_setup.len(), 1);

    // Flatten
    config.phase.flatten_host_phases();

    // After flattening: host phases should be moved to direct array
    assert_eq!(config.phase.host.before_setup.len(), 0);
    assert_eq!(config.phase.before_setup.len(), 2);
    assert_eq!(config.phase.before_setup[0].name, "direct-phase");
    assert_eq!(config.phase.before_setup[1].name, "host-phase");
}

/// Test that flatten_host_phases merges host.after_setup into after_setup
#[test]
fn test_flatten_host_after_setup() {
    let toml = r#"
        [[phase.host.after_setup]]
        name = "host-cleanup"
        script = "echo 'cleanup'"
    "#;

    let mut config: Config = toml::from_str(toml).expect("Failed to parse TOML");

    // Before flattening
    assert_eq!(config.phase.host.after_setup.len(), 1);
    assert_eq!(config.phase.after_setup.len(), 0);

    // Flatten
    config.phase.flatten_host_phases();

    // After flattening
    assert_eq!(config.phase.host.after_setup.len(), 0);
    assert_eq!(config.phase.after_setup.len(), 1);
    assert_eq!(config.phase.after_setup[0].name, "host-cleanup");
}

/// Test that flatten_host_phases merges host.before_runtime into before_runtime
#[test]
fn test_flatten_host_before_runtime() {
    let toml = r#"
        [[phase.host.before_runtime]]
        name = "host-pre-runtime"
        script = "echo 'pre'"

        [[phase.before_runtime]]
        name = "direct-pre-runtime"
        script = "echo 'also pre'"
    "#;

    let mut config: Config = toml::from_str(toml).expect("Failed to parse TOML");

    // Flatten
    config.phase.flatten_host_phases();

    // After flattening
    assert_eq!(config.phase.host.before_runtime.len(), 0);
    assert_eq!(config.phase.before_runtime.len(), 2);
    assert_eq!(config.phase.before_runtime[0].name, "direct-pre-runtime");
    assert_eq!(config.phase.before_runtime[1].name, "host-pre-runtime");
}

/// Test that flatten_host_phases merges host.after_runtime into after_runtime
#[test]
fn test_flatten_host_after_runtime() {
    let toml = r#"
        [[phase.host.after_runtime]]
        name = "host-post-runtime"
        script = "echo 'post'"
    "#;

    let mut config: Config = toml::from_str(toml).expect("Failed to parse TOML");

    // Flatten
    config.phase.flatten_host_phases();

    // After flattening, host.after_runtime is NOT flattened (no backward compat)
    // phase.after_runtime is now for VM phases, not host phases
    assert_eq!(config.phase.host.after_runtime.len(), 1);
    assert_eq!(config.phase.after_runtime.len(), 0);
    assert_eq!(config.phase.host.after_runtime[0].name, "host-post-runtime");
}

/// Test that host.teardown phases are properly loaded
/// Note: [[phase.teardown]] is no longer supported (backward compat removed)
#[test]
fn test_host_teardown_phases() {
    let toml = r#"
        [[phase.host.teardown]]
        name = "teardown-1"
        script = "echo 'teardown 1'"

        [[phase.host.teardown]]
        name = "teardown-2"
        script = "echo 'teardown 2'"
    "#;

    let config: Config = toml::from_str(toml).expect("Failed to parse TOML");

    // Verify both teardown phases are loaded
    assert_eq!(config.phase.host.teardown.len(), 2);
    assert_eq!(config.phase.host.teardown[0].name, "teardown-1");
    assert_eq!(config.phase.host.teardown[1].name, "teardown-2");
}

/// Test that flattening all host phases at once works correctly
#[test]
fn test_flatten_all_host_phases() {
    let toml = r#"
        [[phase.host.before_setup]]
        name = "h-before-setup"
        script = "echo '1'"

        [[phase.host.after_setup]]
        name = "h-after-setup"
        script = "echo '2'"

        [[phase.host.before_runtime]]
        name = "h-before-runtime"
        script = "echo '3'"

        [[phase.host.after_runtime]]
        name = "h-after-runtime"
        script = "echo '4'"

        [[phase.host.teardown]]
        name = "h-teardown"
        script = "echo '5'"
    "#;

    let mut config: Config = toml::from_str(toml).expect("Failed to parse TOML");

    // Flatten
    config.phase.flatten_host_phases();

    // After flattening, host.teardown and host.after_runtime should still have phases (no backward compat)
    assert_eq!(config.phase.host.before_setup.len(), 0);
    assert_eq!(config.phase.host.after_setup.len(), 0);
    assert_eq!(config.phase.host.before_runtime.len(), 0);
    assert_eq!(config.phase.host.after_runtime.len(), 1); // NOT flattened
    assert_eq!(config.phase.host.teardown.len(), 1);

    // Direct arrays should have the phases (except teardown and after_runtime which no longer exist as backward compat)
    assert_eq!(config.phase.before_setup.len(), 1);
    assert_eq!(config.phase.after_setup.len(), 1);
    assert_eq!(config.phase.before_runtime.len(), 1);
    assert_eq!(config.phase.after_runtime.len(), 0); // Now for VM phases only
}

/// Test that multiple host phases from same type accumulate (merging behavior)
/// Note: This tests the data structure, not the merge method which is private
#[test]
fn test_multiple_host_phases_accumulate() {
    let toml = r#"
        [[phase.host.before_setup]]
        name = "first-phase"
        script = "echo 'first'"

        [[phase.host.before_setup]]
        name = "second-phase"
        script = "echo 'second'"
    "#;

    let config: Config = toml::from_str(toml).expect("Failed to parse TOML");

    // Multiple phases should accumulate in the array
    assert_eq!(config.phase.host.before_setup.len(), 2);
    assert_eq!(config.phase.host.before_setup[0].name, "first-phase");
    assert_eq!(config.phase.host.before_setup[1].name, "second-phase");
}

/// Test that host phases and regular phases can coexist
#[test]
fn test_host_and_regular_phases_coexist() {
    let toml = r#"
        [[phase.host.before_setup]]
        name = "host-before"
        script = "echo 'host before'"

        [[phase.setup]]
        name = "regular-setup"
        script = "echo 'regular setup'"

        [[phase.host.after_setup]]
        name = "host-after"
        script = "echo 'host after'"
    "#;

    let mut config: Config = toml::from_str(toml).expect("Failed to parse TOML");

    // Before flattening
    assert_eq!(config.phase.host.before_setup.len(), 1);
    assert_eq!(config.phase.setup.len(), 1);
    assert_eq!(config.phase.host.after_setup.len(), 1);

    // Flatten
    config.phase.flatten_host_phases();

    // After flattening
    assert_eq!(config.phase.before_setup.len(), 1);
    assert_eq!(config.phase.setup.len(), 1);
    assert_eq!(config.phase.after_setup.len(), 1);
}

/// Test that host phases with file scripts are properly parsed
#[test]
fn test_host_phase_with_file_scripts() {
    let toml = r#"
        [[phase.host.before_setup]]
        name = "with-files"
        script_files = ["./script1.sh", "./script2.sh"]
    "#;

    let config: Config = toml::from_str(toml).expect("Failed to parse TOML");
    let phase = &config.phase.host.before_setup[0];
    assert_eq!(phase.script_files.len(), 2);
    assert_eq!(phase.script_files[0], "./script1.sh");
    assert_eq!(phase.script_files[1], "./script2.sh");
}

/// Test that host phases support mixed inline and file scripts
#[test]
fn test_host_phase_mixed_scripts() {
    let toml = r#"
        [[phase.host.before_runtime]]
        name = "mixed"
        script = "echo 'inline'"
        script_files = ["./file.sh"]
    "#;

    let config: Config = toml::from_str(toml).expect("Failed to parse TOML");
    let phase = &config.phase.host.before_runtime[0];
    assert!(phase.script.is_some());
    assert_eq!(phase.script_files.len(), 1);
}

/// Test realistic host phase configuration
#[test]
fn test_realistic_host_phases() {
    let toml = r#"
        [[phase.host.before_setup]]
        name = "check-requirements"
        script = """
#!/bin/bash
echo 'Checking host requirements...'
command -v limactl || exit 1
"""

        [[phase.host.after_setup]]
        name = "backup-template"
        continue_on_error = true
        script = "tar -czf /tmp/template-backup.tar.gz ~/.lima/template"

        [[phase.host.before_runtime]]
        name = "start-port-forward"
        when = "test -f ~/.ssh/config"
        env = { PORT = "8080" }
        script = "ssh -L $PORT:localhost:$PORT -N vm &"

        [[phase.host.after_runtime]]
        name = "stop-port-forward"
        continue_on_error = true
        script = "pkill -f 'ssh -L'"

        [[phase.host.teardown]]
        name = "cleanup-temp"
        script = "rm -rf /tmp/claude-vm-*"
    "#;

    let config: Config = toml::from_str(toml).expect("Failed to parse TOML");

    // Verify all phases parsed correctly
    assert_eq!(config.phase.host.before_setup.len(), 1);
    assert_eq!(config.phase.host.after_setup.len(), 1);
    assert_eq!(config.phase.host.before_runtime.len(), 1);
    assert_eq!(config.phase.host.after_runtime.len(), 1);
    assert_eq!(config.phase.host.teardown.len(), 1);

    // Verify specific phase properties
    assert_eq!(config.phase.host.before_setup[0].name, "check-requirements");
    assert!(config.phase.host.after_setup[0].continue_on_error);
    assert!(config.phase.host.before_runtime[0].when.is_some());
    assert_eq!(
        config.phase.host.before_runtime[0].env.get("PORT"),
        Some(&"8080".to_string())
    );
}

/// Test that empty host phase arrays are valid
#[test]
fn test_empty_host_phases() {
    let toml = r#"
        [vm]
        disk = 20
        memory = 8
    "#;

    let config: Config = toml::from_str(toml).expect("Failed to parse TOML");
    assert_eq!(config.phase.host.before_setup.len(), 0);
    assert_eq!(config.phase.host.after_setup.len(), 0);
    assert_eq!(config.phase.host.before_runtime.len(), 0);
    assert_eq!(config.phase.host.after_runtime.len(), 0);
    assert_eq!(config.phase.host.teardown.len(), 0);
}
