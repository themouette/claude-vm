#!/bin/bash
# Claude deployment functions

deploy_context() {
    local context_file="$1"

    if [ ! -f "$context_file" ]; then
        echo "ERROR: Context file not found: $context_file" >&2
        return 1
    fi

    echo "Deploying context for Claude..."
    mkdir -p ~/.claude || {
        echo "ERROR: Failed to create Claude config directory" >&2
        return 1
    }

    if ! mv "$context_file" ~/.claude/CLAUDE.md; then
        echo "ERROR: Failed to deploy context file" >&2
        return 1
    fi
}

deploy_mcp() {
    local mcp_file="$1"

    # Skip if no MCP config provided
    if [ ! -f "$mcp_file" ]; then
        echo "No MCP configuration to deploy"
        return 0
    fi

    echo "Deploying MCP config for Claude..."
    mkdir -p ~/.claude || {
        echo "ERROR: Failed to create Claude config directory" >&2
        return 1
    }

    if ! mv "$mcp_file" ~/.claude.json; then
        echo "ERROR: Failed to deploy MCP config" >&2
        return 1
    fi
}
