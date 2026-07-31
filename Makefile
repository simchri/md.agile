# Build & packaging entry points for mdagile.
#
# Everything runs inside the project's docker-compose dev image (see
# docker-compose.yml / docker/Dockerfile) so the host only needs Docker
# installed. No `devenv` wrapper is required.
#
# Targets:
#   make toolchain      - build (or rebuild) the docker dev image
#   make build-release  - release-build the cli+lsp (cargo) and the gui (dx bundle)
#   make package        - assemble mdagile-cli, mdagile-lsp, mdagile-gui .deb packages into dist/

SHELL := bash

# docker-compose service used for all container commands. The "-no-gpu"
# variant is the default since building/packaging doesn't need GPU passthrough.
COMPOSE_SERVICE ?= dev-container-no-gpu

# docker-compose.yml requires these env vars to be set.
UID       := $(shell id -u)
USER      := $(shell whoami)
DISPLAY   ?= :0
export UID USER DISPLAY

COMPOSE_RUN := docker compose run --rm -T $(COMPOSE_SERVICE) bash -lc

VERSION := $(shell sed -n 's/^version *= *"\(.*\)"/\1/p' Cargo.toml | head -1)
ARCH    := amd64

.PHONY: toolchain build-release package clean-package

## Install the toolchain: build the docker dev image (rust nightly, dioxus-cli, etc.).
toolchain:
	docker compose build $(COMPOSE_SERVICE)

## Release-build the cli+lsp binaries (cargo) and the gui web bundle (dx bundle).
build-release: toolchain
	$(COMPOSE_RUN) "set -euo pipefail && \
		cargo build --release -p mdagile && \
		cd crates/gui && dx bundle --release --platform web"

## Assemble mdagile-cli, mdagile-lsp and mdagile-gui .deb packages into dist/.
package: build-release
	$(COMPOSE_RUN) "set -euo pipefail && scripts/package-deb.sh $(VERSION) $(ARCH)"

## Remove packaging output.
clean-package:
	rm -rf dist
