# Tests

This directory contains the test suite for `claude-vm`.

## Prerequisites

Install [bats-core](https://github.com/bats-core/bats-core):

```bash
# macOS
brew install bats-core

# Ubuntu/Debian
sudo apt-get install bats

# Manual installation
git clone https://github.com/bats-core/bats-core.git
cd bats-core
sudo ./install.sh /usr/local
```

## Running Tests

### Unit Tests (Fast)

Unit tests mock external dependencies and don't create real VMs:

```bash
# Run all unit tests
bats test/unit/

# Run specific test file
bats test/unit/test_project_functions.bats

# Verbose output
bats -t test/unit/
```

### Integration Tests (Slow)

Integration tests create real VMs and require Lima:

```bash
# Run integration tests (requires Lima)
INTEGRATION=1 bats test/integration/
```

**Note**: Integration tests are skipped by default and only run when `INTEGRATION=1` is set.

### All Tests

```bash
# Run all tests (unit only, integration skipped)
bats test/

# Run everything including integration tests
INTEGRATION=1 bats test/
```

## Test Structure

```
test/
├── test_helper.bash           # Test utilities and mocks
├── unit/                      # Fast unit tests (no VMs)
│   ├── test_project_functions.bats
│   └── test_argument_parsing.bats
├── integration/               # Slow integration tests (real VMs)
│   └── test_template_lifecycle.bats
└── fixtures/                  # Test fixtures and sample files
    └── sample-setup.sh
```

## Continuous Integration

Tests run automatically on GitHub Actions:

- **Shellcheck**: Lints the bash script for common issues
- **Unit Tests**: Run on every push and PR
- **Integration Tests**: Only run on pushes to main branch (optional)

See `.github/workflows/test.yml` for CI configuration.

## Writing Tests

### Unit Test Template

```bash
#!/usr/bin/env bats

setup() {
  load '../test_helper'
  # Setup code here
}

teardown() {
  # Cleanup code here
}

@test "describe what this tests" {
  run your-command

  [ "$status" -eq 0 ]
  [[ "$output" =~ "expected output" ]]
}
```

### Useful Assertions

```bash
[ "$status" -eq 0 ]              # Command succeeded
[ "$status" -eq 1 ]              # Command failed
[ "$output" = "exact match" ]    # Exact output match
[[ "$output" =~ "pattern" ]]     # Regex match
[ -f "/path/to/file" ]           # File exists
[ -d "/path/to/dir" ]            # Directory exists
```

## Debugging Tests

Run tests with verbose output:

```bash
bats -t test/unit/              # Tap format
bats --print-output-on-failure test/unit/
```

Run a single test:

```bash
bats test/unit/test_project_functions.bats -f "get-project-id returns git root"
```

## Mocking

The `test_helper.bash` file provides mocks for external commands:

- `limactl`: Mocked by default (`LIMACTL_MOCK=1`)
- Use `LIMACTL_MOCK=0` to use real limactl in tests

## Coverage

Current test coverage focuses on:

- ✅ Project ID detection (git root vs pwd)
- ✅ Template name generation and sanitization
- ✅ Command-line argument parsing
- ✅ Script file validation
- ⏸️ VM lifecycle (integration tests, manual)

## Contributing

When adding new features:

1. Write unit tests for new functions
2. Update argument parsing tests for new flags
3. Add integration tests for VM operations (optional)
4. Run tests locally before submitting PR
