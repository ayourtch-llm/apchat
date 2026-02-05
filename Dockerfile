# Multi-stage build for apchat
# Stage 1: Build the application
FROM rust:1.83-slim as builder

# Install build dependencies
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the entire workspace
COPY . .

# Build the release binary
RUN cargo build --release -p apchat-main

# Stage 2: Create minimal runtime image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y ca-certificates libssl3 && \
    rm -rf /var/lib/apt/lists/*

# Create a non-root user
RUN useradd -m -u 1000 apchat

# Copy the binary from builder
COPY --from=builder /app/target/release/apchat /usr/local/bin/apchat

# Set the user
USER apchat

# Set working directory
WORKDIR /home/apchat

# The entrypoint is the apchat binary
ENTRYPOINT ["/usr/local/bin/apchat"]

# Default to showing help
CMD ["--help"]
