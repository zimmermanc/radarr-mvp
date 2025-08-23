# Radarr MVP - Rust Implementation

![CI Pipeline](https://github.com/zimmermanc/radarr-mvp/workflows/CI%20Pipeline/badge.svg)
![Security Scanning](https://github.com/zimmermanc/radarr-mvp/workflows/Security%20Scanning/badge.svg)
![Code Quality](https://github.com/zimmermanc/radarr-mvp/workflows/Code%20Quality/badge.svg)
[![Codacy Badge](https://app.codacy.com/project/badge/Grade/PROJECT_GRADE)](https://www.codacy.com/gh/zimmermanc/radarr-mvp/dashboard)
[![Codacy Coverage](https://app.codacy.com/project/badge/Coverage/PROJECT_COVERAGE)](https://www.codacy.com/gh/zimmermanc/radarr-mvp/dashboard)
[![codecov](https://codecov.io/gh/zimmermanc/radarr-mvp/branch/main/graph/badge.svg)](https://codecov.io/gh/zimmermanc/radarr-mvp)
[![Dependency Status](https://deps.rs/repo/github/zimmermanc/radarr-mvp/status.svg)](https://deps.rs/repo/github/zimmermanc/radarr-mvp)
![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)

A modern, high-performance movie collection manager built with Rust, featuring automated downloading, quality management, and comprehensive media organization.

## 🚀 Features

- **Automated Downloads**: Integration with torrent indexers and download clients
- **Quality Management**: Smart quality decision engine with custom formats
- **HDBits Integration**: Advanced scene group analysis and reputation scoring
- **Import Pipeline**: Automated file organization with hardlinking support
- **WebSocket Updates**: Real-time progress tracking and notifications
- **Circuit Breaker**: Resilient external service integration
- **PostgreSQL Backend**: Robust data persistence with migrations

## 📊 Project Status

- **Completion**: ~82% MVP Complete
- **Tests**: 162+ passing across 8 crates
- **Production**: Deployed at http://192.168.0.138:7878/
- **CI/CD**: Comprehensive GitHub Actions pipeline with security scanning

## 🛠️ Technology Stack

- **Backend**: Rust 2021, Axum, Tokio, SQLx
- **Frontend**: React, TypeScript, Vite, TailwindCSS
- **Database**: PostgreSQL 16
- **Testing**: 162+ tests with Tarpaulin coverage
- **CI/CD**: GitHub Actions, Codacy, Dependabot

## 🏗️ Architecture

```
unified-radarr/
├── crates/
│   ├── core/          # Domain logic (no external deps)
│   ├── api/           # HTTP API (Axum)
│   ├── indexers/      # Torrent indexer integrations
│   ├── decision/      # Quality decision engine
│   ├── downloaders/   # Download client integrations
│   ├── import/        # Media import pipeline
│   ├── infrastructure/# Database, external services
│   ├── analysis/      # HDBits scene analysis tools
│   └── notifications/ # Notification providers
├── web/               # React frontend
├── migrations/        # SQL migrations
└── tests/            # Integration tests
```

## 🚦 CI/CD Pipeline

Our comprehensive CI/CD pipeline ensures code quality and security:

### Security Scanning
- **SAST**: Semgrep, CodeQL for static analysis
- **SCA**: cargo-audit, Snyk for dependency vulnerabilities
- **Secrets**: GitLeaks, TruffleHog for credential scanning
- **Container**: Trivy for Docker image scanning

### Code Quality
- **Linting**: Clippy with pedantic rules
- **Formatting**: rustfmt, Prettier
- **Coverage**: Tarpaulin with Codecov/Codacy integration
- **Complexity**: cargo-bloat, cargo-geiger

### Automation
- **Dependabot**: Weekly dependency updates
- **PR Validation**: Size checks, conventional commits
- **Multi-platform**: Linux, macOS, Windows testing
- **Performance**: Benchmark regression detection

## 🚀 Quick Start

### Prerequisites
- Rust 1.75+
- PostgreSQL 16
- Node.js 20+ (for frontend)

### Installation

```bash
# Clone the repository
git clone https://github.com/zimmermanc/radarr-mvp.git
cd radarr-mvp/unified-radarr

# Install dependencies
cargo build --workspace

# Setup database
createdb radarr
export DATABASE_URL="postgresql://localhost/radarr"
sqlx migrate run

# Run tests
cargo test --workspace

# Start the server
cargo run --bin radarr-mvp
```

### Frontend Development

```bash
cd web
npm install
npm run dev
```

## 🧪 Testing

```bash
# Run all tests
cargo test --workspace

# Run with coverage
cargo tarpaulin --workspace --all-features

# Run specific crate tests
cargo test -p radarr-core

# Run integration tests
cargo test --test integration
```

## 📖 Documentation

- [CI/CD Guide](docs/CI-CD-GUIDE.md) - Complete CI/CD pipeline documentation
- [API Documentation](docs/api/) - OpenAPI specification
- [Development Setup](docs/DEVELOPER_SETUP.md) - Local development guide
- [Architecture Decisions](docs/decisions/) - ADRs and design choices

## 🔒 Security

- Regular dependency audits via cargo-audit and Dependabot
- SAST scanning with Semgrep and CodeQL
- Secret scanning with GitLeaks and TruffleHog
- Security advisories tracked in GitHub Security tab

## 🤝 Contributing

We welcome contributions! Please ensure:

1. All tests pass: `cargo test --workspace`
2. Code is formatted: `cargo fmt --all`
3. No Clippy warnings: `cargo clippy --all`
4. PR follows conventional commits format
5. Description includes what and why

## 📊 Metrics

- **Test Coverage**: ~70% (increasing)
- **Code Quality**: Codacy Grade A
- **Dependencies**: All up-to-date via Dependabot
- **Security**: No known vulnerabilities

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Original Radarr project for inspiration
- Rust community for excellent libraries
- Contributors and testers

## 📞 Support

- [GitHub Issues](https://github.com/zimmermanc/radarr-mvp/issues)
- [Documentation](docs/)
- [CI/CD Status](https://github.com/zimmermanc/radarr-mvp/actions)

---

**Note**: This is an MVP implementation focusing on core functionality. See [TASKLIST.md](TASKLIST.md) for development progress and roadmap.