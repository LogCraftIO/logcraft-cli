# -----
FROM docker.io/library/rust:1.78-alpine as builder

# Set `SYSROOT` to a dummy path (default is /usr) because pkg-config-rs *always*
# links those located in that path dynamically but we want static linking, c.f.
# https://github.com/rust-lang/pkg-config-rs/blob/54325785816695df031cef3b26b6a9a203bbc01b/src/lib.rs#L613
ENV SYSROOT=/dummy

# Install dependencies
RUN apk update && apk add --no-cache \
    g++ \
    musl-dev \
    libressl-dev \
    protobuf-dev

ENV PROTOC=/usr/bin/protoc

WORKDIR /wd
COPY . /wd

RUN cargo build --bin lgc --release

# -----
FROM cgr.dev/chainguard/wolfi-base:latest

LABEL org.opencontainers.image.title            "LogCraft CLI"
LABEL org.opencontainers.image.authors          "LogCraft <dev@logcraft.io>"
LABEL org.opencontainers.image.url              "https://github.com/LogCraftIO/logcraft-cli/pkgs/container/logcraft-cli"
LABEL org.opencontainers.image.documentation    "https://docs.logcraft.io/"
LABEL org.opencontainers.image.source           "https://github.com/LogCraftIO/logcraft-cli"
LABEL org.opencontainers.image.vendor           "LogCraft"
LABEL org.opencontainers.image.licenses         "MPL-2.0"
LABEL org.opencontainers.image.description      "Easily build Detection-as-Code pipelines for modern security tools (SIEM, EDR, XDR, ...)"

WORKDIR /wd
RUN chown -R nonroot.nonroot /wd/
USER nonroot

COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=builder /wd/target/release/lgc /usr/local/bin/lgc