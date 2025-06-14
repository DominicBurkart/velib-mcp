# Build stage
FROM rust:1.82-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -m -u 1001 appuser

# Set work directory
WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src

# Build application
RUN cargo build --release

# Runtime stage
FROM gcr.io/distroless/cc-debian12

# Copy user from builder
COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group

# Copy the binary
COPY --from=builder /app/target/release/velib-mcp /usr/local/bin/velib-mcp

# Use non-root user
USER appuser

# Expose port
EXPOSE 8080

# Run the binary
ENTRYPOINT ["/usr/local/bin/velib-mcp"]