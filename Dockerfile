# syntax=docker/dockerfile:1.7
# Multi-stage Dockerfile for Radarr MVP with modern optimizations
# Builds both Rust backend and React frontend with optimal layer caching

# ========================================
# Stage 0: Cargo Chef for Dependency Caching
# ========================================
FROM lukemathwalker/cargo-chef:latest-rust-1.89 AS chef
WORKDIR /app

# ========================================
# Stage 1: Cargo Planner
# ========================================
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ========================================
# Stage 2: Rust Builder
# ========================================
FROM chef AS rust-builder

# Install required system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Copy recipe from planner stage
COPY --from=planner /app/recipe.json recipe.json

# Build dependencies using cargo-chef for better caching
# Note: Cache mounts require BuildKit - fallback to regular build if not available
RUN cargo chef cook --release --recipe-path recipe.json

# Copy source code
COPY . .

# Build the actual application
RUN cargo build --release --bin radarr-mvp && \
    cp target/release/radarr-mvp /usr/local/bin/

# ========================================
# Stage 3: Web Builder (Node.js)
# ========================================
FROM node:22-alpine as web-builder

# Set working directory
WORKDIR /app

# Copy package files for better layer caching
COPY web/package.json web/package-lock.json* ./

# Install dependencies
RUN npm ci --only=production

# Copy web source files
COPY web/ ./

# Build the frontend
RUN npm run build

# ========================================
# Stage 4: Runtime
# ========================================
FROM debian:bookworm-slim

# Install runtime dependencies and security updates
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    tini \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean \
    && apt-get autoremove -y

# Create app user for security
RUN useradd --create-home --shell /bin/bash --user-group --uid 1000 radarr

# Create necessary directories
RUN mkdir -p /usr/local/share/radarr/web \
    && mkdir -p /var/lib/radarr \
    && mkdir -p /var/log/radarr \
    && chown -R radarr:radarr /var/lib/radarr /var/log/radarr

# Copy binary from rust builder
COPY --from=rust-builder /usr/local/bin/radarr-mvp /usr/local/bin/radarr-mvp
RUN chmod +x /usr/local/bin/radarr-mvp

# Copy web assets from web builder
COPY --from=web-builder /app/dist/ /usr/local/share/radarr/web/

# Switch to app user
USER radarr

# Set environment variables
ENV RUST_LOG=info
ENV RADARR_HOST=0.0.0.0
ENV RADARR_PORT=7878
ENV WEB_ROOT=/usr/local/share/radarr/web
ENV DATA_DIR=/var/lib/radarr

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:7878/health || exit 1

# Expose port
EXPOSE 7878

# Add container metadata for production
LABEL org.opencontainers.image.title="Radarr MVP" \
      org.opencontainers.image.description="Modern Radarr implementation in Rust" \
      org.opencontainers.image.vendor="Radarr MVP Project" \
      org.opencontainers.image.licenses="MIT" \
      org.opencontainers.image.source="https://github.com/radarr-mvp/radarr-mvp" \
      org.opencontainers.image.documentation="https://github.com/radarr-mvp/radarr-mvp/blob/main/README.md" \
      org.opencontainers.image.version="latest" \
      security.compliance="hardened" \
      security.non-root="true"

# Use tini as init system for proper signal handling
ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["radarr-mvp"]