#!/bin/bash

# Check if cargo is installed
if ! command -v cargo >/dev/null 2>&1; then
  echo "Cargo not found. Please install Rust and Cargo before proceeding." >&2
  exit 1
fi

# Build the composer binary using cargo
echo "Building the composer binary..."
cargo build --release

# Check if the build was successful
if [ $? -ne 0 ]; then
  echo "Build failed. Please check the error messages above." >&2
  exit 1
fi

# Install the composer binary for the current user
echo "Installing the composer binary for the current user..."
INSTALL_DIR="$HOME/.local/bin"
mkdir -p "$INSTALL_DIR"
cp "target/release/composer" "$INSTALL_DIR"

# Check if the installation was successful
if [ $? -eq 0 ]; then
  echo "Installation successful. The composer binary has been installed to $INSTALL_DIR"
  echo "Make sure $INSTALL_DIR is in your PATH."
else
  echo "Installation failed. Please check the error messages above." >&2
  exit 1
fi
