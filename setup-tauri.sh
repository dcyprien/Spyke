#!/bin/bash
# Quick setup script to prepare Tauri builds

echo "🚀 Chat App - Tauri Setup"
echo "=========================="

# Check Node.js and npm
if ! command -v node &> /dev/null; then
    echo "❌ Node.js not found. Please install Node.js 18+"
    exit 1
fi

# Check Rust
if ! command -v rustc &> /dev/null; then
    echo "❌ Rust not found. Please install Rust from https://rustup.rs/"
    exit 1
fi

echo "✅ Node.js $(node -v)"
echo "✅ npm $(npm -v)"
echo "✅ Rust $(rustc -V)"

# Install frontend dependencies
echo ""
echo "📦 Installing frontend dependencies..."
cd frontend
npm install

# Add Tauri CLI
echo ""
echo "📦 Installing Tauri CLI..."
npm install -D @tauri-apps/cli @tauri-apps/api

echo ""
echo "✅ Setup complete!"
echo ""
echo "Next steps:"
echo "1. cd frontend"
echo "2. npm run tauri:dev     (for development)"
echo "3. npm run tauri:build   (to build native app)"
echo ""
echo "📖 See TAURI_SETUP.md for detailed instructions"
