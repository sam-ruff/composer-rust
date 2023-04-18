#!/bin/bash
mkdir .tmp-install
cd .tmp-install || exit
# Grab the latest release
curl -s https://api.github.com/repos/bytesquid/composer-rust/releases/latest | grep "browser_download_url.*gnu.tar.gz" | cut -d : -f 2,3 | tr -d \" | wget -qi
# Untar it
tar -xzf *.tar.gz
# Move it to PATH
# Install the composer binary for the current user
echo "Installing the composer binary for the current user..."
INSTALL_DIR="$HOME/.local/bin"
mkdir -p "$INSTALL_DIR"
cp "./composer" "$INSTALL_DIR"

# Check if the installation was successful
if [ $? -eq 0 ]; then
  echo "Installation successful. The composer binary has been installed to $INSTALL_DIR"
  echo "Make sure $INSTALL_DIR is in your PATH."
else
  echo "Installation failed. Please check the error messages above." >&2
  exit 1
fi