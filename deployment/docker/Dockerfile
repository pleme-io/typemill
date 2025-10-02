# Stage 1: Base - for sharing source code
FROM rust:1.70-slim as base
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY rust ./rust

# Stage 2: Builder - for compiling the release binary
FROM base as builder
WORKDIR /app/rust
RUN cargo build --release --bin codebuddy

# Stage 3: Final - for production
FROM debian:bookworm-slim as final
WORKDIR /app

# Install runtime dependencies (LSPs, curl for healthcheck)
RUN apt-get update && apt-get install -y \
    curl \
    ca-certificates \
    nodejs \
    npm \
    python3 \
    python3-pip \
    && rm -rf /var/lib/apt/lists/*

# Create a non-root user for security
RUN useradd -r -s /bin/false -m codebuddy
COPY --from=builder /app/rust/target/release/codebuddy /usr/local/bin/
RUN chown codebuddy:codebuddy /usr/local/bin/codebuddy
USER codebuddy

# Install LSP servers as non-root user
ENV NPM_CONFIG_PREFIX=/home/codebuddy/.npm-global
ENV PATH="/home/codebuddy/.npm-global/bin:${PATH}"
RUN npm install -g typescript-language-server typescript && \
    pip3 install --user python-lsp-server[all]

# Expose ports and set up health check
EXPOSE 3000 4000
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:4000/health || exit 1

# Default command for production
ENTRYPOINT ["codebuddy"]
CMD ["serve", "--port", "3000"]

# Stage 4: Development - for hot-reloading
FROM base as development
WORKDIR /app/rust
RUN cargo install cargo-watch
EXPOSE 3000 4000
CMD ["cargo", "watch", "-q", "-c", "-w", ".", "-x", "run", "--", "--port", "3000"]
