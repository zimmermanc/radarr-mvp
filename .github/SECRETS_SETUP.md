# GitHub Secrets Configuration Guide

## 🔐 SECURITY BEST PRACTICES

**NEVER** commit secrets, tokens, or credentials to your repository. Always use GitHub Secrets for sensitive information.

## Required Secrets Setup

### 1. Codacy Project Token

1. Go to your [Codacy Project Settings](https://app.codacy.com/gh/zimmermanc/radarr-mvp/settings)
2. Navigate to **Integrations** → **Project API**
3. Click **Create API Token**
4. Copy the generated token (it will look like: `VIWQTI2FLPbxf2FB3WG9` but NEVER share it)
5. Add to GitHub:
   - Go to your GitHub repository
   - Navigate to **Settings** → **Secrets and variables** → **Actions**
   - Click **New repository secret**
   - Name: `CODACY_PROJECT_TOKEN`
   - Value: Paste your token
   - Click **Add secret**

### 2. Codecov Token (Optional but Recommended)

1. Sign in to [Codecov.io](https://codecov.io)
2. Navigate to your repository
3. Go to **Settings** → **General**
4. Copy the **Repository Upload Token**
5. Add to GitHub:
   - Name: `CODECOV_TOKEN`
   - Value: Your Codecov token

### 3. Snyk Token (Optional)

1. Sign in to [Snyk.io](https://snyk.io)
2. Go to **Account Settings** → **Auth Token**
3. Copy your authentication token
4. Add to GitHub:
   - Name: `SNYK_TOKEN`
   - Value: Your Snyk token

### 4. GitLeaks License (Optional - Pro Features)

1. Purchase GitLeaks Pro license
2. Add to GitHub:
   - Name: `GITLEAKS_LICENSE`
   - Value: Your license key

## Verifying Secret Configuration

To verify your secrets are properly configured:

1. Go to **Settings** → **Secrets and variables** → **Actions**
2. You should see these secrets listed (values are hidden):
   - ✅ CODACY_PROJECT_TOKEN
   - ✅ CODECOV_TOKEN (if configured)
   - ✅ SNYK_TOKEN (if configured)
   - ✅ GITLEAKS_LICENSE (if configured)

## Testing the Configuration

After adding secrets, trigger a workflow run:

```bash
# Push to a branch
git push origin feature/test-ci

# Or manually trigger
# Go to Actions → Select a workflow → Run workflow
```

## Security Checklist

- [ ] Never commit tokens directly to code
- [ ] Never share tokens in issues, PRs, or comments
- [ ] Rotate tokens regularly (every 90 days recommended)
- [ ] Use repository secrets, not organization secrets for project-specific tokens
- [ ] Enable secret scanning in repository settings
- [ ] Review workflow permissions regularly

## If a Token is Exposed

If you accidentally expose a token:

1. **Immediately revoke the exposed token** in the service's dashboard
2. **Generate a new token**
3. **Update GitHub Secrets** with the new token
4. **Check git history** for other exposed secrets:
   ```bash
   # Install gitleaks
   brew install gitleaks  # or download from GitHub releases
   
   # Scan repository
   gitleaks detect --source . -v
   ```
5. **Enable push protection** in GitHub settings to prevent future exposures

## Troubleshooting

### Workflow fails with "Bad credentials"
- Token may be expired or revoked
- Regenerate token in service dashboard
- Update GitHub Secret

### Codacy not receiving reports
- Verify CODACY_PROJECT_TOKEN is set
- Check workflow logs for upload errors
- Ensure repository is added to Codacy

### Coverage not updating
- Verify test database is accessible
- Check Tarpaulin is generating reports
- Confirm upload step is not skipped

## Additional Resources

- [GitHub Encrypted Secrets](https://docs.github.com/en/actions/security-guides/encrypted-secrets)
- [Codacy Documentation](https://docs.codacy.com/getting-started/integrating-codacy-with-your-git-workflow/)
- [Security Best Practices](https://docs.github.com/en/actions/security-guides/security-hardening-for-github-actions)

---

⚠️ **Remember**: Treat all tokens as passwords. Never share them publicly!