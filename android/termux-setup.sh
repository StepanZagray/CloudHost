#!/bin/bash

# Termux setup script for CloudHost TUI
# This script helps users set up CloudHost TUI in Termux

echo "📱 Setting up CloudHost TUI for Termux on Android..."

# Check if running in Termux
if [ ! -d "/data/data/com.termux" ]; then
    echo "❌ This script should be run in Termux"
    echo "Please install Termux from F-Droid and run this script there"
    exit 1
fi

echo "✅ Running in Termux environment"

# Update package list
echo "📦 Updating package list..."
pkg update -y

# Install required packages
echo "📦 Installing required packages..."
pkg install -y rust cargo git curl wget

# Install additional tools for development
echo "📦 Installing development tools..."
pkg install -y clang make cmake

# Set up Rust environment
echo "🔧 Setting up Rust environment..."
rustup update

# Create project directory
echo "📁 Setting up project directory..."
mkdir -p ~/cloudhost-tui
cd ~/cloudhost-tui

# Clone the repository (if not already present)
if [ ! -d "cloudhost-tui" ]; then
    echo "📥 Cloning CloudHost TUI repository..."
    git clone https://github.com/StepanZagray/cloudhost-tui.git
fi

cd cloudhost-tui

# Build the project
echo "🔨 Building CloudHost TUI..."
cargo build --release

# Check if build was successful
if [ $? -eq 0 ]; then
    echo "✅ Build successful!"
    echo "📱 CloudHost TUI is ready to use"
    echo ""
    echo "🚀 To run CloudHost TUI:"
    echo "   cd ~/cloudhost-tui/cloudhost-tui"
    echo "   ./target/release/cloudhost-tui"
    echo ""
    echo "📋 Required permissions:"
    echo "   • Storage access (granted automatically in Termux)"
    echo "   • Network access (for cloud functionality)"
    echo "   • File system access (for cloud folders)"
else
    echo "❌ Build failed!"
    echo "Please check the error messages above"
    exit 1
fi
