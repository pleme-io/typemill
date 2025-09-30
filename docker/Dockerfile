# Multi-stage Rust build with FUSE support for Codebuddy
# Stage 1: Build
FROM rust:1.70-slim as builder
WORKDIR /build

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libfuse-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files (context is parent directory)
COPY Cargo.toml Cargo.lock ./
COPY rust/ ./rust/

# Build in release mode
WORKDIR /build/rust
RUN cargo build --release --bin codebuddy

# Stage 2: Runtime
FROM debian:bookworm-slim
WORKDIR /app

# Install runtime dependencies (FUSE + LSP servers)
RUN apt-get update && apt-get install -y \
    fuse \
    libfuse2 \
    curl \
    ca-certificates \
    # Node.js for TypeScript LSP
    nodejs \
    npm \
    # Python for Python LSP
    python3 \
    python3-pip \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user and add to fuse group
RUN useradd -r -s /bin/false -m codebuddy && \
    usermod -aG fuse codebuddy && \
    mkdir -p /workspace /tmp/codeflow-mounts && \
    chown -R codebuddy:codebuddy /workspace /tmp/codeflow-mounts

# Create config directory
RUN mkdir -p /app/.codebuddy && chown codebuddy:codebuddy /app/.codebuddy

# Copy binary from builder
COPY --from=builder /build/rust/target/release/codebuddy /usr/local/bin/

# Switch to non-root user
USER codebuddy

# Install LSP servers as non-root user in user directory
ENV NPM_CONFIG_PREFIX=/home/codebuddy/.npm-global
ENV PATH="/home/codebuddy/.npm-global/bin:${PATH}"
RUN npm install -g typescript-language-server typescript && \
    pip3 install --user python-lsp-server[all]

# Expose ports
EXPOSE 3000 4000

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:4000/health || exit 1

# Default command
ENTRYPOINT ["codebuddy"]
CMD ["serve", "--port", "3000"]