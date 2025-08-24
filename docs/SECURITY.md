# Security Guide: Credential Management for Radarr MVP

## 🔒 Overview

This document outlines comprehensive security practices for managing credentials, API keys, and sensitive data in the Radarr MVP project. **All developers must follow these procedures to prevent credential exposure.**

## 🚨 Emergency Response: Credential Exposure

If credentials are accidentally committed to git:

### Immediate Actions (Within 5 minutes)
1. **Revoke/Rotate the exposed credential immediately**:
   - HDBits: Generate new passkey in account settings
   - TMDB: Regenerate API key in developer console
   - Database: Change passwords and connection strings
   - Discord: Regenerate webhook URLs

2. **Remove from git history**:
   ```bash
   # For recent commits (not pushed)
   git reset --soft HEAD~1
   git commit --amend
   
   # For pushed commits - DANGEROUS (requires force push)
   git filter-branch --force --index-filter \
   'git rm --cached --ignore-unmatch path/to/file' \
   --prune-empty --tag-name-filter cat -- --all
   
   git push --force-with-lease
   ```

3. **Notify team immediately** about the exposure
4. **Update all deployment environments** with new credentials
5. **Monitor for unauthorized usage** of the old credentials

## 🔐 Credential Management Standards

### Environment Variables (.env files)

#### ✅ Correct Usage
- **Development**: Use `.env` (never commit)
- **Examples**: Use `.env.example` with placeholder values
- **Testing**: Use `.env.test` with mock/test data only
- **Production**: Use secure environment variable injection

#### ❌ Never Do
- Commit `.env` files to git
- Use real credentials in example files
- Store credentials in code files
- Use production credentials in test files

### HDBits Passkey Management

**What it is**: 128-character hexadecimal authentication key for HDBits API access

#### ✅ Secure Storage
```bash
# In .env (local development only)
HDBITS_PASSKEY=your_actual_passkey_here

# In production (environment injection)
export HDBITS_PASSKEY="actual_passkey"
```

#### ✅ Example/Template Format
```bash
# In .env.example
HDBITS_PASSKEY=xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx

# Alternative placeholder
HDBITS_PASSKEY=your_hdbits_passkey_here_128_chars
```

#### ❌ Security Violations
- Hardcoding passkeys in source code
- Including real passkeys in test files
- Committing passkeys to version control
- Sharing passkeys in documentation or chat

### TMDB API Key Management

**What it is**: JWT bearer token for The Movie Database API access

#### ✅ Secure Storage
```bash
# In .env
TMDB_API_KEY=eyJhbGciOiJIUzI1NiJ9.your_actual_jwt_token.signature

# In Rust code (reading from environment)
let api_key = std::env::var("TMDB_API_KEY")
    .expect("TMDB_API_KEY must be set");
```

#### ✅ Example Format
```bash
# In .env.example
TMDB_API_KEY=eyJhbGciOiJIUzI1NiJ9.eyJhdWQiOiJ5b3VyX2FwaV9rZXlfaGVyZSIsInN1YiI6InlvdXJfdXNlcl9pZCIsInNjb3BlcyI6WyJhcGlfcmVhZCJdLCJ2ZXJzaW9uIjoxfQ.your_signature_here
```

### Database Credentials

#### ✅ Connection String Format
```bash
# Development
DATABASE_URL=postgresql://radarr_dev:secure_dev_password@localhost:5432/radarr_dev

# Production (with environment variables)
DATABASE_URL=postgresql://${DB_USER}:${DB_PASSWORD}@${DB_HOST}:${DB_PORT}/${DB_NAME}
```

#### ✅ Separate Components
```bash
DB_HOST=localhost
DB_PORT=5432
DB_NAME=radarr
DB_USER=radarr_user
DB_PASSWORD=secure_random_password_here
```

### qBittorrent Credentials

#### ✅ Secure Configuration
```bash
QBITTORRENT_HOST=http://192.168.0.138:8080
QBITTORRENT_USERNAME=admin
QBITTORRENT_PASSWORD=secure_admin_password
```

### Discord Webhook URLs

#### ✅ Environment Storage
```bash
DISCORD_WEBHOOK_URL=https://discord.com/api/webhooks/1234567890123456789/abcdefghijklmnopqrstuvwxyz
```

## 🛡️ Secret Detection System

### GitLeaks Configuration
The project uses GitLeaks with custom rules to detect:
- HDBits passkeys (128-char hex)
- TMDB JWT tokens
- Generic API keys (64+ chars)
- Database connection strings
- Discord webhooks
- Basic auth credentials
- Prometheus tokens

### Pre-commit Hooks
Automatically runs before each commit:
- **GitLeaks**: Primary secret detection
- **detect-secrets**: Secondary pattern matching
- **Talisman**: Advanced sensitive data detection
- **Custom checks**: Environment files, test credentials

### Installation
```bash
# Install pre-commit
pip install pre-commit

# Install hooks
pre-commit install

# Test configuration
pre-commit run --all-files
```

## 📋 Environment File Standards

### .env.example Template
```bash
# HDBits Configuration
HDBITS_PASSKEY=your_hdbits_passkey_128_characters_here
HDBITS_USERNAME=your_hdbits_username

# TMDB API
TMDB_API_KEY=eyJhbGciOiJIUzI1NiJ9.your_tmdb_jwt_token_here.signature

# Database
DATABASE_URL=postgresql://username:password@localhost:5432/radarr
DB_HOST=localhost
DB_PORT=5432
DB_NAME=radarr
DB_USER=radarr_user
DB_PASSWORD=your_secure_password

# qBittorrent
QBITTORRENT_HOST=http://localhost:8080
QBITTORRENT_USERNAME=admin
QBITTORRENT_PASSWORD=your_qbittorrent_password

# Notifications
DISCORD_WEBHOOK_URL=https://discord.com/api/webhooks/your/webhook/url
EMAIL_SMTP_HOST=smtp.gmail.com
EMAIL_SMTP_PORT=587
EMAIL_USERNAME=your_email@gmail.com
EMAIL_PASSWORD=your_app_password

# Application
API_PORT=8000
LOG_LEVEL=info
RUST_BACKTRACE=1
```

### Development Setup
```bash
# 1. Copy example to development environment
cp .env.example .env

# 2. Edit .env with your actual credentials
nano .env

# 3. Never commit .env to git
echo "Your .env file should appear in .gitignore"
```

## 🔄 Credential Rotation Procedures

### Monthly Rotation (Recommended)
1. **HDBits Passkey**:
   - Login to HDBits account
   - Navigate to Profile → Security
   - Generate new passkey
   - Update all environments

2. **TMDB API Key**:
   - Visit TMDB API settings
   - Regenerate bearer token
   - Update configuration

3. **Database Passwords**:
   - Update database user passwords
   - Update connection strings
   - Restart all services

### Quarterly Rotation (Required)
- All API keys and tokens
- Database credentials
- Service account passwords
- Webhook URLs

## 🔍 Security Auditing

### Regular Security Scans
```bash
# Run GitLeaks on entire repository
gitleaks detect --config=.gitleaks.toml --verbose

# Run detect-secrets baseline update
detect-secrets scan --baseline .secrets.baseline

# Check for exposed credentials in git history
gitleaks detect --config=.gitleaks.toml --log-level=info --no-git
```

### Security Checklist

#### Before Each Release
- [ ] Run full credential scan
- [ ] Verify no secrets in git history
- [ ] Confirm all example files use placeholders
- [ ] Test pre-commit hooks are working
- [ ] Rotate any credentials older than 90 days

#### Monthly Security Review
- [ ] Audit environment variable usage
- [ ] Review access logs for unauthorized usage
- [ ] Update security documentation
- [ ] Train team on new security practices
- [ ] Test emergency response procedures

## 📞 Security Contact Information

### Internal Team
- **Security Lead**: [Your Security Contact]
- **DevOps**: [Your DevOps Contact]
- **Project Lead**: [Your Project Lead]

### External Resources
- **HDBits Support**: Via their internal messaging system
- **TMDB Support**: https://www.themoviedb.org/talk
- **Discord Security**: https://discord.com/safety

## 🚨 Known Security Incidents

### Previous Exposures (Reference)
As of the last security scan (`secret_scan_report.json`), the following credential types were previously exposed:
- HDBits passkeys (multiple instances)
- TMDB JWT tokens
- Generic API keys
- Grafana credentials
- Prometheus tokens
- Basic auth credentials

**All exposed credentials have been rotated and are no longer valid.**

## 🔄 Security Updates

This document is updated with each security enhancement. Last updated: **August 24, 2025**

### Recent Changes
- Added comprehensive GitLeaks configuration
- Implemented pre-commit security hooks
- Enhanced .gitignore for security artifacts
- Created emergency response procedures
- Documented credential rotation procedures

---

**Remember**: Security is everyone's responsibility. When in doubt, ask the team and err on the side of caution.