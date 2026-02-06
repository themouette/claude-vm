# Runtime Scripts

Runtime scripts automatically execute before Claude runs or before opening a shell, allowing you to set up services, configure environments, and provide dynamic context to Claude.

## Table of Contents

- [What are Runtime Scripts?](#what-are-runtime-scripts)
- [Quick Start](#quick-start)
- [Script Discovery](#script-discovery)
- [Execution Order](#execution-order)
- [Features](#features)
- [Contributing Context to Claude](#contributing-context-to-claude)
- [Examples](#examples)
- [Debugging](#debugging)

## What are Runtime Scripts?

Runtime scripts are bash scripts that run automatically before each Claude session or shell command. They allow you to:

- Start background services (databases, APIs, etc.)
- Set environment variables
- Initialize development environments
- Provide dynamic context to Claude
- Run health checks
- Seed databases
- Configure session-specific settings

Unlike setup scripts (which run once during template creation), runtime scripts run every time you start a session.

## Quick Start

Create `.claude-vm.runtime.sh` in your project root:

```bash
#!/bin/bash
# .claude-vm.runtime.sh

# Start services
echo "Starting services..."
docker-compose up -d

# Wait for services
sleep 2

# Set environment
export API_KEY="dev-key"
export DEBUG=true

echo "✓ Environment ready"
```

Now every time you run `claude-vm` or `claude-vm shell`, this script executes first.

## Script Discovery

### Auto-Detected Scripts

**No configuration needed** - Claude VM automatically detects and runs:

```bash
./.claude-vm.runtime.sh    # Project runtime script
```

**Script location:**
- In a git worktree: searched at the worktree top directory
- In a git repository: searched at the repository root
- Outside git: searched in current directory

### Configuration-Based Scripts

Add additional scripts in `.claude-vm.toml`:

```toml
[runtime]
scripts = [
    "./.claude-vm.runtime.sh",    # Auto-detected (optional to list)
    "./scripts/start-services.sh", # Additional scripts
    "~/scripts/dev-setup.sh",      # Global scripts
]
```

### Command-Line Scripts

Pass scripts via CLI:

```bash
# Single script
claude-vm --runtime-script ./start-db.sh shell

# Multiple scripts
claude-vm --runtime-script ./setup.sh --runtime-script ./seed.sh shell
```

## Execution Order

Scripts run in this order:

1. **Project runtime script** - `./.claude-vm.runtime.sh` (if exists)
2. **Config runtime scripts** - From `[runtime] scripts` in `.claude-vm.toml`
3. **CLI runtime scripts** - From `--runtime-script` flags

All scripts and the main command run in a single shell invocation, sharing environment and processes.

## Features

### Shared Environment

All scripts and the main command share the same shell environment:

```bash
# script1.sh
export API_KEY="secret"

# script2.sh
echo "API_KEY is: $API_KEY"  # Can access variable from script1

# Main command also has access
claude-vm --runtime-script script1.sh --runtime-script script2.sh shell
$ echo $API_KEY  # "secret"
```

### Fail-Fast Behavior

If any script fails (exit code ≠ 0), execution stops:

```bash
#!/bin/bash
# .claude-vm.runtime.sh

docker-compose up -d || exit 1  # Stop if docker fails
npm run migrate || exit 1       # Stop if migration fails

echo "✓ Ready"  # Only reached if above succeeds
```

The main command (Claude or shell) won't run if a runtime script fails.

### Background Processes

Background processes started in runtime scripts continue running:

```bash
#!/bin/bash
# .claude-vm.runtime.sh

# Start services in background
docker-compose up -d

# Start development server (in background)
npm run dev &

# Continue with other setup
echo "Services started"
```

Services remain running for the entire session.

### Interactive Support

Runtime scripts can prompt for input:

```bash
#!/bin/bash
# .claude-vm.runtime.sh

if [ -z "$API_KEY" ]; then
  read -p "Enter API key: " API_KEY
  export API_KEY
fi

read -p "Enable debug mode? (y/n): " enable_debug
if [ "$enable_debug" = "y" ]; then
  export DEBUG=true
fi
```

Full terminal support including colors and cursor control.

### Security

- Script paths are properly escaped
- Filenames are sanitized
- Unicode filenames supported
- No shell injection vulnerabilities

## Contributing Context to Claude

Runtime scripts can write dynamic context that Claude receives via `~/.claude/CLAUDE.md`.

### How It Works

1. Runtime script writes to `~/.claude-vm/context/<name>.txt`
2. Content is automatically merged into `~/.claude/CLAUDE.md`
3. Claude receives the context at session start

### Basic Example

```bash
#!/bin/bash
# .claude-vm.runtime.sh

# Create context directory
mkdir -p ~/.claude-vm/context

# Write context
cat > ~/.claude-vm/context/services.txt <<EOF
Development services running:
- PostgreSQL: localhost:5432
- Redis: localhost:6379
- API: http://localhost:3000
EOF
```

Claude will see this in its context as:

```markdown
## Runtime Script Results

### services

Development services running:
- PostgreSQL: localhost:5432
- Redis: localhost:6379
- API: http://localhost:3000
```

### Context File Naming

- **Filename**: `~/.claude-vm/context/<name>.txt`
- **Section heading**: `<name>` (basename without .txt)
- **Multiple files**: Each file becomes a separate section
- **Ordering**: Files are included in alphabetical order

### Service Status Example

```bash
#!/bin/bash
# .claude-vm.runtime.sh

# Start services
docker-compose up -d

# Wait for readiness
until curl -sf http://localhost:3000/health > /dev/null; do
  sleep 1
done

# Write context
mkdir -p ~/.claude-vm/context
cat > ~/.claude-vm/context/services.txt <<EOF
Services Status:
- API: http://localhost:3000 (healthy)
- Database: postgresql://localhost:5432/myapp_dev
  - Tables: users, posts, comments
  - Test data: seeded
- Cache: redis://localhost:6379

Commands:
- View logs: docker-compose logs -f
- Reset DB: npm run db:reset
- API docs: http://localhost:3000/docs
EOF
```

### Environment Detection Example

```bash
#!/bin/bash
# .claude-vm.runtime.sh

# Detect project type
PROJECT_TYPE="unknown"
if [ -f "package.json" ]; then
  PROJECT_TYPE="Node.js $(node --version)"
elif [ -f "Cargo.toml" ]; then
  PROJECT_TYPE="Rust $(rustc --version | cut -d' ' -f2)"
elif [ -f "requirements.txt" ]; then
  PROJECT_TYPE="Python $(python3 --version | cut -d' ' -f2)"
fi

# Detect git info
GIT_BRANCH=$(git branch --show-current 2>/dev/null || echo "none")
GIT_STATUS=$(git status --short 2>/dev/null | wc -l | tr -d ' ')

# Write context
mkdir -p ~/.claude-vm/context
cat > ~/.claude-vm/context/environment.txt <<EOF
Environment:
- Project: $PROJECT_TYPE
- Git branch: $GIT_BRANCH
- Uncommitted changes: $GIT_STATUS files
- Working directory: $(pwd)

Available tools:
- Docker: $(docker --version 2>/dev/null || echo "not installed")
- Make: $(make --version 2>/dev/null | head -n1 || echo "not installed")
EOF
```

### Multiple Context Files

Different scripts can contribute different context files:

```bash
# .claude-vm.runtime.sh
cat > ~/.claude-vm/context/database.txt <<EOF
Database: PostgreSQL 15
Connection: localhost:5432/myapp
Status: healthy
EOF

# ./scripts/check-api.sh
cat > ~/.claude-vm/context/api.txt <<EOF
API: http://localhost:3000
Health: OK
Version: 1.2.3
EOF
```

Both appear in Claude's context as separate sections.

## Examples

### Database Setup

```bash
#!/bin/bash
# .claude-vm.runtime.sh

echo "Setting up database..."

# Start PostgreSQL
docker-compose up -d postgres

# Wait for readiness
until pg_isready -h localhost -p 5432 -U postgres; do
  echo "Waiting for database..."
  sleep 1
done

# Run migrations
npm run db:migrate

# Seed if empty
if [ "$(psql -h localhost -U postgres -d myapp -tAc "SELECT COUNT(*) FROM users")" -eq 0 ]; then
  echo "Seeding database..."
  npm run db:seed
fi

echo "✓ Database ready"
```

### Service Orchestration

```bash
#!/bin/bash
# .claude-vm.runtime.sh

echo "Starting services..."

# Start all services
docker-compose up -d

# Wait for each service
services=("postgres:5432" "redis:6379" "api:3000")
for service in "${services[@]}"; do
  IFS=: read -r name port <<< "$service"
  echo "Waiting for $name..."

  until nc -z localhost "$port" 2>/dev/null; do
    sleep 1
  done

  echo "✓ $name ready"
done

# Run health checks
curl -sf http://localhost:3000/health || {
  echo "API health check failed"
  exit 1
}

echo "✓ All services ready"
```

### Environment Configuration

```bash
#!/bin/bash
# .claude-vm.runtime.sh

# Load .env file if exists
if [ -f .env ]; then
  export $(grep -v '^#' .env | xargs)
fi

# Prompt for missing critical vars
if [ -z "$API_KEY" ]; then
  read -p "Enter API key: " API_KEY
  export API_KEY
fi

# Set development defaults
export NODE_ENV="${NODE_ENV:-development}"
export DEBUG="${DEBUG:-true}"
export LOG_LEVEL="${LOG_LEVEL:-debug}"

# Display configuration
echo "Environment:"
echo "  NODE_ENV: $NODE_ENV"
echo "  DEBUG: $DEBUG"
echo "  API_KEY: ${API_KEY:0:10}..."
```

### Conditional Setup

```bash
#!/bin/bash
# .claude-vm.runtime.sh

# Only start services if not already running
if ! docker-compose ps | grep -q "Up"; then
  echo "Starting services..."
  docker-compose up -d
else
  echo "Services already running"
fi

# Only run migrations if needed
PENDING=$(npm run db:migrate:status | grep -c "pending")
if [ "$PENDING" -gt 0 ]; then
  echo "Running $PENDING pending migrations..."
  npm run db:migrate
else
  echo "Database up to date"
fi
```

## Debugging

### Verbose Mode

See detailed script execution:

```bash
claude-vm --verbose shell
```

**Output includes:**
- Script copying progress (✓/✗)
- Lima VM startup logs
- Script execution output
- Error messages with context

### Script Errors

When a script fails, you'll see:

```
Error: Runtime script failed: .claude-vm.runtime.sh
Exit code: 1
Output:
  Starting services...
  Error: docker-compose not found
```

### Testing Scripts

Test scripts independently:

```bash
# Copy to VM and run manually
limactl shell <template-name> bash -c "$(cat .claude-vm.runtime.sh)"

# Or test locally (may not have all dependencies)
bash .claude-vm.runtime.sh
```

### Common Issues

**Script not found:**
- Ensure script exists and path is correct
- Check script is executable: `chmod +x .claude-vm.runtime.sh`

**Environment variables not set:**
- Variables only persist within the session
- Use `export` to make variables available to subsequent scripts

**Services not starting:**
- Check docker/docker-compose is installed in template
- Verify services are defined in docker-compose.yml
- Check for port conflicts

## Best Practices

### 1. Make Scripts Idempotent

Scripts should be safe to run multiple times:

```bash
# Good: Check before starting
if ! docker-compose ps | grep -q "Up"; then
  docker-compose up -d
fi

# Bad: Always starts (may fail if already running)
docker-compose up -d
```

### 2. Fail Fast

Exit early on errors:

```bash
set -e  # Exit on any error

docker-compose up -d
npm run migrate
npm run seed
```

### 3. Provide Feedback

Show progress to the user:

```bash
echo "Starting services..."
docker-compose up -d
echo "✓ Services started"

echo "Running migrations..."
npm run db:migrate
echo "✓ Migrations complete"
```

### 4. Keep Scripts Fast

Runtime scripts run on every session - keep them quick:

```bash
# Good: Check if work is needed
if [ ! -f ".initialized" ]; then
  npm run init
  touch .initialized
fi

# Bad: Always runs expensive operation
npm run init
```

### 5. Use Context Wisely

Provide useful, actionable information:

```bash
# Good: Specific, actionable context
cat > ~/.claude-vm/context/services.txt <<EOF
API: http://localhost:3000
Test user: admin@example.com / password123
Docs: http://localhost:3000/api-docs
EOF

# Less useful: Vague information
cat > ~/.claude-vm/context/services.txt <<EOF
Everything is running.
EOF
```

## Next Steps

- **[Templates](templates.md)** - Understand template VMs
- **[Custom Mounts](custom-mounts.md)** - Mount additional directories
- **[Configuration](../configuration.md)** - Configure runtime scripts
- **[Troubleshooting](../advanced/troubleshooting.md)** - Debug script issues
