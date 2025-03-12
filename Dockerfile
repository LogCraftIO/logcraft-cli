# -----
# Stage 1: Builder
FROM docker.io/library/rust:1.84-alpine AS builder

# Set dummy SYSROOT to force static linking
ENV SYSROOT=/dummy

# Install build dependencies
RUN apk add --no-cache musl-dev libressl-dev

# Set working directory
WORKDIR /build

# Copy source code and build artifacts
COPY . .
# Build the CLI
RUN cargo build --release -p lgc
# Build the plugins
RUN rustup target add wasm32-wasip2
RUN cargo build --release --target wasm32-wasip2 \
    -p splunk \
    -p sentinel

# -----
# Stage 2: Final image
FROM cgr.dev/chainguard/wolfi-base:latest

# Define a variable for the installation directory
ENV LOGCRAFT_DIR=/opt/logcraft-cli
ENV PATH="${LOGCRAFT_DIR}:$PATH"

# Metadata
LABEL org.opencontainers.image.title="LogCraft CLI" \
      org.opencontainers.image.authors="LogCraft <dev@logcraft.io>" \
      org.opencontainers.image.url="https://github.com/LogCraftIO/logcraft-cli/pkgs/container/logcraft-cli" \
      org.opencontainers.image.documentation="https://docs.logcraft.io/" \
      org.opencontainers.image.source="https://github.com/LogCraftIO/logcraft-cli" \
      org.opencontainers.image.vendor="LogCraft" \
      org.opencontainers.image.licenses="MPL-2.0" \
      org.opencontainers.image.description="Easily build Detection-as-Code pipelines for modern security tools (SIEM, EDR, XDR, ...)"

# Set the working directory and change ownership
WORKDIR /srv/workspace

# Copy artifacts from the builder stage using the variable
COPY --from=builder /build/target/release/lgc ${LOGCRAFT_DIR}/lgc
COPY --from=builder /build/target/wasm32-wasip2/release/splunk.wasm ${LOGCRAFT_DIR}/plugins/splunk.wasm
COPY --from=builder /build/target/wasm32-wasip2/release/sentinel.wasm ${LOGCRAFT_DIR}/plugins/sentinel.wasm