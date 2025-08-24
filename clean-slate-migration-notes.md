# Clean Slate Migration Notes - Production Server

**Date**: 2025-08-24 17:22 UTC
**Purpose**: Document system before clean slate testing of cargo-dist installation

## Current System State

### Service Configuration
- **Service Name**: radarr.service
- **Binary Path**: /opt/radarr/radarr-mvp (16MB)
- **Working Directory**: /opt/radarr
- **User**: root (non-standard, should be radarr)
- **Status**: Running but auto-restarting (potential issue)

### Database
- **Database**: radarr (PostgreSQL)
- **User**: radarr
- **Backup Created**: /tmp/radarr_backup_before_clean_slate_20250824_172237.sql.gz (13KB)

### Configuration
- **Location**: /opt/radarr/.env
- **Port**: 7878
- **Environment**: production
- **API Keys**: 
  - RADARR_API_KEY: mwdSTYfAEZaNxvHPwJ1WTcknod1AALi7
  - TMDB_API_KEY: [JWT token configured]

### Directory Structure
```
/opt/radarr/
├── .env (479 bytes)
├── backups/
├── config/
├── data/
├── migrations/
└── radarr-mvp (16,125,472 bytes)
```

### Full System Backup
- **Config Backup**: /tmp/radarr_config_backup_20250824_172237/
- **Database Backup**: /tmp/radarr_backup_before_clean_slate_20250824_172237.sql.gz

## Migration Strategy: Clean Slate Testing

**Objective**: Complete removal and fresh installation using cargo-dist v1.0.0 release
**Risk**: All current data will be lost (acceptable for testing)
**Rollback**: Restore from backups if needed

## Post-Migration Testing Checklist

- [ ] Download GitHub release artifacts
- [ ] Test automated installer script
- [ ] Verify embedded web UI functionality
- [ ] Test database setup automation
- [ ] Validate service management
- [ ] Check API endpoint functionality
- [ ] Test configuration management
- [ ] Verify backup/restore scripts

## Notes

- Current system has auto-restart issues (good candidate for clean installation)
- Running as root user (new installer should create proper radarr user)
- Will test production-grade installation workflow
- Validates new user onboarding experience