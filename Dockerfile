# LLM-Simulator Dockerfile
# Multi-stage build for minimal production image

# Build stage
FROM rust:1.75-bookworm AS builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./
COPY rust-toolchain.toml ./

# Create dummy source to cache dependencies
RUN mkdir -p src/bin && \
    echo "fn main() {}" > src/bin/dummy.rs && \
    echo "pub fn dummy() {}" > src/lib.rs

# Build dependencies (cached layer)
RUN cargo build --release --bin dummy || true

# Copy actual source code
COPY src ./src
COPY benches ./benches

# Build the application
RUN cargo build --release --bin llm-simulator

# Runtime stage
FROM debian:bookworm-slim AS runtime

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -r -s /bin/false simulator

# Copy binary from builder
COPY --from=builder /app/target/release/llm-simulator /usr/local/bin/

# Copy default configuration
COPY config/default.yaml /etc/llm-simulator/config.yaml

# Set ownership
RUN chown -R simulator:simulator /etc/llm-simulator

# Switch to non-root user
USER simulator

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Default command
ENTRYPOINT ["llm-simulator"]
CMD ["--config", "/etc/llm-simulator/config.yaml"]

# Labels
LABEL org.opencontainers.image.title="LLM-Simulator"
LABEL org.opencontainers.image.description="Enterprise-grade offline LLM API simulator"
LABEL org.opencontainers.image.version="1.0.0"
LABEL org.opencontainers.image.source="https://github.com/llm-devops/llm-simulator"
