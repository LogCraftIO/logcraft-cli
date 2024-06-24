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
    pkgconfig \
    libressl-dev \
    protobuf-dev \
    protoc

ENV PROTOC=/usr/bin/protoc

WORKDIR /wd
COPY . /wd

RUN cargo build --bin lgc --release

# -----
FROM cgr.dev/chainguard/wolfi-base

ARG description="Easily build Detection-as-Code pipelines for modern security tools (SIEM, EDR, XDR, ...)"
LABEL name="lgc" \
      maintainer="dev@logcraft.io" \
      vendor="LogCraft" \
      license="MPL-2.0" \
      summary="Detection-as-Code CLI" \
      description=${description}
LABEL org.opencontainers.image.description ${description}

WORKDIR /wd
RUN chown -R nonroot.nonroot /wd/
USER nonroot

COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=builder /wd/target/release/lgc /usr/local/bin/lgc