# Build stage
FROM rust:1.82-slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Build dependencies (cached layer)
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy source code
COPY . .

# Build application
RUN touch src/main.rs && \
    cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 -s /bin/bash rshelper

# Copy binary from builder
COPY --from=builder /app/target/release/rs-helper /usr/local/bin/rs-helper

# Copy templates
COPY --from=builder /app/templates /app/templates

# Set working directory
WORKDIR /app

# Change ownership
RUN chown -R rshelper:rshelper /app

# Switch to non-root user
USER rshelper

# Expose ports
EXPOSE 3000 9090

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/healthz || exit 1

# Set default environment variables
ENV SERVER_HOST=0.0.0.0 \
    SERVER_PORT=3000 \
    CACHE_ENABLED=true \
    RATE_LIMIT_ENABLED=true \
    METRICS_ENABLED=true \
    RUST_LOG=rs_helper=info

# Run the application
CMD ["rs-helper"]
