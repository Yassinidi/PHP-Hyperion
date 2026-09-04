#!/bin/bash

# Script to reinstall PCOV when switching PHP versions
# Save this as: /usr/local/bin/reinstall-pcov

echo "Reinstalling PCOV for current PHP version..."

# Get current PHP version
PHP_VERSION=$(php -r "echo PHP_MAJOR_VERSION . '.' . PHP_MINOR_VERSION;")
echo "Detected PHP $PHP_VERSION"

# Check if PCOV is already installed for this version
if php -m | grep -q pcov; then
    echo "✓ PCOV is already installed for PHP $PHP_VERSION"
    exit 0
fi

# Export paths for pcre2 (required for compilation)
export LDFLAGS="-L/opt/homebrew/opt/pcre2/lib"
export CPPFLAGS="-I/opt/homebrew/opt/pcre2/include"

# Install PCOV via PECL
echo "Installing PCOV..."
pecl install pcov

# Verify installation
if php -m | grep -q pcov; then
    echo "✓ PCOV installed successfully!"
    php -v | grep "with Zend"
else
    echo "✗ Installation failed"
    exit 1
fi
