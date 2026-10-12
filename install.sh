#!/usr/bin/env bash
# =============================================================================
# ⚡ PHP-Hyperion Universal Installer for All Operating Systems
# Supports: macOS (Apple Silicon M1-M4 & Intel x86_64), Linux (x86_64 & aarch64)
# =============================================================================

set -e

BOLD="\033[1m"
GREEN="\033[0;32m"
CYAN="\033[0;36m"
YELLOW="\033[1;33m"
RED="\033[0;31m"
RESET="\033[0m"

echo -e "${CYAN}${BOLD}"
echo "  ⚡ PHP-Hyperion Universal Deployment Installer"
echo "  Next-Generation High-Performance PHP Runtime"
echo -e "${RESET}"

OS="$(uname -s)"
ARCH="$(uname -m)"

echo -e "${BOLD}Detecting System Environment:${RESET}"
echo -e "  Operating System: ${GREEN}${OS}${RESET}"
echo -e "  Architecture:     ${GREEN}${ARCH}${RESET}"

# Determine target installation directory
INSTALL_DIR="/usr/local/bin"
if [ "$OS" = "Darwin" ]; then
    if [ -d "/opt/homebrew/bin" ] && [ -w "/opt/homebrew/bin" ]; then
        INSTALL_DIR="/opt/homebrew/bin"
    elif [ -d "/Users/$USER/.cargo/bin" ] && [ -w "/Users/$USER/.cargo/bin" ]; then
        INSTALL_DIR="/Users/$USER/.cargo/bin"
    fi
fi

if [ ! -d "$INSTALL_DIR" ]; then
    mkdir -p "$INSTALL_DIR" 2>/dev/null || true
fi

TARGET_FILE="${INSTALL_DIR}/hyperion"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

install_local_or_remote() {
    # Check if pre-compiled deployed binary is present in current repository
    if [ -f "${SCRIPT_DIR}/deploy/hyperion" ]; then
        echo -e "\n${CYAN}Installing pre-compiled Universal binary from ${SCRIPT_DIR}/deploy/hyperion...${RESET}"
        if [ -w "$INSTALL_DIR" ]; then
            cp "${SCRIPT_DIR}/deploy/hyperion" "$TARGET_FILE"
        else
            echo -e "${YELLOW}Need sudo permissions to write to ${INSTALL_DIR}:${RESET}"
            sudo cp "${SCRIPT_DIR}/deploy/hyperion" "$TARGET_FILE"
        fi
    elif [ -f "${SCRIPT_DIR}/bin/hyperion" ]; then
        echo -e "\n${CYAN}Installing pre-compiled Universal binary from ${SCRIPT_DIR}/bin/hyperion...${RESET}"
        if [ -w "$INSTALL_DIR" ]; then
            cp "${SCRIPT_DIR}/bin/hyperion" "$TARGET_FILE"
        else
            echo -e "${YELLOW}Need sudo permissions to write to ${INSTALL_DIR}:${RESET}"
            sudo cp "${SCRIPT_DIR}/bin/hyperion" "$TARGET_FILE"
        fi
    else
        echo -e "\n${CYAN}Downloading latest production release binary from GitHub...${RESET}"
        TMP_DIR="$(mktemp -d)"
        REPO="Yassinidi/PHP-Hyperion"
        
        # Download universal deploy binary
        curl -fsSL "https://raw.githubusercontent.com/${REPO}/main/deploy/hyperion" -o "${TMP_DIR}/hyperion"
        
        if [ -w "$INSTALL_DIR" ]; then
            mv "${TMP_DIR}/hyperion" "$TARGET_FILE"
        else
            echo -e "${YELLOW}Need sudo permissions to write to ${INSTALL_DIR}:${RESET}"
            sudo mv "${TMP_DIR}/hyperion" "$TARGET_FILE"
        fi
        rm -rf "$TMP_DIR"
    fi

    # Set executable permissions
    if [ -w "$TARGET_FILE" ]; then
        chmod +x "$TARGET_FILE"
    else
        sudo chmod +x "$TARGET_FILE"
    fi
}

install_local_or_remote

echo -e "\n${GREEN}${BOLD}✓ Installation Successful!${RESET}"
echo -e "Installed binary: ${CYAN}${TARGET_FILE}${RESET}"

# Verify version
if command -v hyperion >/dev/null 2>&1; then
    echo -e "\n${BOLD}Runtime Verification:${RESET}"
    hyperion --version
else
    echo -e "${YELLOW}Note: Ensure ${INSTALL_DIR} is in your PATH:${RESET}"
    echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
    "${TARGET_FILE}" --version
fi

echo -e "\n${CYAN}${BOLD}Quick Start:${RESET}"
echo "  1. Run single script:          hyperion script.php"
echo "  2. Built-in web server:        hyperion -S 127.0.0.1:8000"
echo "  3. Persistent worker mode:     hyperion -S 127.0.0.1:8000 -W"
echo "  4. FastCGI backend for Nginx:  hyperion -b 127.0.0.1:9000 -W"
