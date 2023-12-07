#!/bin/bash

# Define the GitHub API URL
API_URL="https://api.github.com/repos/bytesquid/composer-rust/releases/latest"

# Ensure jq is installed for JSON parsing
if ! command -v jq &> /dev/null; then
    echo "jq is not installed. Please install jq for JSON parsing."
    exit 1
fi

# Fetch the latest release information and extract the tarball URL
TARBALL_URL=$(curl -s $API_URL | jq -r '.tarball_url')
if [ -z "$TARBALL_URL" ]; then
    echo "Failed to fetch or parse tarball URL."
    exit 1
fi

# Download the tarball
curl -L $TARBALL_URL -o release.tar.gz
if [ $? -ne 0 ]; then
    echo "Failed to download the tarball."
    exit 1
fi

# Untar the downloaded file
tar -xzf release.tar.gz
if [ $? -ne 0 ]; then
    echo "Failed to extract the tarball."
    exit 1
fi

# Check if the script argument (specific file) is provided
if [ -z "$1" ]; then
    echo "No file specified to download. Please provide a file name as the first argument."
    exit 1
fi

# Create $HOME/.local/bin if it doesn't exist
if [ ! -d "$HOME/.local/bin" ]; then
    mkdir -p "$HOME/.local/bin"
    if [ $? -ne 0 ]; then
        echo "Failed to create $HOME/.local/bin directory."
        exit 1
    fi
fi

# Install the specific file into $HOME/.local/bin
cp -r "composer-rust-*/$1" "$HOME/.local/bin/"
if [ $? -ne 0 ]; then
    echo "Failed to install the specified file."
    exit 1
fi

# Clean up: Remove the downloaded tarball and the extracted directory
rm -rf release.tar.gz composer-rust-*

echo "Operation completed successfully."
