# SPDX-License-Identifier: MPL-2.0
# Build environment based on siglus_rs platform/vita/docker/Dockerfile,
# source blob 3b6fadc21d37e77e4a373c7c950692c573698e08.
# SDK/tooling only: no game assets, fonts or proprietary shader compiler.
FROM ubuntu:22.04

ARG RUST_TOOLCHAIN=nightly-2026-09-17
ARG CARGO_VITA_VERSION=0.2.2

ENV DEBIAN_FRONTEND=noninteractive \
    VITASDK=/usr/local/vitasdk \
    RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo
ENV PATH=$VITASDK/bin:$CARGO_HOME/bin:$PATH

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential ca-certificates curl git python3 xz-utils zstd unzip \
    && rm -rf /var/lib/apt/lists/*

# bootstrap-vitasdk.sh creates VITASDK itself and rejects an existing directory.
RUN git clone --depth 1 https://github.com/vitasdk/vdpm /tmp/vdpm \
    && /tmp/vdpm/bootstrap-vitasdk.sh \
    && VDPM_NONINTERACTIVE=1 "$VITASDK/bin/vdpm" install libvita2d \
    && test -f "$VITASDK/arm-vita-eabi/lib/libvita2d.a" \
    && rm -rf /tmp/vdpm "$VITASDK/var/cache/pacman/pkg"/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
        | sh -s -- -y --no-modify-path --profile minimal \
            --default-toolchain "$RUST_TOOLCHAIN" --component rust-src \
    && cargo install cargo-vita --version "$CARGO_VITA_VERSION" --locked \
    && rm -rf "$CARGO_HOME/registry" "$CARGO_HOME/git" \
    && chmod -R a+rwX "$RUSTUP_HOME" "$CARGO_HOME"
