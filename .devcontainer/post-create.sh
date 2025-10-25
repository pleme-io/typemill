#!/bin/bash
# Post-create script for dev container setup
set -e

echo "🚀 Setting up Typemill development environment..."
echo ""
echo "Running: make first-time-setup"
echo "This will install all tools and build the project (~5-8 minutes)"
echo ""

# Run the complete first-time setup (same as developers use locally)
make first-time-setup

echo ""
echo "✨ Development environment ready!"
echo ""
echo "Quick start:"
echo "  • Build: cargo build"
echo "  • Test:  make test"
echo "  • Run:   cargo run -- start"
echo ""
echo "See CONTRIBUTING.md for development workflow"
