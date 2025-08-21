# Multi-stage Dockerfile for Radarr MVP
# Builds both Rust backend and React frontend with optimal layer caching

# ========================================
# Stage 1: Rust Builder
# ========================================
FROM rust:1.75 as rust-builder

# Install required system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app directory and set working directory
WORKDIR /app

# Copy dependency files first for better layer caching
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/

# Create a dummy src/main.rs to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies (cached layer)
RUN cargo build --release && rm -rf src/

# Copy actual source code
COPY src/ ./src/
COPY migrations/ ./migrations/

# Build the actual application
RUN cargo build --release --bin radarr-mvp

# ========================================
# Stage 2: Web Builder (Node.js)
# ========================================
FROM node:20-alpine as web-builder

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
# Stage 3: Runtime
# ========================================
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Create app user for security
RUN useradd --create-home --shell /bin/bash --user-group --uid 1000 radarr

# Create necessary directories
RUN mkdir -p /usr/local/share/radarr/web \
    && mkdir -p /var/lib/radarr \
    && mkdir -p /var/log/radarr \
    && chown -R radarr:radarr /var/lib/radarr /var/log/radarr

# Copy binary from rust builder
COPY --from=rust-builder /app/target/release/radarr-mvp /usr/local/bin/radarr-mvp
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

# Set the command
CMD ["radarr-mvp"]