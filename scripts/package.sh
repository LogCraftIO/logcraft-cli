#!/bin/bash
set -e

#########################################################################################
# This script creates two tarballs for Linux and macOS:                                #
# 1. `lgc-minimal-{os}-{arch}.tar.gz`: Contains `target/release/lgc` and `README.md`.   #
# 2. `lgc-{os}-{arch}.tar.gz`: Extends the above by including `.wasm` files.           #
#########################################################################################

# Ensure RUNNER_OS and RUNNER_ARCH are set
if [ -z "$RUNNER_OS" ]; then
  echo "Error: RUNNER_OS is not set. Exiting."
  exit 1
fi
if [ -z "$RUNNER_ARCH" ]; then
  echo "Error: RUNNER_ARCH is not set. Exiting."
  exit 1
fi

# Convert RUNNER_OS to lowercase
os=$(echo "$RUNNER_OS" | tr '[:upper:]' '[:lower:]')
arch=$(echo "$RUNNER_ARCH" | tr '[:upper:]' '[:lower:]')

# Define package names
minimal_package="lgc-minimal-${os}-${arch}.tar.gz"
full_package="lgc-${os}-${arch}.tar.gz"

# Create a `releases` directory
mkdir -p releases

# Create tarball with CLI and Readme
if [ "$os" == "linux" ]; then
  tar --null --transform 's|target/release/||' -czvf "$minimal_package" target/release/lgc README.md
else
  # For macOS, manually adjust file paths
  tar -czvf "$minimal_package" -C target/release lgc -C ../../ README.md
fi

# Check for .wasm files and create tarball with CLI, plugins, and Readme
wasm_files=$(find target/wasm32-wasip2/release/ -type f -name "*.wasm")
if [ -n "$wasm_files" ]; then
  if [ "$os" == "linux" ]; then
    find target/wasm32-wasip2/release/ \
      -type f \( -name "*.wasm" \) -print0 \
      | tar --null --transform 's|target/release/||' --transform 's|.*/|plugins/|' \
      -czvf "$full_package" target/release/lgc README.md --files-from -
  else
    # Move wasm plugins in the correct directory
    mkdir -p temp/plugins
    cp target/wasm32-wasip2/release/*.wasm temp/plugins/
    cp target/release/lgc README.md temp/
    # For macOS, manually adjust file paths
    tar -czvf "$full_package" -C temp .;
    rm -rf temp
  fi
else
  echo "No .wasm files found. Skipping creation of $full_package."
fi

# Move the tarballs to the releases directory and generate checksums
sha256sum "$full_package" > "releases/$full_package.sha256"
mv "$full_package" releases/
sha256sum "$minimal_package" > "releases/$minimal_package.sha256"
mv "$minimal_package" releases/

echo "Tarballs created successfully."