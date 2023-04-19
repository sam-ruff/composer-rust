#!/bin/bash

# Set environment variables
export S3_URL="https://composer-releases.s3.eu-west-2.amazonaws.com/composer"
export BINARY_NAME="composer"
export INSTALL_DIR="$HOME/.local/bin"

# Function to download and install the binary
install_binary() {
    # Check if the install directory exists, if not, create it
    if [ ! -d "$INSTALL_DIR" ]; then
        echo "Creating install directory: $INSTALL_DIR"
        mkdir -p "$INSTALL_DIR"
    fi

    # Download the binary from the S3 bucket
    echo "Downloading binary from $S3_URL"
    curl -fsSL "$S3_URL" -o "$INSTALL_DIR/$BINARY_NAME"
    download_status=$?

    if [ $download_status -ne 0 ]; then
        echo "Error: Unable to download the binary. Please check the URL and try again."
        exit 1
    fi

    # Set the binary as executable
    chmod +x "$INSTALL_DIR/$BINARY_NAME"
    echo "Binary has been installed to $INSTALL_DIR/$BINARY_NAME"

    # Add the installation directory to the PATH, if not already present
    if [[ ! ":$PATH:" == *":$INSTALL_DIR:"* ]]; then
        echo 'export PATH=$PATH:'"$INSTALL_DIR" >> "$HOME/.bashrc"
        echo "Installation directory added to PATH. Please restart your terminal or run 'source ~/.bashrc' to update the PATH."
    fi
}

# Call the function to install the binary
install_binary
