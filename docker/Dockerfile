# =============================================================================
# Shared multi-service Dockerfile with sccache
# =============================================================================
# Builds any workspace service by name. Uses sccache to cache compiled object
# files across builds — typically cuts rebuild time from ~15 min to ~2 min.
#
# Usage (from the repo root — the build context must stay there):
#   docker buildx build --build-arg SERVICE=auth -f docker/Dockerfile .
# =============================================================================

# --- Stage 1: Build ----------------------------------------------------------
FROM rust:1.98.1-alpine3.23 AS builder

WORKDIR /build

RUN apk add --no-cache \
  pkgconfig \
  openssl-dev \
  openssl-libs-static \
  musl-dev \
  cmake \
  make \
  mold \
  curl

# Install sccache for compilation caching.
ARG SCCACHE_VERSION=v0.15.0
RUN curl -fsSL "https://github.com/mozilla/sccache/releases/download/${SCCACHE_VERSION}/sccache-${SCCACHE_VERSION}-x86_64-unknown-linux-musl.tar.gz" \
    | tar xz --strip-components=1 -C /usr/local/bin \
      "sccache-${SCCACHE_VERSION}-x86_64-unknown-linux-musl/sccache"

ENV RUSTC_WRAPPER=sccache

# 40G, not the 10 GiB default. This is where sccache demonstrably pays — CI
# rebuilds the same sources across runs, so the working set repeats, which is
# the case the cache is good at (locally, where sources change constantly, the
# hit rate is near zero; see .envrc).
#
# Release artifacts are larger than dev ones, and the cap is the only thing
# governing how much of a repeat build can be served rather than recompiled.
# sccache evicts LRU on its own, so a bigger cap never needs pruning — it just
# stops the working set falling out between builds. The stats printed after the
# build below are what to check if build times creep back up.
ENV SCCACHE_CACHE_SIZE=40G

# Use the mold linker by default for faster bin link times. Override with
# `--build-arg LINKER=ld` if mold ever produces a misbehaving binary; the
# stock musl ld is the safe fallback.
ARG LINKER=mold
ENV RUSTFLAGS="-C link-arg=-fuse-ld=${LINKER}"

COPY . .

ARG SERVICE
ARG BUILD_SHA
# Only the sccache object cache is mounted — NOT cargo's target/ directory.
# Mounting target/ caused stale linked binaries to be served across builds.
# sccache alone caches individual compilation units and cargo always links
# fresh, so source changes are always picked up.
RUN --mount=type=cache,id=sccache,target=/root/.cache/sccache,sharing=locked \
    sccache --start-server \
    && BUILD_SHA="${BUILD_SHA}" cargo build --release --bin ${SERVICE} \
    && echo "=== sccache stats ===" && sccache --show-stats \
    && sccache --stop-server \
    && cp /build/target/release/${SERVICE} /app-binary

# --- Stage 2: Runtime --------------------------------------------------------
FROM alpine:3.23 AS production-slim

ARG SERVICE

RUN apk add --no-cache ca-certificates && \
  addgroup -g 1000 appgroup && \
  adduser -u 1000 -G appgroup -h /app -s /sbin/nologin -D appuser

WORKDIR /app

COPY --from=builder /app-binary /app/service

RUN chown -R appuser:appgroup /app

USER appuser

ENV SERVICE__PORT=8080

EXPOSE ${SERVICE__PORT}

ENTRYPOINT ["/app/service"]
