#!/bin/bash
# Write GitHub CLI context for Claude
mkdir -p ~/.claude-vm/context

# Check auth status
if gh auth status 2>/dev/null; then
    # Authenticated - show actual status
    username=$(gh api /user -q '.login' 2>/dev/null || echo "unknown")
    # Extract actual scopes/permissions from gh auth status
    scopes=$(gh auth status 2>&1 | grep "Token scopes" | cut -d: -f2- | xargs || echo "fine-grained token")

    # Read stored metadata if available
    auth_method="unknown"
    token_name=""
    token_created=""
    if [ -f ~/.claude-vm/gh-auth-info ]; then
        source ~/.claude-vm/gh-auth-info
    fi

    # For token auth, show link to manage tokens (not create)
    if [ "$auth_method" = "token" ]; then
        cat > ~/.claude-vm/context/gh.txt <<'GHEOF'
gh version: $(gh --version 2>/dev/null | head -n1)
Authentication: ✓ Logged in as USERNAME
Token scopes: SCOPES
Auth method: METHOD
GHEOF
        sed -i "s/USERNAME/$username/g" ~/.claude-vm/context/gh.txt
        sed -i "s/SCOPES/$scopes/g" ~/.claude-vm/context/gh.txt
        sed -i "s/METHOD/$auth_method/g" ~/.claude-vm/context/gh.txt

        if [ -n "$token_created" ]; then
            echo "Created: $token_created" >> ~/.claude-vm/context/gh.txt
        fi

        echo "" >> ~/.claude-vm/context/gh.txt
        echo "Manage your tokens: https://github.com/settings/personal-access-tokens" >> ~/.claude-vm/context/gh.txt
    else
        # For device flow or unknown, show standard info
        cat > ~/.claude-vm/context/gh.txt <<'GHEOF'
gh version: $(gh --version 2>/dev/null | head -n1)
Authentication: ✓ Logged in as USERNAME
Token scopes: SCOPES
Auth method: METHOD
GHEOF
        sed -i "s/USERNAME/$username/g" ~/.claude-vm/context/gh.txt
        sed -i "s/SCOPES/$scopes/g" ~/.claude-vm/context/gh.txt
        sed -i "s/METHOD/$auth_method/g" ~/.claude-vm/context/gh.txt

        if [ -n "$token_created" ]; then
            echo "Created: $token_created" >> ~/.claude-vm/context/gh.txt
        fi
    fi
else
    # Not authenticated - show re-authentication instructions based on stored method
    auth_method="unknown"
    token_name=""
    if [ -f ~/.claude-vm/gh-auth-info ]; then
        source ~/.claude-vm/gh-auth-info
    fi

    cat > ~/.claude-vm/context/gh.txt <<'GHEOF'
gh version: $(gh --version 2>/dev/null | head -n1)
Authentication: ✗ Not logged in

GHEOF

    if [ "$auth_method" = "token" ]; then
        # Previously used token - guide to use existing token or reconfigure
        cat >> ~/.claude-vm/context/gh.txt <<'GHEOF'
Authentication expired or revoked.

To re-authenticate:
GHEOF

        if [ -n "$token_name" ]; then
            echo "Previous token: $token_name" >> ~/.claude-vm/context/gh.txt
            echo "" >> ~/.claude-vm/context/gh.txt
        fi

        cat >> ~/.claude-vm/context/gh.txt <<'GHEOF'
1. Use an existing token from: https://github.com/settings/personal-access-tokens
   Then run: echo "YOUR_TOKEN" | gh auth login --with-token

2. Or run: claude-vm setup (to create a new token with correct permissions)
GHEOF
    elif [ "$auth_method" = "device" ]; then
        # Previously used device flow
        cat >> ~/.claude-vm/context/gh.txt <<'GHEOF'
To re-authenticate with device flow:
Run: gh auth login --git-protocol https

Or run: claude-vm setup (to reconfigure)
GHEOF
    else
        # No previous method - recommend setup
        cat >> ~/.claude-vm/context/gh.txt <<'GHEOF'
To authenticate, run: claude-vm setup

Or authenticate manually:
  • Token: Use existing token from https://github.com/settings/personal-access-tokens
    Then run: echo "YOUR_TOKEN" | gh auth login --with-token
  • Device flow: gh auth login --git-protocol https
GHEOF
    fi
fi
