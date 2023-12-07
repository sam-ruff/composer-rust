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

# Find the download URL for the specific release file
DOWNLOAD_URL=$(echo $LATEST_RELEASE_JSON | jq -r --arg FILENAME "$FILENAME" '.assets[] | select(.name == $FILENAME).browser_download_url')
if [ -z "$DOWNLOAD_URL" ]; then
    echo "Failed to find download URL for the specified file."
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

# Download the specific file into $HOME/.local/bin
curl -L $DOWNLOAD_URL -o "$HOME/.local/bin/$FILENAME"
if [ $? -ne 0 ]; then
    echo "Failed to download the specified file."
    exit 1
fi

# Optionally, make the file executable
chmod +x "$HOME/.local/bin/$FILENAME"

echo "Operation completed successfully."
