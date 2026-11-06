#!/bin/sh
# Builds the plugin and stages a package directory in pkg/.
set -e
cd "$(dirname "$0")"
mkdir -p pkg
cp manifest.json pkg/manifest.json
cp {{script}} "pkg/{{script}}"
echo "plugin staged in pkg/"
