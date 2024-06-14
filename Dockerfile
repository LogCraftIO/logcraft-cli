# -----
FROM docker.io/library/rust:1.78-alpine as builder
# TODO: Add `git clone` for kcl github repo (forked version until PR valid and in release)
# # Set `SYSROOT` to a dummy path (default is /usr) because pkg-config-rs *always*
# # links those located in that path dynamically but we want static linking, c.f.
# # https://github.com/rust-lang/pkg-config-rs/blob/54325785816695df031cef3b26b6a9a203bbc01b/src/lib.rs#L613
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
# FROM alpine
FROM cgr.dev/chainguard/static
ARG version="0.1.0"
ARG release="v0.1.0"
LABEL name="lgc" \
      maintainer="dev@logcraft.io" \
      vendor="LogCraft" \
      license="MPL-2.0" \
      version=${version} \
      release=${release} \
      summary="Detection-as-Code CLI" \
      description="Easily build Detection-as-Code pipelines for modern security tools (SIEM, EDR, XDR, ...)"

COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=builder /wd/target/release/lgc /lgc
ENTRYPOINT ["/lgc"]