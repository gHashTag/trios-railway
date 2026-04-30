# trios-railway Makefile
# Smoke testing targets for proving pipeline is alive in <60 seconds

.PHONY: all build test smoke clean smoke-test-only smoke-locally

# Default target
all: build

# Build all crates
build:
	cargo build --all --release

# Run all tests
test:
	cargo test --all

# Smoke test: full pipeline validation in <60s
smoke: smoke-test-only smoke-locally
	@echo "✓✓✓ SMOKE TEST COMPLETE - pipeline is healthy"

# Unit smoke tests only (no DB)
smoke-test-only:
	@echo "Running unit smoke tests..."
	cargo test -p trios-railway-smoke --lib -- --nocapture

# Local smoke test (mock DB)
smoke-locally:
	@echo "Running smoke-agent with mock DB..."
	cargo run -p trios-igla-race --bin smoke-agent --features smoke \
		--neon-url "mock://skip-db" \
		--worker-id local-smoke 2>&1 | tee smoke-output.jsonl
	@echo ""
	@echo "=== Checking for 'zero steps' bug ==="
	@if grep -q '"step"' smoke-output.jsonl; then \
		STEPS=$$(grep -c '"step"' smoke-output.jsonl); \
		echo "✓ Found $$STEPS step(s) in output"; \
		if [ "$$STEPS" -ge 1 ]; then \
			echo "✓✓✓ SMOKE PASSED - no zero steps bug"; \
		else \
			echo "✗ FAIL: Expected at least 1 step"; \
			exit 1; \
		fi \
	else \
		echo "✗ FAIL: ZERO STEPS DETECTED"; \
		echo "This is the bug causing 186 failed experiments!"; \
		exit 1; \
	fi

# Format code
fmt:
	cargo fmt --all

# Lint code
lint:
	cargo clippy --all-targets --all-features -- -D warnings

# Clean build artifacts
clean:
	cargo clean

# Check smoke integration
check-smoke-ci:
	@echo "This would run in CI via: .github/workflows/smoke.yml"
	@echo "To run locally: make smoke"

# Quick smoke (faster, less output)
smoke-quick:
	@echo "Quick smoke test..."
	cargo test -p trios-railway-smoke --lib smoke_test 2>&1 | head -20

# Help target
help:
	@echo "trios-railway Makefile"
	@echo ""
	@echo "Targets:"
	@echo "  all              - Build all crates (default)"
	@echo "  build            - Build all crates"
	@echo "  test             - Run all tests"
	@echo "  smoke            - Full smoke test: unit + local agent (<60s)"
	@echo "  smoke-test-only  - Unit tests only (no DB)"
	@echo "  smoke-locally    - Run smoke-agent with mock DB"
	@echo "  smoke-quick      - Quick unit test only"
	@echo "  fmt              - Format code"
	@echo "  lint             - Lint with clippy"
	@echo "  clean            - Clean build artifacts"
	@echo "  help             - Show this help"
