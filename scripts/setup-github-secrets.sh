#!/bin/bash

# GitHub Secrets Setup Script
# This script helps you securely configure GitHub Actions secrets

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "================================================"
echo "GitHub Actions Secrets Configuration"
echo "================================================"
echo ""

# Check if gh CLI is installed
if ! command -v gh &> /dev/null; then
    echo -e "${RED}GitHub CLI (gh) is not installed.${NC}"
    echo "Install it from: https://cli.github.com/"
    exit 1
fi

# Check if authenticated
if ! gh auth status &> /dev/null; then
    echo -e "${YELLOW}Not authenticated with GitHub CLI.${NC}"
    echo "Running: gh auth login"
    gh auth login
fi

# Get repository info
REPO=$(gh repo view --json nameWithOwner -q .nameWithOwner)
echo -e "${GREEN}Configuring secrets for repository: $REPO${NC}"
echo ""

# Function to set a secret
set_secret() {
    local secret_name=$1
    local secret_description=$2
    local required=$3
    
    echo "----------------------------------------"
    echo -e "${YELLOW}$secret_name${NC}"
    echo "$secret_description"
    echo ""
    
    if [ "$required" = "required" ]; then
        echo -e "${RED}This secret is REQUIRED for CI/CD to work properly.${NC}"
    else
        echo "This secret is optional but recommended."
    fi
    echo ""
    
    read -p "Do you want to set $secret_name? (y/n): " -n 1 -r
    echo ""
    
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        echo "Enter the value for $secret_name (input will be hidden):"
        read -s secret_value
        
        if [ -z "$secret_value" ]; then
            echo -e "${RED}No value entered. Skipping...${NC}"
        else
            echo "$secret_value" | gh secret set "$secret_name"
            echo -e "${GREEN}✓ $secret_name has been set successfully${NC}"
        fi
    else
        echo "Skipping $secret_name"
    fi
    echo ""
}

# Configure each secret
echo "Let's configure your GitHub Actions secrets:"
echo ""

set_secret "CODACY_PROJECT_TOKEN" \
    "Token for Codacy code analysis and coverage reporting.
    Get it from: https://app.codacy.com/gh/$REPO/settings
    Navigate to: Integrations → Project API → Create API Token" \
    "required"

set_secret "CODECOV_TOKEN" \
    "Token for Codecov coverage reporting.
    Get it from: https://codecov.io/gh/$REPO
    Navigate to: Settings → General → Repository Upload Token" \
    "optional"

set_secret "SNYK_TOKEN" \
    "Token for Snyk vulnerability scanning.
    Get it from: https://app.snyk.io/account
    Navigate to: Account Settings → Auth Token" \
    "optional"

set_secret "GITLEAKS_LICENSE" \
    "License key for GitLeaks Pro features.
    Only needed if you have a GitLeaks Pro license." \
    "optional"

set_secret "DEPENDENCY_TRACK_URL" \
    "URL for Dependency Track SBOM submission.
    Example: https://your-dependency-track.com" \
    "optional"

set_secret "DEPENDENCY_TRACK_API_KEY" \
    "API key for Dependency Track.
    Get it from your Dependency Track instance." \
    "optional"

# List configured secrets
echo "================================================"
echo "Configured Secrets:"
echo "================================================"
gh secret list

echo ""
echo -e "${GREEN}✓ Secret configuration complete!${NC}"
echo ""
echo "Next steps:"
echo "1. Trigger a workflow run to test the configuration"
echo "2. Check the Actions tab in GitHub to monitor the run"
echo "3. Review any errors in the workflow logs"
echo ""
echo "To manually trigger a workflow:"
echo "  gh workflow run ci.yml"
echo ""
echo -e "${YELLOW}Remember: NEVER commit secrets directly to your repository!${NC}"