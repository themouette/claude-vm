#!/bin/bash
# Claude deployment functions

deploy_context() {
    local context_file="$1"
    echo "Deploying context for Claude..."
    mkdir -p ~/.claude
    mv "$context_file" ~/.claude/CLAUDE.md
}

deploy_mcp() {
    local mcp_file="$1"

    # Skip if no MCP config provided
    if [ ! -f "$mcp_file" ]; then
        echo "No MCP configuration to deploy"
        return 0
    fi

    echo "Deploying MCP config for Claude..."
    mkdir -p ~/.claude
    mv "$mcp_file" ~/.claude.json
}
