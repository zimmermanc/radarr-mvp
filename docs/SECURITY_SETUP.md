# Security Setup Guide: Implementing Secret Protection

This guide provides step-by-step instructions for setting up the secret protection system in the Radarr MVP project.

## 🚀 Quick Setup (5 minutes)

### 1. Install GitLeaks
```bash
# Option 1: Download binary (recommended)
wget https://github.com/gitleaks/gitleaks/releases/download/v8.21.2/gitleaks_8.21.2_linux_x64.tar.gz
tar -xzf gitleaks_8.21.2_linux_x64.tar.gz
sudo mv gitleaks /usr/local/bin/
rm gitleaks_8.21.2_linux_x64.tar.gz

# Option 2: Using Go (if you have Go installed)
go install github.com/gitleaks/gitleaks/v8@latest

# Option 3: Docker (alternative)
alias gitleaks="docker run --rm -v $(pwd):/code ghcr.io/gitleaks/gitleaks:latest"
```

### 2. Test GitLeaks Configuration
```bash
# Test on current repository
gitleaks detect --config=.gitleaks.toml --verbose

# Should show: "leaks found: X" (where X > 0 due to historical secrets)
```

### 3. Install Pre-commit (Optional but Recommended)
```bash
# Ubuntu/Debian
sudo apt update && sudo apt install python3-pip python3-venv
python3 -m pip install --user pre-commit

# Or use system package
sudo apt install pre-commit

# Install hooks
pre-commit install

# Test hooks
pre-commit run --all-files
```

## 📋 Complete Setup Process

### Step 1: Verify Configuration Files

Ensure these files exist in your repository root:
- ✅ `.gitleaks.toml` - GitLeaks configuration
- ✅ `.pre-commit-config.yaml` - Pre-commit hooks
- ✅ `.gitignore` - Updated with security exclusions

### Step 2: Test Secret Detection

```bash
# Create a test file with fake secrets
cat > test_secrets.txt << EOF
API_KEY=abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890
HDBITS_KEY=ed487790cd0dee98941ab5c132179bd2c8c5e23622c0c04a800ad543cde2990cd44ed960892d990214ea1618bf29780386a77246a21dc636d83420e077e69863
DATABASE_URL=postgresql://user:password123@localhost:5432/db
EOF

# Test detection
gitleaks detect --config=.gitleaks.toml --source=. --no-git

# Clean up test file
rm test_secrets.txt
```

Expected output should show multiple leaks detected.

### Step 3: Set Up Development Environment

```bash
# 1. Copy environment template
cp config/services.env.example config/services.env

# 2. Edit with your actual credentials (NEVER COMMIT THIS FILE)
nano config/services.env

# 3. Verify it's ignored by git
git status
# Should not show config/services.env in untracked files
```

### Step 4: Configure Git Hooks (Team Setup)

```bash
# Install pre-commit hooks for all developers
pre-commit install

# Install commit-msg hook for additional checks
pre-commit install --hook-type commit-msg

# Test the complete hook setup
pre-commit run --all-files

# Expected: Some hooks may fail on existing secrets (that's correct!)
```

## 🔧 Tool Configuration

### GitLeaks Rules Summary

Our configuration detects:
- **HDBits Passkeys**: 128-character hexadecimal strings
- **TMDB API Keys**: JWT bearer tokens
- **Database URLs**: PostgreSQL/MySQL connection strings
- **Generic API Keys**: 64+ character high-entropy strings
- **Discord Webhooks**: Discord API webhook URLs
- **Basic Auth**: Base64 encoded credentials
- **Service Keys**: Grafana, Prometheus, email credentials

### Pre-commit Hook Summary

The hooks run in this order:
1. **GitLeaks** - Primary secret detection
2. **detect-secrets** - Secondary pattern matching
3. **Talisman** - Advanced sensitive data detection
4. **File checks** - Large files, merge conflicts, etc.
5. **Configuration validation** - JSON, YAML, TOML syntax
6. **Custom checks** - Environment files, test credentials
7. **Rust security** - Clippy security lints
8. **SQL safety** - Database migration credential check

## 🚨 Emergency Procedures

### If a Secret is Detected During Commit

```bash
# Commit will be blocked with output like:
# ❌ GitLeaks found secrets in your commit
# File: config/services.env
# Secret: HDBits passkey detected

# 1. Remove the secret from the file
nano path/to/file/with/secret

# 2. Use environment variable or move to ignored file
mv config/services.env config/services.env.local

# 3. Retry commit
git add . && git commit -m "Your commit message"
```

### If a Secret is Already Committed

```bash
# 1. IMMEDIATELY rotate the exposed credential
echo "Change the actual credential in the service!"

# 2. Remove from git history (DANGEROUS - coordinate with team)
git filter-branch --force --index-filter \
'git rm --cached --ignore-unmatch path/to/file' \
--prune-empty --tag-name-filter cat -- --all

# 3. Force push (REQUIRES TEAM COORDINATION)
git push --force-with-lease origin main

# 4. All team members must re-clone the repository
```

## 🛠️ Developer Workflows

### Daily Development

```bash
# Before starting work - check for accidental secrets
gitleaks detect --config=.gitleaks.toml --source=. --no-git

# Normal development workflow
git add .
git commit -m "Your changes"
# Pre-commit hooks run automatically

# If hooks fail, fix the issues and retry
git add . && git commit -m "Your changes"
```

### Adding New Credentials

```bash
# 1. Add to environment file (ignored by git)
echo "NEW_API_KEY=your_secret_key_here" >> config/services.env

# 2. Add placeholder to example file
echo "NEW_API_KEY=your_api_key_here" >> config/services.env.example

# 3. Update documentation
echo "NEW_API_KEY=your_api_key_here" >> docs/SECURITY.md

# 4. Commit only the example and documentation
git add config/services.env.example docs/SECURITY.md
git commit -m "Add configuration for new API integration"
```

### Testing Secret Detection

```bash
# Test specific file
gitleaks detect --config=.gitleaks.toml --source=path/to/file --no-git

# Test with verbose output
gitleaks detect --config=.gitleaks.toml --verbose --redact

# Generate report
gitleaks detect --config=.gitleaks.toml --report-path=security-scan.json
```

## 📊 Monitoring and Maintenance

### Weekly Security Scan

```bash
#!/bin/bash
# security_scan.sh

echo "🔒 Weekly Security Scan - $(date)"
echo "================================="

echo "1. Scanning for secrets..."
gitleaks detect --config=.gitleaks.toml --report-path=weekly-scan.json --redact
SECRETS_FOUND=$?

echo "2. Checking pre-commit hooks..."
pre-commit run --all-files > precommit-check.log 2>&1
HOOKS_STATUS=$?

echo "3. Verifying environment files..."
if [ -f "config/services.env" ]; then
    echo "   ❌ services.env found (should be ignored)"
else
    echo "   ✅ services.env properly ignored"
fi

if [ -f "config/services.env.example" ]; then
    echo "   ✅ services.env.example exists"
else
    echo "   ❌ services.env.example missing"
fi

echo "4. Summary:"
if [ $SECRETS_FOUND -eq 0 ]; then
    echo "   ✅ No new secrets detected"
else
    echo "   ⚠️  Secrets found - check weekly-scan.json"
fi

echo "5. Next actions:"
echo "   - Review any detected secrets"
echo "   - Rotate credentials older than 90 days"
echo "   - Update security documentation"
echo "================================="
```

### Monthly Security Review

```bash
# Update secret detection rules
curl -s https://raw.githubusercontent.com/gitleaks/gitleaks/master/config/gitleaks.toml > .gitleaks.toml.latest
diff .gitleaks.toml .gitleaks.toml.latest

# Update pre-commit hooks
pre-commit autoupdate

# Review and rotate old credentials
grep -r "created.*202[0-9]" config/ | head -10
```

## 🎯 Success Criteria

After setup, you should achieve:
- ✅ No secrets in new commits (pre-commit blocks them)
- ✅ Regular secret detection scans (weekly/monthly)
- ✅ All credentials in environment variables or ignored files
- ✅ Team trained on security procedures
- ✅ Emergency response procedures tested

## 📞 Support and Troubleshooting

### Common Issues

**Pre-commit hooks too slow:**
```bash
# Skip specific hooks in development
SKIP=clippy,talisman-commit git commit -m "Quick fix"
```

**GitLeaks false positives:**
```bash
# Add to allowlist in .gitleaks.toml
[allowlist]
regexes = ["your_false_positive_pattern"]
```

**Hook installation fails:**
```bash
# Manual hook installation
cat > .git/hooks/pre-commit << 'EOF'
#!/bin/bash
gitleaks detect --config=.gitleaks.toml --staged --verbose --redact --no-banner
EOF
chmod +x .git/hooks/pre-commit
```

### Getting Help

1. Check existing security documentation: `docs/SECURITY.md`
2. Review configuration files: `.gitleaks.toml`, `.pre-commit-config.yaml`
3. Test with verbose output: `gitleaks detect --config=.gitleaks.toml --verbose`
4. Contact security team with specific error messages

---

**Security is a team effort. If you encounter issues or have questions, ask for help immediately.**