#!/bin/bash
set -e
gh --version >&2

echo "" >&2
echo "Choose GitHub authentication method:" >&2
echo "  1) Personal Access Token (recommended - supports multiple VMs)" >&2
echo "  2) Device Flow (legacy - only one device at a time)" >&2
echo "" >&2
read -p "Enter choice [1-2]: " choice

case "$choice" in
    1)
        echo "" >&2
        echo "Setting up Personal Access Token authentication..." >&2
        echo "" >&2

        # Extract target_name from git remote
        if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
            owners=$(git remote -v 2>/dev/null | grep -oP '(?<=github.com[:/])[^/]+' | sort -u)
            owner_count=$(echo "$owners" | wc -l)
            if [ "$owner_count" -eq 1 ] && [ -n "$owners" ]; then
                target_name="$owners"
                target_param="&target_name=$target_name"
            else
                target_name=""
                target_param=""
            fi
        else
            target_name=""
            target_param=""
        fi

        # Use PROJECT_NAME from environment (automatically provided by claude-vm)
        project_name="$PROJECT_NAME"

        # Build the pre-configured URL
        token_url="https://github.com/settings/personal-access-tokens/new"
        token_url="${token_url}?name=Claude+VM+-+${project_name}+(${TEMPLATE_NAME})"
        token_url="${token_url}&description=Access+token+for+Claude+VM+project+${project_name}"
        token_url="${token_url}${target_param}"
        token_url="${token_url}&expires_in=90"
        token_url="${token_url}&contents=read"
        token_url="${token_url}&issues=write"
        token_url="${token_url}&pull_requests=write"
        token_url="${token_url}&workflows=write"
        token_url="${token_url}&metadata=read"

        echo "Option 1 - CREATE a new token with pre-configured permissions:" >&2
        echo "  $token_url" >&2
        echo "" >&2
        echo "Option 2 - USE or REGENERATE an existing token:" >&2
        echo "  https://github.com/settings/personal-access-tokens" >&2
        echo "" >&2
        echo "Required permissions:" >&2
        echo "  • contents: read (agent should NOT push without ssh-agent)" >&2
        echo "  • issues: write" >&2
        echo "  • pull_requests: write" >&2
        echo "  • workflows: write" >&2
        echo "  • metadata: read (required)" >&2
        echo "" >&2
        echo "After creating or regenerating your token, paste it below:" >&2

        # Read token with asterisk feedback
        echo -n "Token: " >&2
        token=""
        while IFS= read -r -s -n1 char; do
            # Enter key (empty character)
            if [[ -z $char ]]; then
                break
            fi
            # Backspace or Delete
            if [[ $char == $'\177' ]] || [[ $char == $'\b' ]]; then
                if [ ${#token} -gt 0 ]; then
                    token="${token%?}"
                    echo -ne "\b \b" >&2
                fi
            else
                token+="$char"
                echo -n "*" >&2
            fi
        done
        echo "" >&2

        if [ -z "$token" ]; then
            echo "✗ No token provided, skipping authentication" >&2
            exit 0
        fi

        # Authenticate with token
        if echo "$token" | gh auth login --with-token 2>&1; then
            echo "✓ Successfully authenticated with token" >&2

            # Store metadata (token itself is stored securely by gh, not here)
            mkdir -p ~/.claude-vm
            cat > ~/.claude-vm/gh-auth-info <<EOF
auth_method=token
token_name="Claude VM - ${project_name} (${TEMPLATE_NAME})"
token_created=$(date +%Y-%m-%d)
EOF
        else
            echo "✗ Failed to authenticate with token" >&2
            exit 1
        fi
        ;;

    2)
        echo "" >&2
        echo "Setting up Device Flow authentication..." >&2
        echo "" >&2

        # Use device flow
        if gh auth login --git-protocol https --hostname github.com 2>&1 <<EOF
EOF
        then
            echo "✓ Successfully authenticated with device flow" >&2

            # Store metadata (token itself is stored securely by gh, not here)
            mkdir -p ~/.claude-vm
            cat > ~/.claude-vm/gh-auth-info <<EOF
auth_method=device
token_name=""
token_created=$(date +%Y-%m-%d)
EOF
        else
            echo "✗ Failed to authenticate with device flow" >&2
            exit 1
        fi
        ;;

    *)
        echo "Invalid choice, skipping authentication" >&2
        exit 0
        ;;
esac

echo "" >&2
echo "✓ GitHub CLI authentication configured" >&2
