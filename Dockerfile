# ==============================================================================
# Dockerfile: Visual Basic 6.0 Runtime (MSVBVM60) Test Runner on Alpine Linux
# ==============================================================================

# Stage 1: Build the standalone native runner for Alpine Linux (musl)
FROM rust:alpine AS builder

WORKDIR /usr/src/classicbasic

# Install required build utilities
RUN apk add --no-cache bash musl-dev

# Copy build manifests, configs, definitions, and workspace crates
COPY Cargo.toml Cargo.lock msvbvm60.def ./
COPY .cargo .cargo
COPY crates crates

# Compile vb6-run runner in release mode (statically linked for musl)
RUN RUSTFLAGS="-C target-feature=+crt-static" cargo build --bin vb6-run --release

# Stage 2: Minimal Alpine Linux runtime environment
FROM alpine:latest

WORKDIR /app

# Copy the compiled standalone runner
COPY --from=builder /usr/src/classicbasic/target/x86_64-alpine-linux-musl/release/vb6-run /usr/local/bin/vb6-run

# Copy the legacy VB6 test binaries and registry
COPY artifacts /app/artifacts
COPY registry.json /app/registry.json

# Expose HTTP / WebSocket port for --gui-web WebAssembly Canvas streaming
EXPOSE 8080

# Execute the VB6 executable and print its text output
ENTRYPOINT ["/usr/local/bin/vb6-run"]
CMD ["/app/artifacts/bin/App42-gui-calculator-e2e-native.exe", "--gui-web"]
