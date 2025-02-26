#!/usr/bin/env bash
set -euo pipefail

#############################################################
# This script creates a tarball (`plugins.tar.gz`)          # 
# containing all plugins and their checksums.               #
# The resulting files are moved to a `releases/` directory. #
#############################################################

# Create a `releases` directory
mkdir -p releases

# Create a tarball with the plugins, placing them under a `plugins/` folder in the tarball
find target/wasm32-wasip2/release/ \
  -type f \( -name "*.wasm" \) -print0 \
  | tar --null --transform 's|.*/|plugins/|' -czvf plugins.tar.gz --files-from -

# Move the tarball to the releases directory and generate its checksum
sha256sum plugins.tar.gz > releases/plugins.tar.gz.sha256
mv plugins.tar.gz releases/

echo "Tarball created successfully."