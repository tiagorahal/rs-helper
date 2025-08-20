.PHONY: help build run test clean docker-build docker-run format lint audit

# Variables
BINARY_NAME := rs-helper
DOCKER_IMAGE := rs-helper
DOCKER_TAG := latest
PORT := 3000

# Colors for output
RED := \033[0;31m
GREEN := \033[0;32m
YELLOW := \033[1;33m
NC := \033[0m # No Color

## help: Display this help message
help:
	@echo "$(GREEN)RS Helper - Available Commands$(NC)"
	@echo ""
	@grep -E '^##' Makefile | sed 's/## //'
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' Makefile | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(YELLOW)%-15s$(NC) %s\n", $$1, $$2}'

## build: Build the application in release mode
build:
	@echo "$(GREEN)Building $(BINARY_NAME)...$(NC)"
	cargo build --release
	@echo "$(GREEN)Build complete!$(NC)"

## dev: Run in development mode with auto-reload
dev:
	@echo "$(GREEN)Starting development server...$(NC)"
	cargo watch -x 'run' -w src -w templates

## run: Run the application
run:
	@echo "$(GREEN)Starting $(BINARY_NAME)...$(NC)"
	cargo run --release

## test: Run all tests
test:
	@echo "$(GREEN)Running tests...$(NC)"
	cargo test --verbose
	cargo test --doc

## test-watch: Run tests in watch mode
test-watch:
	cargo watch -x 'test'

## bench: Run benchmarks
bench:
	@echo "$(GREEN)Running benchmarks...$(NC)"
	cargo bench

## clean: Clean build artifacts
clean:
	@echo "$(RED)Cleaning build artifacts...$(NC)"
	cargo clean
	rm -rf target/
	@echo "$(GREEN)Clean complete!$(NC)"

## format: Format code using rustfmt
format:
	@echo "$(GREEN)Formatting code...$(NC)"
	cargo fmt
	@echo "$(GREEN)Format complete!$(NC)"

## lint: Run clippy linter
lint:
	@echo "$(GREEN)Running clippy...$(NC)"
	cargo clippy -- -D warnings

## audit: Run security audit
audit:
	@echo "$(GREEN)Running security audit...$(NC)"
	cargo audit

## docs: Generate and open documentation
docs:
	@echo "$(GREEN)Generating documentation...$(NC)"
	cargo doc --no-deps --open

## docker-build: Build Docker image
docker-build:
	@echo "$(GREEN)Building Docker image...$(NC)"
	docker build -t $(DOCKER_IMAGE):$(DOCKER_TAG) .
	@echo "$(GREEN)Docker build complete!$(NC)"

## docker-run: Run Docker container
docker-run:
	@echo "$(GREEN)Starting Docker container...$(NC)"
	docker run -d \
		--name $(BINARY_NAME) \
		-p $(PORT):$(PORT) \
		-p 9090:9090 \
		--env-file .env \
		$(DOCKER_IMAGE):$(DOCKER_TAG)
	@echo "$(GREEN)Container started on port $(PORT)$(NC)"

## docker-stop: Stop Docker container
docker-stop:
	@echo "$(RED)Stopping Docker container...$(NC)"
	docker stop $(BINARY_NAME) || true
	docker rm $(BINARY_NAME) || true
	@echo "$(GREEN)Container stopped$(NC)"

## docker-logs: Show Docker container logs
docker-logs:
	docker logs -f $(BINARY_NAME)

## compose-up: Start with docker-compose
compose-up:
	@echo "$(GREEN)Starting services with docker-compose...$(NC)"
	docker-compose up -d
	@echo "$(GREEN)Services started!$(NC)"
	@echo "  - App: http://localhost:$(PORT)"
	@echo "  - Metrics: http://localhost:9090/metrics"
	@echo "  - Prometheus: http://localhost:9091"
	@echo "  - Grafana: http://localhost:3001 (admin/admin)"

## compose-down: Stop docker-compose services
compose-down:
	@echo "$(RED)Stopping services...$(NC)"
	docker-compose down
	@echo "$(GREEN)Services stopped$(NC)"

## compose-logs: Show docker-compose logs
compose-logs:
	docker-compose logs -f

## install-deps: Install development dependencies
install-deps:
	@echo "$(GREEN)Installing development dependencies...$(NC)"
	cargo install cargo-watch cargo-edit cargo-audit
	@echo "$(GREEN)Dependencies installed!$(NC)"

## check: Run all checks (format, lint, test, audit)
check: format lint test audit
	@echo "$(GREEN)All checks passed!$(NC)"

## release: Create a new release
release:
	@echo "$(GREEN)Creating release build...$(NC)"
	cargo build --release
	strip target/release/$(BINARY_NAME)
	@echo "$(GREEN)Release binary created at target/release/$(BINARY_NAME)$(NC)"

## perf: Run performance test
perf:
	@echo "$(GREEN)Running performance test...$(NC)"
	@echo "Starting server..."
	@cargo run --release > /dev/null 2>&1 & SERVER_PID=$$!; \
	sleep 3; \
	echo "Running benchmark..."; \
	wrk -t12 -c400 -d30s --latency http://localhost:$(PORT)/healthz; \
	kill $$SERVER_PID

## loc: Count lines of code
loc:
	@echo "$(GREEN)Lines of code:$(NC)"
	@tokei src templates

## env: Create .env file from example
env:
	@if [ ! -f .env ]; then \
		cp .env.example .env; \
		echo "$(GREEN).env file created from .env.example$(NC)"; \
	else \
		echo "$(YELLOW).env file already exists$(NC)"; \
	fi

## update: Update dependencies
update:
	@echo "$(GREEN)Updating dependencies...$(NC)"
	cargo update
	@echo "$(GREEN)Dependencies updated!$(NC)"

## ci: Run CI pipeline locally
ci:
	@echo "$(GREEN)Running CI pipeline...$(NC)"
	@$(MAKE) format
	@$(MAKE) lint
	@$(MAKE) test
	@$(MAKE) audit
	@$(MAKE) build
	@echo "$(GREEN)CI pipeline complete!$(NC)"

# Default target
.DEFAULT_GOAL := help
