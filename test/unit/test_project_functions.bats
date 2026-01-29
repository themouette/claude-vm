#!/usr/bin/env bats

# Tests for get-project-id and get-template-name functions

setup() {
  # Load test helpers
  load '../test_helper'

  # Create a temporary directory for testing
  TEST_DIR=$(mktemp -d)
  ORIGINAL_PWD=$(pwd)

  # Source the functions from claude-vm
  # We need to extract just the functions without running main()
  eval "$(sed -n '/^get-project-id()/,/^}/p' "$PROJECT_ROOT/claude-vm")"
  eval "$(sed -n '/^get-template-name()/,/^}/p' "$PROJECT_ROOT/claude-vm")"
}

teardown() {
  # Clean up test directory
  cd "$ORIGINAL_PWD"
  rm -rf "$TEST_DIR"
}

@test "get-project-id returns git root when in git repo" {
  cd "$TEST_DIR"
  git init -q
  mkdir -p subdir
  cd subdir

  result=$(get-project-id)

  # Compare resolved paths using realpath or readlink -f (in case of symlinks like /var -> /private/var on macOS)
  local expected=$(cd "$TEST_DIR" && pwd -P)
  local actual=$(cd "$result" && pwd -P)
  [ "$actual" = "$expected" ]
}

@test "get-project-id returns pwd when not in git repo" {
  cd "$TEST_DIR"

  result=$(get-project-id)

  [ "$result" = "$TEST_DIR" ]
}

@test "get-template-name generates correct format" {
  # Override get-project-id to return a known path
  get-project-id() { echo "/path/to/my-project"; }

  result=$(get-template-name)

  # Should match: claude-tpl--my-project--<8char-hash>
  [[ "$result" =~ ^claude-tpl--my-project--[a-f0-9]{8}$ ]]
}

@test "get-template-name sanitizes uppercase to lowercase" {
  get-project-id() { echo "/path/to/MyProject"; }

  result=$(get-template-name)

  # Should be lowercase
  [[ "$result" =~ ^claude-tpl--myproject--[a-f0-9]{8}$ ]]
}

@test "get-template-name sanitizes special characters" {
  get-project-id() { echo "/path/to/my_project!@#"; }

  result=$(get-template-name)

  # Should replace special chars with dashes (consecutive special chars become multiple dashes)
  # The regex should allow for one or more dashes between valid characters
  [[ "$result" =~ ^claude-tpl--my-project[a-f0-9-]+--[a-f0-9]{8}$ ]]
}

@test "get-template-name removes leading dashes" {
  get-project-id() { echo "/path/to/_project"; }

  result=$(get-template-name)

  # Should not start with dash after tpl--
  [[ "$result" =~ ^claude-tpl--project--[a-f0-9]{8}$ ]]
}

@test "get-template-name removes trailing dashes" {
  get-project-id() { echo "/path/to/project_"; }

  result=$(get-template-name)

  # Should not have dash before hash
  [[ "$result" =~ ^claude-tpl--project--[a-f0-9]{8}$ ]]
}

@test "get-template-name generates same hash for same path" {
  get-project-id() { echo "/path/to/my-project"; }

  result1=$(get-template-name)
  result2=$(get-template-name)

  [ "$result1" = "$result2" ]
}

@test "get-template-name generates different hash for different paths" {
  get-project-id() { echo "/path/to/project1"; }
  result1=$(get-template-name)

  get-project-id() { echo "/path/to/project2"; }
  result2=$(get-template-name)

  [ "$result1" != "$result2" ]
}
