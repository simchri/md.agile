# FROM rustlang/rust:nightly-trixie-slim
# ref by digest (static, ensure always get the same version)
FROM rustlang/rust@sha256:abcd6e8ab04df97c789b48172c6b55fdc2799f7e6440f9bb515a3191819a416c

ARG USER
ARG UID

RUN mkdir /home/${USER}
RUN chown -R ${UID} /home/${USER}

RUN cargo install cargo-binstall

RUN apt-get update && apt-get install --yes \
      bash \
      build-essential \
      curl \
      file \
      g++ \
      git \
      libssl-dev \
      make \
      sudo \
      vim \
      wget

RUN rustup component add rust-analyzer
RUN rust-analyzer --version

RUN chown -R ${UID} /usr/local/cargo/

ENTRYPOINT ["/bin/bash"]
ENV TERM=xterm-256color

RUN useradd -m -u ${UID} ${USER}
USER ${USER}

RUN curl -fsSL https://claude.ai/install.sh | bash

ENV PATH="/home/${USER}/.local/bin:${PATH}"

# Claude
RUN curl -fsSL https://claude.ai/install.sh | bash
ENV PATH="/home/${USER}/.local/bin:${PATH}"
