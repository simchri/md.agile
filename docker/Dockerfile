# FROM rustlang/rust:nightly-trixie-slim
# ref by digest (static, ensure always get the same version)
FROM rustlang/rust@sha256:abcd6e8ab04df97c789b48172c6b55fdc2799f7e6440f9bb515a3191819a416c

ARG USER
ARG UID

RUN mkdir /home/${USER}
RUN chown -R ${UID} /home/${USER}

# Pinned to 1.18.1 (not the newer 1.24.0): 1.19.0+ bundles a Cargo.lock that
# pins cargo-platform@0.3.3 (requires rustc 1.91) and vergen@10.0.3 (requires
# rustc 1.96.0), both newer than this image's pinned rustc 1.90.0-nightly.
# 1.18.1 is the latest release still locked to cargo-platform@0.3.2/vergen@9.1.0
# (rustc 1.88 each), which this toolchain satisfies. Bump this together with
# the FROM digest above if the base image's rustc is ever upgraded.
RUN cargo install cargo-binstall@1.18.1 --locked

RUN apt-get update && apt-get install --yes \
      bash \
      bats \
      build-essential \
      curl \
      file \
      g++ \
      git \
      libayatana-appindicator3-dev \
      libcanberra-gtk3-module=0.30-18 \
      libglib2.0-dev \
      librsvg2-dev \
      libssl-dev \
      libsqlite3-dev \
      libwebkit2gtk-4.1-dev \
      libxdo-dev \
      libssl-dev \
      make \
      rpm \
      sudo \
      vim \
      wget

RUN cargo binstall dioxus-cli@0.7.7

RUN rustup component add rust-analyzer
RUN rust-analyzer --version

RUN chown -R ${UID} /usr/local/cargo/

ENTRYPOINT ["/bin/bash"]
ENV TERM=xterm-256color

RUN useradd -m -u ${UID} ${USER}
USER ${USER}

ENV PATH="/home/${USER}/.local/bin:${PATH}"

ENV VISUAL=vim
