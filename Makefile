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

# NOTE: the dev image's ENTRYPOINT is already ["/bin/bash"], so the command
# passed to `docker compose run` becomes *arguments* to that bash, not a
# separate invocation. Passing "-c <cmd>" (without repeating "bash") makes
# bash run <cmd> directly; repeating "bash" here would make bash treat the
# literal word "bash" as a script filename to execute, which fails. Using
# "-c" rather than "-lc" avoids a login shell re-sourcing /etc/profile,
# which resets PATH and drops the image's /usr/local/cargo/bin entry.
COMPOSE_RUN := docker compose run --rm -T $(COMPOSE_SERVICE) -c

VERSION := $(shell sed -n 's/^version *= *"\(.*\)"/\1/p' Cargo.toml | head -1)
ARCH    := amd64

# Where detect-packaging-system records the host's packaging system, and
# where install reads it back from. Lives under dist/ (gitignored) since it's
# only meaningful alongside a fresh `package` build.
PACKAGING_SYSTEM_FILE := dist/.packaging-system

.PHONY: help toolchain build-release package clean-package detect-packaging-system install

.DEFAULT_GOAL := help

## Show this help message (default target).
help:
	@echo "Available targets:"
	@awk 'BEGIN {FS = ":.*"} /^## / {desc = substr($$0, 4)} /^[a-zA-Z_-]+:/ && desc {printf "  %-16s %s\n", $$1, desc; desc = ""}' $(MAKEFILE_LIST)

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

## Detect the host's (not the container's) packaging system; errors out if unsupported.
detect-packaging-system:
	@mkdir -p dist
	@if [ -r /etc/os-release ] && ( . /etc/os-release; \
			case " $$ID $$ID_LIKE " in *" debian "*) exit 0 ;; *) exit 1 ;; esac ); then \
		echo "debian" > $(PACKAGING_SYSTEM_FILE); \
		echo "Detected host packaging system: debian"; \
	else \
		echo "error: unsupported host packaging system (only Debian/Ubuntu-family hosts, i.e. apt/dpkg, are currently supported)" >&2; \
		exit 1; \
	fi

## Install the built .deb packages (mdagile-cli, mdagile-lsp, mdagile-gui) onto the host.
install: package detect-packaging-system
	@system="$$(cat $(PACKAGING_SYSTEM_FILE))"; \
	case "$$system" in \
		debian) \
			echo "Installing packages via apt-get..."; \
			sudo apt-get install -y ./dist/mdagile-cli_$(VERSION)_$(ARCH).deb \
				./dist/mdagile-lsp_$(VERSION)_$(ARCH).deb \
				./dist/mdagile-gui_$(VERSION)_$(ARCH).deb \
			;; \
		*) \
			echo "error: unsupported packaging system '$$system'" >&2; \
			exit 1 \
			;; \
	esac
