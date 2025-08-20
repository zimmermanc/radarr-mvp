# Multi-stage Rust build for Radarr MVP
# =============================================================================

# Build stage
FROM rust:1.75-slim-bullseye as builder

LABEL stage=builder
LABEL description="Build stage for Radarr MVP"

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    ca-certificates \
    curl \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Set the working directory
WORKDIR /app

# Create a non-root user for building
RUN useradd -u 1001 -m builder
USER builder

# Copy dependency files first for better caching
COPY --chown=builder:builder Cargo.toml Cargo.lock ./
COPY --chown=builder:builder crates/*/Cargo.toml ./crates/*/

# Create dummy source files to cache dependencies
RUN find crates -name "Cargo.toml" -execdir mkdir -p src \; -execdir touch src/lib.rs \;
RUN echo 'fn main() {}' > src/main.rs

# Build dependencies only (this layer will be cached)
RUN cargo build --release --workspace

# Remove dummy files
RUN find . -name "*.rs" -delete

# Copy actual source code
COPY --chown=builder:builder . .

# Update timestamps to force rebuild
RUN find . -name "*.rs" -exec touch {} +

# Build the application
RUN cargo build --release --workspace

# Development stage (for local development with hot reload)
# =============================================================================
FROM rust:1.75-slim-bullseye as development

LABEL stage=development
LABEL description="Development stage with cargo-watch for hot reloading"

# Install runtime and development dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    ca-certificates \
    curl \
    build-essential \
    postgresql-client \
    && rm -rf /var/lib/apt/lists/*

# Install cargo-watch for hot reloading
RUN cargo install cargo-watch

# Create app user
RUN useradd -u 1001 -m -d /app radarr

WORKDIR /app

# Copy source code
COPY --chown=radarr:radarr . .

# Switch to app user
USER radarr

# Expose ports
EXPOSE 7878 9090

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:7878/health || exit 1

# Default development command with hot reload
CMD ["cargo", "watch", "-x", "run"]

# Production base stage
# =============================================================================
FROM debian:bullseye-slim as production-base

# Install runtime dependencies only
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl1.1 \
    libpq5 \
    curl \
    dumb-init \
    postgresql-client \
    redis-tools \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Create app user with specific UID/GID for consistency
RUN groupadd -g 1001 radarr && \
    useradd -u 1001 -g radarr -m -d /app -s /bin/bash radarr

# Create necessary directories
RUN mkdir -p /app/{config,logs,temp,migrations} && \
    mkdir -p /{movies,downloads} && \
    chown -R radarr:radarr /app /{movies,downloads}

WORKDIR /app

# Production stage
# =============================================================================
FROM production-base as production

LABEL maintainer="Radarr Team"
LABEL version="1.0.0"
LABEL description="Radarr MVP - Movie automation and management system"

# Copy the built binary from builder stage
COPY --from=builder --chown=radarr:radarr /app/target/release/radarr-mvp /usr/local/bin/radarr-mvp

# Copy migrations
COPY --chown=radarr:radarr migrations/ /app/migrations/

# Copy scripts and configuration templates
COPY --chown=radarr:radarr scripts/ /app/scripts/
COPY --chown=radarr:radarr .env.example /app/.env.example

# Copy and set up entrypoint script
COPY --chown=radarr:radarr scripts/docker-entrypoint.sh /usr/local/bin/docker-entrypoint.sh
RUN chmod +x /usr/local/bin/docker-entrypoint.sh

# Make binary executable
RUN chmod +x /usr/local/bin/radarr-mvp

# Switch to app user
USER radarr

# Expose ports
EXPOSE 7878 9090

# Health check endpoint
HEALTHCHECK --interval=30s --timeout=10s --start-period=60s --retries=3 \
    CMD curl -f http://localhost:7878/health || exit 1

# Default environment variables
ENV RUST_LOG=info
ENV RADARR_HOST=0.0.0.0
ENV RADARR_PORT=7878
ENV METRICS_PORT=9090

# Use custom entrypoint with dumb-init
ENTRYPOINT ["/usr/bin/dumb-init", "--", "/usr/local/bin/docker-entrypoint.sh"]

# Default command (handled by entrypoint)
CMD []

# Debug stage (for troubleshooting)
# =============================================================================
FROM production-base as debug

LABEL stage=debug
LABEL description="Debug stage with additional tools"

# Install debugging tools
RUN apt-get update && apt-get install -y \
    strace \
    gdb \
    valgrind \
    htop \
    vim \
    net-tools \
    && rm -rf /var/lib/apt/lists/*

# Copy debug binary (built with debug info)
COPY --from=builder --chown=radarr:radarr /app/target/debug/radarr-mvp /usr/local/bin/radarr-mvp
COPY --chown=radarr:radarr migrations/ /app/migrations/
COPY --chown=radarr:radarr scripts/ /app/scripts/

# Copy and set up entrypoint script
COPY --chown=radarr:radarr scripts/docker-entrypoint.sh /usr/local/bin/docker-entrypoint.sh
RUN chmod +x /usr/local/bin/radarr-mvp /usr/local/bin/docker-entrypoint.sh

USER radarr

EXPOSE 7878 9090

ENV RUST_LOG=debug
ENV RUST_BACKTRACE=1

ENTRYPOINT ["/usr/bin/dumb-init", "--", "/usr/local/bin/docker-entrypoint.sh"]
CMD []