#!/bin/bash
# OpenCode deployment functions

deploy_context() {
    local context_file="$1"
    echo "Deploying context for OpenCode..."
    mkdir -p ~/.config/opencode
    mv "$context_file" ~/.config/opencode/AGENTS.md
}

deploy_mcp() {
    local mcp_file="$1"

    # Skip if no MCP config provided
    if [ ! -f "$mcp_file" ]; then
        echo "No MCP configuration to deploy"
        return 0
    fi

    echo "Deploying MCP config for OpenCode..."
    mkdir -p ~/.config/opencode

    # Merge into existing config or create new
    if [ -f ~/.config/opencode/opencode.json ]; then
        jq -s '.[0] * {mcpServers: .[1].mcpServers}' \
            ~/.config/opencode/opencode.json "$mcp_file" \
            > ~/.config/opencode/opencode.json.tmp
        mv ~/.config/opencode/opencode.json.tmp ~/.config/opencode/opencode.json
        rm "$mcp_file"
    else
        echo '{}' | jq --slurpfile mcp "$mcp_file" \
            '. + {mcpServers: $mcp[0].mcpServers}' \
            > ~/.config/opencode/opencode.json
        rm "$mcp_file"
    fi
}
