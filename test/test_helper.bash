#!/usr/bin/env bash

# Test helper utilities and mocks

# Get the project root directory
# BATS_TEST_DIRNAME is the directory containing the test file
# We need to go up to the test/ directory, then up to project root
if [[ "$BATS_TEST_DIRNAME" == */test/unit ]]; then
  PROJECT_ROOT="$BATS_TEST_DIRNAME/../.."
elif [[ "$BATS_TEST_DIRNAME" == */test/integration ]]; then
  PROJECT_ROOT="$BATS_TEST_DIRNAME/../.."
elif [[ "$BATS_TEST_DIRNAME" == */test ]]; then
  PROJECT_ROOT="$BATS_TEST_DIRNAME/.."
else
  PROJECT_ROOT="$BATS_TEST_DIRNAME"
fi

# Mock limactl for testing without creating real VMs
export LIMACTL_MOCK=${LIMACTL_MOCK:-1}

limactl() {
  if [ "$LIMACTL_MOCK" = "1" ]; then
    echo "MOCK: limactl $*" >&2

    case "$1" in
      list)
        if [ "$2" = "-q" ]; then
          # Return mock template names
          echo "claude-tpl_test-project_abc12345"
          echo "claude-tpl_another-proj_def67890"
        else
          # Return full list format
          echo "NAME                              STATUS   SSH"
          echo "claude-tpl_test-project_abc12345 Running  127.0.0.1:60022"
        fi
        ;;
      create|start|stop|delete|shell|clone)
        # Silently succeed for these operations
        return 0
        ;;
    esac
    return 0
  else
    # Use real limactl if not mocked
    command limactl "$@"
  fi
}

# Export so it's available to subshells
export -f limactl

# Helper to source the main script and extract functions
load_claude_vm() {
  # Source the script in a way that doesn't execute main()
  # We temporarily replace main() with a no-op
  (
    main() { :; }
    export -f main
    source "$PROJECT_ROOT/claude-vm"
  )
}
