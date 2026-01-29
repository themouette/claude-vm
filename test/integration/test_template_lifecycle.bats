#!/usr/bin/env bats

# Integration tests that create real VMs
# Run with: INTEGRATION=1 bats test/integration/

setup() {
  load '../test_helper'

  if [ "$INTEGRATION" != "1" ]; then
    skip "Integration tests disabled. Set INTEGRATION=1 to run"
  fi

  # Create a temporary test project
  export TEST_PROJECT=$(mktemp -d)
  cd "$TEST_PROJECT"
  git init -q

  # Get the template name for this test project
  eval "$(sed -n '/^get-project-id()/,/^}/p' "$PROJECT_ROOT/claude-vm")"
  eval "$(sed -n '/^get-template-name()/,/^}/p' "$PROJECT_ROOT/claude-vm")"
  export TEMPLATE_NAME=$(get-template-name)
}

teardown() {
  # Clean up test VMs
  limactl stop "$TEMPLATE_NAME" 2>/dev/null || true
  limactl delete "$TEMPLATE_NAME" --force 2>/dev/null || true

  rm -rf "$TEST_PROJECT"
}

@test "setup creates a template" {
  skip "Integration test - create real VM (slow)"

  # This would actually create a VM - only run manually
  # run "$PROJECT_ROOT/claude-vm" setup --disk 10 --memory 4
  # [ "$status" -eq 0 ]
  # run limactl list -q
  # [[ "$output" =~ "$TEMPLATE_NAME" ]]
}

@test "list shows created templates" {
  skip "Integration test - requires real VM"

  # Would test: claude-vm list
}

@test "clean removes template" {
  skip "Integration test - requires real VM"

  # Would test: claude-vm clean
}
