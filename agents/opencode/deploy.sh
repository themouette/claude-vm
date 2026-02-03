#!/bin/bash
# OpenCode deployment functions

deploy_context() {
    local context_file="$1"

    if [ ! -f "$context_file" ]; then
        echo "ERROR: Context file not found: $context_file" >&2
        return 1
    fi

    echo "Deploying context for OpenCode..."
    mkdir -p ~/.config/opencode || {
        echo "ERROR: Failed to create OpenCode config directory" >&2
        return 1
    }

    if ! mv "$context_file" ~/.config/opencode/AGENTS.md; then
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

    echo "Deploying MCP config for OpenCode..."
    mkdir -p ~/.config/opencode || {
        echo "ERROR: Failed to create OpenCode config directory" >&2
        return 1
    }

    # Merge into existing config or create new
    if [ -f ~/.config/opencode/opencode.json ]; then
        if ! jq -s '.[0] * {mcpServers: .[1].mcpServers}' \
            ~/.config/opencode/opencode.json "$mcp_file" \
            > ~/.config/opencode/opencode.json.tmp; then
            echo "ERROR: Failed to merge MCP configuration" >&2
            rm -f ~/.config/opencode/opencode.json.tmp
            return 1
        fi
        if ! mv ~/.config/opencode/opencode.json.tmp ~/.config/opencode/opencode.json; then
            echo "ERROR: Failed to update OpenCode config" >&2
            return 1
        fi
    else
        if ! echo '{}' | jq --slurpfile mcp "$mcp_file" \
            '. + {mcpServers: $mcp[0].mcpServers}' \
            > ~/.config/opencode/opencode.json; then
            echo "ERROR: Failed to create OpenCode MCP configuration" >&2
            rm -f ~/.config/opencode/opencode.json
            return 1
        fi
    fi

    rm -f "$mcp_file"
}
