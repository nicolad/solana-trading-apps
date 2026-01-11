.PHONY: help test test-all test-integration run run-release build clean preflight

help:
	@echo "Jupiter LaserStream Bot - Makefile Commands"
	@echo "============================================"
	@echo ""
	@echo "Testing:"
	@echo "  make test              - Run integration tests (excluding external APIs)"
	@echo "  make test-all          - Run ALL tests including external APIs"
	@echo "  make test-integration  - Run integration tests with output"
	@echo ""
	@echo "Running:"
	@echo "  make run               - Run bot in debug mode"
	@echo "  make run-release       - Run bot in release mode (optimized)"
	@echo "  make preflight         - Run pre-flight checks for devnet"
	@echo ""
	@echo "Building:"
	@echo "  make build             - Build in debug mode"
	@echo "  make build-release     - Build in release mode"
	@echo ""
	@echo "Utilities:"
	@echo "  make clean             - Clean build artifacts"
	@echo "  make check-env         - Verify .env file exists"

# Testing targets
test:
	@echo "🧪 Running integration tests..."
	cargo test --test integration_tests

test-all:
	@echo "🧪 Running ALL tests (including external APIs)..."
	cargo test --test integration_tests -- --include-ignored

test-integration:
	@echo "🧪 Running integration tests with output..."
	cargo test --test integration_tests -- --nocapture

# Running targets
check-env:
	@if [ ! -f .env ]; then \
		echo "⚠️  .env file not found, creating from .env.example..."; \
		cp .env.example .env; \
		echo "✅ Created .env file - please configure it before running"; \
		exit 1; \
	fi

run: check-env
	@echo "🚀 Running Jupiter LaserStream Bot (debug)..."
	cargo run --bin jupiter-bot

run-release: check-env
	@echo "🚀 Running Jupiter LaserStream Bot (release)..."
	cargo run --release --bin jupiter-bot

preflight: check-env
	@echo "🧪 Running pre-flight checks..."
	cargo run --bin preflight

# Building targets
build:
	@echo "🔨 Building in debug mode..."
	cargo build

build-release:
	@echo "🔨 Building in release mode..."
	cargo build --release

# Utility targets
clean:
	@echo "🧹 Cleaning build artifacts..."
	cargo clean
