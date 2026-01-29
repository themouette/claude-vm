#!/usr/bin/env bats

# Tests for command-line argument parsing

setup() {
  load '../test_helper'

  # Mock limactl and other external commands
  export LIMACTL_MOCK=1
}

@test "setup shows help with --help flag" {
  run "$PROJECT_ROOT/claude-vm" setup --help

  [ "$status" -eq 0 ]
  [[ "$output" =~ "Usage: claude-vm setup" ]]
  [[ "$output" =~ "--docker" ]]
  [[ "$output" =~ "--node" ]]
  [[ "$output" =~ "--python" ]]
  [[ "$output" =~ "--chromium" ]]
}

@test "main command shows help with --help flag" {
  run "$PROJECT_ROOT/claude-vm" --help

  [ "$status" -eq 0 ]
  [[ "$output" =~ "Usage: claude-vm" ]]
  [[ "$output" =~ "Commands:" ]]
  [[ "$output" =~ "setup" ]]
  [[ "$output" =~ "shell" ]]
  [[ "$output" =~ "list" ]]
  [[ "$output" =~ "clean" ]]
}

@test "setup rejects unknown flag" {
  run "$PROJECT_ROOT/claude-vm" setup --unknown-flag

  [ "$status" -eq 1 ]
  [[ "$output" =~ "Unknown option" ]]
}

@test "setup accepts --docker flag" {
  # This test just verifies parsing, not execution
  # We need to mock the entire setup to avoid actually creating VMs
  run bash -c "cd '$PROJECT_ROOT' && ./claude-vm setup --docker --help"

  [ "$status" -eq 0 ]
}

@test "setup accepts --node flag" {
  run bash -c "cd '$PROJECT_ROOT' && ./claude-vm setup --node --help"

  [ "$status" -eq 0 ]
}

@test "setup accepts --python flag" {
  run bash -c "cd '$PROJECT_ROOT' && ./claude-vm setup --python --help"

  [ "$status" -eq 0 ]
}

@test "setup accepts --chromium flag" {
  run bash -c "cd '$PROJECT_ROOT' && ./claude-vm setup --chromium --help"

  [ "$status" -eq 0 ]
}

@test "setup accepts --all flag" {
  run bash -c "cd '$PROJECT_ROOT' && ./claude-vm setup --all --help"

  [ "$status" -eq 0 ]
}

@test "setup accepts --disk with value" {
  run bash -c "cd '$PROJECT_ROOT' && ./claude-vm setup --disk 15 --help"

  [ "$status" -eq 0 ]
}

@test "setup accepts --memory with value" {
  run bash -c "cd '$PROJECT_ROOT' && ./claude-vm setup --memory 4 --help"

  [ "$status" -eq 0 ]
}

@test "setup accepts --setup-script with existing file" {
  # Create a temporary script file
  local script_file=$(mktemp)
  echo "#!/bin/bash" > "$script_file"

  run bash -c "cd '$PROJECT_ROOT' && ./claude-vm setup --setup-script '$script_file' --help"

  [ "$status" -eq 0 ]

  rm "$script_file"
}

@test "setup rejects --setup-script with missing file" {
  run "$PROJECT_ROOT/claude-vm" setup --setup-script /nonexistent/file.sh

  [ "$status" -eq 1 ]
  [[ "$output" =~ "Setup script not found" ]]
}

@test "setup rejects --setup-script without argument" {
  run "$PROJECT_ROOT/claude-vm" setup --setup-script

  [ "$status" -eq 1 ]
  [[ "$output" =~ "requires a path argument" ]]
}

@test "setup accepts multiple --setup-script flags" {
  local script1=$(mktemp)
  local script2=$(mktemp)
  echo "#!/bin/bash" > "$script1"
  echo "#!/bin/bash" > "$script2"

  run bash -c "cd '$PROJECT_ROOT' && ./claude-vm setup --setup-script '$script1' --setup-script '$script2' --help"

  [ "$status" -eq 0 ]

  rm "$script1" "$script2"
}
