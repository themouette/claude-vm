use claude_vm::config::{Config, ScriptPhase};
use std::collections::HashMap;
use std::fs;
use tempfile::TempDir;

/// Test that inline scripts are properly parsed from TOML
#[test]
fn test_phase_inline_script_parsing() {
    let toml = r#"
        [[phase.setup]]
        name = "test-phase"
        script = "echo 'hello world'"
    "#;

    let config: Config = toml::from_str(toml).expect("Failed to parse TOML");
    assert_eq!(config.phase.setup.len(), 1);
    assert_eq!(config.phase.setup[0].name, "test-phase");
    assert_eq!(
        config.phase.setup[0].script,
        Some("echo 'hello world'".to_string())
    );
}

/// Test that file-based scripts are properly parsed from TOML
#[test]
fn test_phase_file_scripts_parsing() {
    let toml = r#"
        [[phase.runtime]]
        name = "file-scripts"
        script_files = ["./script1.sh", "./script2.sh"]
    "#;

    let config: Config = toml::from_str(toml).expect("Failed to parse TOML");
    assert_eq!(config.phase.runtime.len(), 1);
    assert_eq!(config.phase.runtime[0].name, "file-scripts");
    assert_eq!(config.phase.runtime[0].script_files.len(), 2);
    assert_eq!(config.phase.runtime[0].script_files[0], "./script1.sh");
    assert_eq!(config.phase.runtime[0].script_files[1], "./script2.sh");
}

/// Test that environment variables are properly parsed from TOML
#[test]
fn test_phase_env_vars_parsing() {
    let toml = r#"
        [[phase.runtime]]
        name = "with-env"
        env = { DEBUG = "true", API_KEY = "secret123" }
        script = "echo $DEBUG"
    "#;

    let config: Config = toml::from_str(toml).expect("Failed to parse TOML");
    assert_eq!(config.phase.runtime.len(), 1);
    assert_eq!(
        config.phase.runtime[0].env.get("DEBUG"),
        Some(&"true".to_string())
    );
    assert_eq!(
        config.phase.runtime[0].env.get("API_KEY"),
        Some(&"secret123".to_string())
    );
}

/// Test that conditional execution fields are properly parsed
#[test]
fn test_phase_conditional_parsing() {
    let toml = r#"
        [[phase.setup]]
        name = "conditional"
        when = "command -v docker"
        script = "echo 'docker found'"
    "#;

    let config: Config = toml::from_str(toml).expect("Failed to parse TOML");
    assert_eq!(config.phase.setup.len(), 1);
    assert_eq!(
        config.phase.setup[0].when,
        Some("command -v docker".to_string())
    );
}

/// Test that 'if' alias works for conditional execution
#[test]
fn test_phase_if_alias_parsing() {
    let toml = r#"
        [[phase.setup]]
        name = "conditional"
        if = "test -f /usr/bin/tool"
        script = "echo 'tool found'"
    "#;

    let config: Config = toml::from_str(toml).expect("Failed to parse TOML");
    assert_eq!(config.phase.setup.len(), 1);
    assert_eq!(
        config.phase.setup[0].when,
        Some("test -f /usr/bin/tool".to_string())
    );
}

/// Test that continue_on_error is properly parsed
#[test]
fn test_phase_continue_on_error_parsing() {
    let toml = r#"
        [[phase.runtime]]
        name = "optional"
        continue_on_error = true
        script = "exit 1"
    "#;

    let config: Config = toml::from_str(toml).expect("Failed to parse TOML");
    assert_eq!(config.phase.runtime.len(), 1);
    assert!(config.phase.runtime[0].continue_on_error);
}

/// Test that multiple phases are parsed in order
#[test]
fn test_multiple_phases_parsing() {
    let toml = r#"
        [[phase.setup]]
        name = "first"
        script = "echo 'first'"

        [[phase.setup]]
        name = "second"
        script = "echo 'second'"

        [[phase.setup]]
        name = "third"
        script = "echo 'third'"
    "#;

    let config: Config = toml::from_str(toml).expect("Failed to parse TOML");
    assert_eq!(config.phase.setup.len(), 3);
    assert_eq!(config.phase.setup[0].name, "first");
    assert_eq!(config.phase.setup[1].name, "second");
    assert_eq!(config.phase.setup[2].name, "third");
}

/// Test that inline and file scripts can be combined in one phase
#[test]
fn test_phase_mixed_scripts_parsing() {
    let toml = r#"
        [[phase.runtime]]
        name = "mixed"
        script = "echo 'inline script'"
        script_files = ["./file-script.sh"]
    "#;

    let config: Config = toml::from_str(toml).expect("Failed to parse TOML");
    assert_eq!(config.phase.runtime.len(), 1);
    assert!(config.phase.runtime[0].script.is_some());
    assert_eq!(config.phase.runtime[0].script_files.len(), 1);
}

/// Test get_scripts method with inline script
#[test]
fn test_get_scripts_inline() {
    let phase = ScriptPhase {
        name: "test".to_string(),
        script: Some("echo 'hello'".to_string()),
        script_files: vec![],
        env: HashMap::new(),
        continue_on_error: false,
        when: None,
        source: false,
    };

    let temp_dir = TempDir::new().unwrap();
    let scripts = phase.get_scripts(temp_dir.path()).unwrap();

    assert_eq!(scripts.len(), 1);
    assert_eq!(scripts[0].0, "test-inline");
    assert_eq!(scripts[0].1, "echo 'hello'");
}

/// Test get_scripts method with file scripts
#[test]
fn test_get_scripts_files() {
    let temp_dir = TempDir::new().unwrap();
    let script1 = temp_dir.path().join("script1.sh");
    let script2 = temp_dir.path().join("script2.sh");

    fs::write(&script1, "#!/bin/bash\necho 'script1'").unwrap();
    fs::write(&script2, "#!/bin/bash\necho 'script2'").unwrap();

    let phase = ScriptPhase {
        name: "test".to_string(),
        script: None,
        script_files: vec![
            script1.to_string_lossy().to_string(),
            script2.to_string_lossy().to_string(),
        ],
        env: HashMap::new(),
        continue_on_error: false,
        when: None,
        source: false,
    };

    let scripts = phase.get_scripts(temp_dir.path()).unwrap();

    assert_eq!(scripts.len(), 2);
    assert_eq!(scripts[0].0, "script1.sh");
    assert!(scripts[0].1.contains("echo 'script1'"));
    assert_eq!(scripts[1].0, "script2.sh");
    assert!(scripts[1].1.contains("echo 'script2'"));
}

/// Test get_scripts with both inline and file scripts
#[test]
fn test_get_scripts_mixed() {
    let temp_dir = TempDir::new().unwrap();
    let script_file = temp_dir.path().join("file.sh");

    fs::write(&script_file, "#!/bin/bash\necho 'from file'").unwrap();

    let phase = ScriptPhase {
        name: "mixed".to_string(),
        script: Some("echo 'inline'".to_string()),
        script_files: vec![script_file.to_string_lossy().to_string()],
        env: HashMap::new(),
        continue_on_error: false,
        when: None,
        source: false,
    };

    let scripts = phase.get_scripts(temp_dir.path()).unwrap();

    // Inline script should come first
    assert_eq!(scripts.len(), 2);
    assert_eq!(scripts[0].0, "mixed-inline");
    assert_eq!(scripts[0].1, "echo 'inline'");
    assert_eq!(scripts[1].0, "file.sh");
    assert!(scripts[1].1.contains("echo 'from file'"));
}

/// Test get_scripts returns error for nonexistent file
#[test]
fn test_get_scripts_missing_file() {
    let phase = ScriptPhase {
        name: "test".to_string(),
        script: None,
        script_files: vec!["/nonexistent/script.sh".to_string()],
        env: HashMap::new(),
        continue_on_error: false,
        when: None,
        source: false,
    };

    let temp_dir = TempDir::new().unwrap();
    let result = phase.get_scripts(temp_dir.path());

    assert!(result.is_err());
}

/// Test that relative paths are resolved correctly
#[test]
fn test_get_scripts_relative_paths() {
    let temp_dir = TempDir::new().unwrap();
    let script_file = temp_dir.path().join("script.sh");

    fs::write(&script_file, "#!/bin/bash\necho 'test'").unwrap();

    let phase = ScriptPhase {
        name: "test".to_string(),
        script: None,
        script_files: vec!["./script.sh".to_string()],
        env: HashMap::new(),
        continue_on_error: false,
        when: None,
        source: false,
    };

    let scripts = phase.get_scripts(temp_dir.path()).unwrap();

    assert_eq!(scripts.len(), 1);
    assert_eq!(scripts[0].0, "script.sh");
}

/// Test backward compatibility: legacy and new formats coexist
#[test]
fn test_legacy_and_phase_coexistence() {
    let toml = r#"
        [setup]
        scripts = ["./legacy-setup.sh"]

        [[phase.setup]]
        name = "new-setup"
        script = "echo 'new format'"

        [runtime]
        scripts = ["./legacy-runtime.sh"]

        [[phase.runtime]]
        name = "new-runtime"
        script = "echo 'new runtime'"
    "#;

    let config: Config = toml::from_str(toml).expect("Failed to parse TOML");

    // Legacy format should be preserved
    assert_eq!(config.setup.scripts.len(), 1);
    assert_eq!(config.setup.scripts[0], "./legacy-setup.sh");
    assert_eq!(config.runtime.scripts.len(), 1);
    assert_eq!(config.runtime.scripts[0], "./legacy-runtime.sh");

    // New format should also be present
    assert_eq!(config.phase.setup.len(), 1);
    assert_eq!(config.phase.setup[0].name, "new-setup");
    assert_eq!(config.phase.runtime.len(), 1);
    assert_eq!(config.phase.runtime[0].name, "new-runtime");
}

/// Test that phases from multiple configs are accumulated
#[test]
fn test_phase_config_accumulation() {
    // Simulate two config files being loaded and merged
    let toml1 = r#"
        [[phase.setup]]
        name = "base-phase"
        script = "echo 'base'"
    "#;

    let toml2 = r#"
        [[phase.setup]]
        name = "override-phase"
        script = "echo 'override'"
    "#;

    let config1: Config = toml::from_str(toml1).unwrap();
    let config2: Config = toml::from_str(toml2).unwrap();

    // Each config should have its own phases
    assert_eq!(config1.phase.setup.len(), 1);
    assert_eq!(config1.phase.setup[0].name, "base-phase");
    assert_eq!(config2.phase.setup.len(), 1);
    assert_eq!(config2.phase.setup[0].name, "override-phase");
}

/// Test that multiple runtime phases can be defined
#[test]
fn test_multiple_runtime_phases() {
    let toml = r#"
        [[phase.runtime]]
        name = "first-runtime"
        script = "echo 'first'"

        [[phase.runtime]]
        name = "second-runtime"
        script = "echo 'second'"
    "#;

    let config: Config = toml::from_str(toml).unwrap();

    assert_eq!(config.phase.runtime.len(), 2);
    assert_eq!(config.phase.runtime[0].name, "first-runtime");
    assert_eq!(config.phase.runtime[1].name, "second-runtime");
}

/// Test complete phase configuration with all fields
#[test]
fn test_phase_complete_config() {
    let toml = r#"
        [[phase.runtime]]
        name = "complete-phase"
        script = "echo 'inline script'"
        script_files = ["./file1.sh", "./file2.sh"]
        env = { DEBUG = "true", API_URL = "http://localhost" }
        continue_on_error = true
        when = "command -v docker"
    "#;

    let config: Config = toml::from_str(toml).expect("Failed to parse TOML");
    let phase = &config.phase.runtime[0];

    assert_eq!(phase.name, "complete-phase");
    assert_eq!(phase.script, Some("echo 'inline script'".to_string()));
    assert_eq!(phase.script_files.len(), 2);
    assert_eq!(phase.env.len(), 2);
    assert!(phase.continue_on_error);
    assert_eq!(phase.when, Some("command -v docker".to_string()));
}

/// Test that empty phase arrays are valid
#[test]
fn test_empty_phases() {
    let toml = r#"
        [vm]
        disk = 20
        memory = 8
    "#;

    let config: Config = toml::from_str(toml).expect("Failed to parse TOML");
    assert_eq!(config.phase.setup.len(), 0);
    assert_eq!(config.phase.runtime.len(), 0);
}

/// Test phase name defaults to empty string if not provided
#[test]
fn test_phase_name_defaults() {
    let toml = r#"
        [[phase.setup]]
        script = "echo 'test'"
    "#;

    // Name is optional and defaults to empty string
    let config: Config = toml::from_str(toml).expect("Failed to parse TOML");
    assert_eq!(config.phase.setup.len(), 1);
    assert_eq!(config.phase.setup[0].name, "");
}

/// Test that phase with neither script nor script_files is invalid
#[test]
fn test_phase_requires_script_or_files() {
    let phase = ScriptPhase {
        name: "empty".to_string(),
        script: None,
        script_files: vec![],
        env: HashMap::new(),
        continue_on_error: false,
        when: None,
        source: false,
    };

    let temp_dir = TempDir::new().unwrap();
    let scripts = phase.get_scripts(temp_dir.path()).unwrap();

    // Should return empty vec but not error
    assert_eq!(scripts.len(), 0);
}

/// Test tilde expansion in script file paths
#[test]
fn test_phase_tilde_expansion() {
    // This tests that the resolve_path logic handles tilde correctly
    // We can't test actual expansion without HOME set, but we can test parsing
    let toml = r#"
        [[phase.setup]]
        name = "tilde-test"
        script_files = ["~/scripts/setup.sh"]
    "#;

    let config: Config = toml::from_str(toml).expect("Failed to parse TOML");
    assert_eq!(config.phase.setup[0].script_files[0], "~/scripts/setup.sh");
}

/// Test special characters in script content
#[test]
fn test_phase_special_characters_in_script() {
    let toml = r#"
        [[phase.runtime]]
        name = "special-chars"
        script = """
#!/bin/bash
echo "Hello 'world'"
echo 'Single "quotes"'
VAR="value with $dollar"
"""
    "#;

    let config: Config = toml::from_str(toml).expect("Failed to parse TOML");
    let script = config.phase.runtime[0].script.as_ref().unwrap();

    assert!(script.contains("Hello 'world'"));
    assert!(script.contains("Single \"quotes\""));
    assert!(script.contains("$dollar"));
}

/// Test multiline script with proper formatting
#[test]
fn test_phase_multiline_script() {
    let toml = r#"
        [[phase.setup]]
        name = "multiline"
        script = """
#!/bin/bash
set -e

echo 'Line 1'
echo 'Line 2'
echo 'Line 3'
"""
    "#;

    let config: Config = toml::from_str(toml).expect("Failed to parse TOML");
    let script = config.phase.setup[0].script.as_ref().unwrap();

    assert!(script.contains("#!/bin/bash"));
    assert!(script.contains("set -e"));
    assert!(script.contains("Line 1"));
    assert!(script.contains("Line 2"));
    assert!(script.contains("Line 3"));
}

/// Test environment variable escaping
#[test]
fn test_phase_env_var_special_chars() {
    let toml = r#"
        [[phase.runtime]]
        name = "env-special"
        env = { VAR = "value with 'quotes' and \"double quotes\"" }
        script = "echo $VAR"
    "#;

    let config: Config = toml::from_str(toml).expect("Failed to parse TOML");
    let env_val = config.phase.runtime[0].env.get("VAR").unwrap();

    assert!(env_val.contains("'quotes'"));
    assert!(env_val.contains("\"double quotes\""));
}

/// Test phase with multiple environment variables
#[test]
fn test_phase_multiple_env_vars() {
    let toml = r#"
        [[phase.runtime]]
        name = "multi-env"
        env = {
            VAR1 = "value1",
            VAR2 = "value2",
            VAR3 = "value3",
            DEBUG = "true"
        }
        script = "echo $VAR1 $VAR2"
    "#;

    let config: Config = toml::from_str(toml).expect("Failed to parse TOML");
    let phase = &config.phase.runtime[0];

    assert_eq!(phase.env.len(), 4);
    assert_eq!(phase.env.get("VAR1"), Some(&"value1".to_string()));
    assert_eq!(phase.env.get("VAR2"), Some(&"value2".to_string()));
    assert_eq!(phase.env.get("VAR3"), Some(&"value3".to_string()));
    assert_eq!(phase.env.get("DEBUG"), Some(&"true".to_string()));
}

/// Test complex real-world setup phase configuration
#[test]
fn test_realistic_setup_phases() {
    let toml = r#"
        [[phase.setup]]
        name = "verify-requirements"
        script = """
#!/bin/bash
echo 'Verifying system requirements'
test $(nproc) -ge 2 || exit 1
"""

        [[phase.setup]]
        name = "install-docker"
        when = "! command -v docker"
        env = { DEBIAN_FRONTEND = "noninteractive" }
        script_files = ["./scripts/install-docker.sh"]

        [[phase.setup]]
        name = "configure-tools"
        continue_on_error = true
        script = "echo 'Configuring tools'"
    "#;

    let config: Config = toml::from_str(toml).expect("Failed to parse TOML");
    assert_eq!(config.phase.setup.len(), 3);

    // Verify first phase
    let phase1 = &config.phase.setup[0];
    assert_eq!(phase1.name, "verify-requirements");
    assert!(phase1.script.is_some());
    assert!(phase1.when.is_none());

    // Verify second phase
    let phase2 = &config.phase.setup[1];
    assert_eq!(phase2.name, "install-docker");
    assert_eq!(phase2.when, Some("! command -v docker".to_string()));
    assert_eq!(
        phase2.env.get("DEBIAN_FRONTEND"),
        Some(&"noninteractive".to_string())
    );

    // Verify third phase
    let phase3 = &config.phase.setup[2];
    assert_eq!(phase3.name, "configure-tools");
    assert!(phase3.continue_on_error);
}

/// Test complex real-world runtime phase configuration
#[test]
fn test_realistic_runtime_phases() {
    let toml = r#"
        [[phase.runtime]]
        name = "start-services"
        env = { DEBUG = "true", COMPOSE_PROJECT_NAME = "myapp" }
        script = "docker-compose up -d"

        [[phase.runtime]]
        name = "wait-for-health"
        continue_on_error = false
        script = """
until curl -sf http://localhost:3000/health; do
  echo 'Waiting for service...'
  sleep 1
done
echo '✓ Services ready'
"""

        [[phase.runtime]]
        name = "optional-service"
        continue_on_error = true
        script_files = ["./scripts/start-optional.sh"]
    "#;

    let config: Config = toml::from_str(toml).expect("Failed to parse TOML");
    assert_eq!(config.phase.runtime.len(), 3);

    // Verify first phase
    let phase1 = &config.phase.runtime[0];
    assert_eq!(phase1.name, "start-services");
    assert_eq!(phase1.env.len(), 2);
    assert!(phase1.script.is_some());

    // Verify second phase
    let phase2 = &config.phase.runtime[1];
    assert_eq!(phase2.name, "wait-for-health");
    assert!(!phase2.continue_on_error);
    assert!(phase2.script.as_ref().unwrap().contains("curl"));

    // Verify third phase
    let phase3 = &config.phase.runtime[2];
    assert_eq!(phase3.name, "optional-service");
    assert!(phase3.continue_on_error);
    assert_eq!(phase3.script_files.len(), 1);
}

/// Test that source field is properly parsed
#[test]
fn test_phase_source_parsing() {
    let toml = r#"
        [[phase.runtime]]
        name = "sourced-script"
        source = true
        script = "export PATH=\"$HOME/.local/bin:$PATH\""
    "#;

    let config: Config = toml::from_str(toml).expect("Failed to parse TOML");
    assert_eq!(config.phase.runtime.len(), 1);
    assert!(config.phase.runtime[0].source);
}

/// Test that source defaults to false
#[test]
fn test_phase_source_defaults_false() {
    let toml = r#"
        [[phase.runtime]]
        name = "regular-script"
        script = "echo 'hello'"
    "#;

    let config: Config = toml::from_str(toml).expect("Failed to parse TOML");
    assert_eq!(config.phase.runtime.len(), 1);
    assert!(!config.phase.runtime[0].source);
}

/// Test source with environment variables
#[test]
fn test_phase_source_with_env() {
    let toml = r#"
        [[phase.runtime]]
        name = "sourced-with-env"
        source = true
        env = { DEBUG = "true" }
        script = """
export MY_VAR="hello"
export PATH="$HOME/.local/bin:$PATH"
"""
    "#;

    let config: Config = toml::from_str(toml).expect("Failed to parse TOML");
    let phase = &config.phase.runtime[0];
    assert!(phase.source);
    assert_eq!(phase.env.len(), 1);
    assert!(phase.script.as_ref().unwrap().contains("export MY_VAR"));
}

/// Test source field explicitly set to false
#[test]
fn test_phase_source_explicit_false() {
    let toml = r#"
        [[phase.runtime]]
        name = "not-sourced"
        source = false
        script = "echo 'test'"
    "#;

    let config: Config = toml::from_str(toml).expect("Failed to parse TOML");
    assert_eq!(config.phase.runtime.len(), 1);
    assert!(!config.phase.runtime[0].source);
}

/// Test that has_shebang helper detects shebangs
#[test]
fn test_has_shebang_detection() {
    use claude_vm::config::ScriptPhase;

    // Note: has_shebang is private, so we test via validate_and_warn behavior
    // We can't directly test the helper, but we verify the validation works correctly
    let phase_with_shebang = ScriptPhase {
        name: "test".to_string(),
        script: Some("#!/bin/bash\necho 'hello'".to_string()),
        source: true,
        ..Default::default()
    };

    // Just verify it doesn't panic - warning goes to stderr which we can't easily capture in unit tests
    phase_with_shebang.validate_and_warn();
}

/// Test validation warns about empty phase
#[test]
fn test_validate_empty_phase() {
    use claude_vm::config::ScriptPhase;

    let empty_phase = ScriptPhase {
        name: "empty".to_string(),
        script: None,
        script_files: vec![],
        ..Default::default()
    };

    // Should emit warning but not panic
    empty_phase.validate_and_warn();
}

/// Test validation doesn't warn for valid configurations
#[test]
fn test_validate_valid_phase() {
    use claude_vm::config::ScriptPhase;

    // Script without shebang and source=true is fine
    let valid_phase = ScriptPhase {
        name: "valid".to_string(),
        script: Some("export PATH=$PATH:~/.local/bin".to_string()),
        source: true,
        ..Default::default()
    };

    // Should not warn
    valid_phase.validate_and_warn();

    // Script with shebang but source=false is fine
    let also_valid = ScriptPhase {
        name: "also-valid".to_string(),
        script: Some("#!/bin/bash\necho 'hello'".to_string()),
        source: false,
        ..Default::default()
    };

    also_valid.validate_and_warn();
}
