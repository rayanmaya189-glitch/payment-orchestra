# =============================================================================
# Payment Orchestra — Backend Makefile
# =============================================================================

.PHONY: help check fmt clippy test test-integration build build-release docker clean

# Default target
help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

# ─── Development ──────────────────────────────────────────────────────────────

check: ## Type-check all crates
	cargo check --workspace

fmt: ## Format all code
	cargo fmt --all

fmt-check: ## Check formatting (CI)
	cargo fmt --all -- --check

clippy: ## Run clippy lints
	cargo clippy --workspace --all-targets

clippy-strict: ## Run clippy with warnings as errors
	cargo clippy --workspace --all-targets -- -D warnings

# ─── Testing ──────────────────────────────────────────────────────────────────

test: ## Run all unit tests
	cargo test --workspace

test-verbose: ## Run tests with output
	cargo test --workspace -- --nocapture

test-integration: ## Run integration tests (requires Docker)
	cargo test --workspace --features integration_test -- --test-threads=1

test-specific: ## Run tests for a specific package (e.g., make test-specific PKG=orchestration-service)
	cargo test -p $(PKG)

test-coverage: ## Generate test coverage report
	cargo install cargo-tarpaulin
	cargo tarpaulin --workspace --out Html --output-dir coverage

# ─── Build ────────────────────────────────────────────────────────────────────

build: ## Build all packages (debug)
	cargo build --workspace

build-release: ## Build all packages (release, optimized)
	cargo build --workspace --release

build-binary: ## Build the API gateway binary
	cargo build --release --package api-gateway

# ─── Docker ───────────────────────────────────────────────────────────────────

docker-build: ## Build Docker image
	docker build -t payment-orchestra/api-gateway:latest .

docker-run: ## Run Docker container locally
	docker run -p 9020:9020 -p 9120:9120 --env-file .env payment-orchestra/api-gateway:latest

docker-compose-up: ## Start all infrastructure
	docker compose up -d

docker-compose-down: ## Stop all infrastructure
	docker compose down -v

# ─── Database ─────────────────────────────────────────────────────────────────

migrate: ## Run database migrations
	cargo run --package migrations

# ─── Kubernetes ───────────────────────────────────────────────────────────────

k8s-apply: ## Apply all K8s manifests
	kubectl apply -f k8s/all-services.yaml
	kubectl apply -f k8s/api-gateway.yaml
	kubectl apply -f k8s/orchestration-service.yaml
	kubectl apply -f k8s/ingress.yaml

k8s-delete: ## Delete all K8s resources
	kubectl delete -f k8s/ingress.yaml
	kubectl delete -f k8s/orchestration-service.yaml
	kubectl delete -f k8s/api-gateway.yaml
	kubectl delete -f k8s/all-services.yaml

k8s-status: ## Check K8s deployment status
	kubectl get pods -n payment-orchestra
	kubectl get ingress -n payment-orchestra
	kubectl get hpa -n payment-orchestra

# ─── Security ─────────────────────────────────────────────────────────────────

audit: ## Run security audit
	cargo audit

deny: ## Run dependency checks
	cargo deny check

# ─── Clean ────────────────────────────────────────────────────────────────────

clean: ## Clean build artifacts
	cargo clean

clean-docker: ## Clean Docker artifacts
	docker system prune -f
	docker volume prune -f
