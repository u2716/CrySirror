# SPDX-License-Identifier: MPL-2.0
# Build environment based on siglus_rs platform/switch/docker/Dockerfile,
# source blob 2175175ec4aec3030868955463dca058b08d725a.
FROM devkitpro/devkita64:latest

ARG RUST_TOOLCHAIN=nightly-2026-09-17

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    DEVKITPRO=/opt/devkitpro \
    DEVKITA64=/opt/devkitpro/devkitA64
ENV PATH=$CARGO_HOME/bin:$DEVKITA64/bin:$DEVKITPRO/tools/bin:$PATH

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates curl git python3 patch build-essential pkg-config \
        librsvg2-bin imagemagick \
    && rm -rf /var/lib/apt/lists/* \
    && dkp-pacman -Syu --noconfirm --needed switch-dev deko3d uam \
    && (command -v magick || ln -s "$(command -v convert)" /usr/local/bin/magick)

# Host shader tools use stable; the Horizon library rebuilds std with nightly.
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
        | sh -s -- -y --no-modify-path --profile minimal --default-toolchain stable \
            --component rustfmt --component clippy \
    && rustup toolchain install "$RUST_TOOLCHAIN" --profile minimal --component rust-src \
    && chmod -R a+rwX "$RUSTUP_HOME" "$CARGO_HOME"
