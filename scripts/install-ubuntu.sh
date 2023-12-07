#!/bin/bash

# Define the GitHub API URL
API_URL="https://api.github.com/repos/bytesquid/composer-rust/releases/latest"

# Ensure jq is installed for JSON parsing
if ! command -v jq &> /dev/null; then
    echo "jq is not installed. Please install jq for JSON parsing."
    exit 1
fi

# Fetch the latest release information
LATEST_RELEASE_JSON=$(curl -s $API_URL)
if [ -z "$LATEST_RELEASE_JSON" ]; then
    echo "Failed to fetch release information."
    exit 1
fi

# Extract the version number
VERSION=$(echo $LATEST_RELEASE_JSON | jq -r '.tag_name')
if [ -z "$VERSION" ]; then
    echo "Failed to extract version number."
    exit 1
fi

# Construct the filename
FILENAME="composer-$VERSION-ubuntu-latest-x86_64-unknown-linux-gnu"

# Extract the tarball URL
TARBALL_URL=$(echo $LATEST_RELEASE_JSON | jq -r '.tarball_url')
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

# Create $HOME/.local/bin if it doesn't exist
if [ ! -d "$HOME/.local/bin" ]; then
    mkdir -p "$HOME/.local/bin"
    if [ $? -ne 0 ]; then
        echo "Failed to create $HOME/.local/bin directory."
        exit 1
    fi
fi

# Install the specific file into $HOME/.local/bin
cp -r "composer-rust-*/$FILENAME" "$HOME/.local/bin/"
if [ $? -ne 0 ]; then
    echo "Failed to install the specified file."
    exit 1
fi

# Clean up: Remove the downloaded tarball and the extracted directory
rm -rf release.tar.gz composer-rust-*

echo "Operation completed successfully."
