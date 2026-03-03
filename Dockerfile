# Multi-stage build for apchat
# Stage 1: Build the application
FROM rust:slim AS builder

# Install build dependencies
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

# Install nightly toolchain for edition2024 support
RUN rustup toolchain install nightly && \
    rustup default nightly

WORKDIR /app

# Copy the entire workspace
COPY . .

# Build the release binary
RUN cargo build --release

# Stage 2: Create minimal runtime image
# Use debian:sid-slim for newer glibc to match the builder
FROM debian:sid-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y ca-certificates libssl3 && \
    rm -rf /var/lib/apt/lists/*

# Create a non-root user
RUN useradd -m -u 1000 apchat

# Create workspace directory and set ownership
RUN mkdir -p /workspace && chown apchat:apchat /workspace

# Copy the binary from builder
COPY --from=builder /app/target/release/apchat /usr/local/bin/apchat

# Copy the entrypoint wrapper
COPY docker-entrypoint.sh /usr/local/bin/docker-entrypoint.sh
RUN chmod +x /usr/local/bin/docker-entrypoint.sh

# Set the user
USER apchat

# Set working directory to /workspace for user projects
WORKDIR /workspace

# The entrypoint is the wrapper script
ENTRYPOINT ["/usr/local/bin/docker-entrypoint.sh"]

# Default to showing help
CMD ["--help"]