# TypeMill Production Dockerfile
# Multi-stage build for minimal final image

# Build stage
FROM rust:1.75-bookworm AS builder

WORKDIR /build

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests first for dependency caching
COPY Cargo.toml Cargo.lock ./
COPY apps/mill/Cargo.toml apps/mill/
COPY crates/ crates/
COPY languages/ languages/

# Create dummy src files for dependency compilation
RUN mkdir -p apps/mill/src && echo "fn main() {}" > apps/mill/src/main.rs
RUN for dir in crates/*/; do mkdir -p "$dir/src" && echo "" > "$dir/src/lib.rs"; done
RUN for dir in languages/*/; do mkdir -p "$dir/src" && echo "" > "$dir/src/lib.rs"; done

# Build dependencies only
RUN cargo build --release --package mill 2>/dev/null || true

# Copy actual source
COPY . .

# Touch files to rebuild with real sources
RUN touch apps/mill/src/main.rs
RUN find crates -name "*.rs" -exec touch {} \;
RUN find languages -name "*.rs" -exec touch {} \;

# Build release binary
RUN cargo build --release --package mill

# Runtime stage
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 mill

WORKDIR /app

# Copy binary from builder
COPY --from=builder /build/target/release/mill /usr/local/bin/mill

# Set ownership
RUN chown -R mill:mill /app

USER mill

ENTRYPOINT ["mill"]
CMD ["start"]
