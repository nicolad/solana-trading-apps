.PHONY: build run release test clean help

# Default target
.DEFAULT_GOAL := help

# Build in debug mode
build:
	@echo "🔨 Building DefiTuna bot (debug)..."
	cargo build

# Build in release mode
release:
	@echo "🚀 Building DefiTuna bot (release)..."
	cargo build --release

# Run in debug mode
run:
	@echo "▶️  Running DefiTuna bot (debug)..."
	cargo run

# Run in release mode
run-release:
	@echo "▶️  Running DefiTuna bot (release)..."
	cargo run --release

# Run tests
test:
	@echo "🧪 Running tests..."
	cargo test

# Check code without building
check:
	@echo "🔍 Checking code..."
	cargo check

# Format code
fmt:
	@echo "✨ Formatting code..."
	cargo fmt

# Lint code
lint:
	@echo "🔎 Linting code..."
	cargo clippy -- -D warnings

# Clean build artifacts
clean:
	@echo "🧹 Cleaning build artifacts..."
	cargo clean

# Setup environment
setup:
	@echo "⚙️  Setting up environment..."
	@if [ ! -f .env ]; then \
		cp .env.example .env; \
		echo "✅ Created .env file from template"; \
		echo "⚠️  Please edit .env and add your configuration"; \
	else \
		echo "✅ .env file already exists"; \
	fi

# Show logs
logs:
	@echo "📋 Showing recent logs..."
	RUST_LOG=info cargo run --release

# Devnet testing
devnet-check:
	@echo "🔍 Checking devnet pool status..."
	cargo run --release --bin init_devnet_pool

devnet-position:
	@echo "🐟 Attempting to open position on devnet..."
	cargo run --release --bin open_position

devnet-order:
	@echo "📊 Attempting to place order on devnet..."
	@echo "Usage: make devnet-order POSITION=<address> LOWER=95 UPPER=105"
	@if [ -z "$(POSITION)" ]; then \
		echo "❌ Error: POSITION not specified"; \
		echo "Example: make devnet-order POSITION=8iRWKA78FY8DrF1tWY7tmcUbyTTv2JBs5dssFfLRzRQv LOWER=95 UPPER=105"; \
	else \
		./target/release/place_order --position $(POSITION) --lower-price $(LOWER) --upper-price $(UPPER); \
	fi

# Build binaries
build-bins:
	@echo "🔨 Building all binaries..."
	cargo build --release --bin open_position
	cargo build --release --bin place_order
	cargo build --release --bin init_devnet_pool

# Show help
help:
	@echo "DefiTuna Trading Bot - Makefile Commands"
	@echo ""
	@echo "Usage: make [target]"
	@echo ""
	@echo "Targets:"
	@echo "  build           Build in debug mode"
	@echo "  release         Build in release mode (optimized)"
	@echo "  run             Run in debug mode"
	@echo "  run-release     Run in release mode"
	@echo "  test            Run tests"
	@echo "  check           Check code without building"
	@echo "  fmt             Format code with rustfmt"
	@echo "  lint            Lint code with clippy"
	@echo "  clean           Clean build artifacts"
	@echo "  setup           Create .env from template"
	@echo "  logs            Run with info logging"
	@echo "  devnet-check    Check if devnet pool exists"
	@echo "  devnet-position Open position on devnet (requires pool)"
	@echo "  devnet-order    Place order on devnet (requires position)"
	@echo "  build-bins      Build all binary scripts"
	@echo "  help            Show this help message"
