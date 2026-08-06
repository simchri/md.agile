#!/bin/sh
# The bundled server resolves its static assets (public/) relative to its
# working directory, so cd into the install dir before exec-ing it.
cd /usr/lib/mdagile-gui || exit 1
exec ./server "$@"
