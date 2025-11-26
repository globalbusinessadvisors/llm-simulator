# ==============================================================================
# LLM-Simulator Makefile
# ==============================================================================
# Production-ready automation for building, testing, and deploying LLM-Simulator
# ==============================================================================

# ------------------------------------------------------------------------------
# Variables
# ------------------------------------------------------------------------------
APP_NAME := llm-simulator
VERSION ?= $(shell git describe --tags --always --dirty 2>/dev/null || echo "dev")
COMMIT_SHA := $(shell git rev-parse --short HEAD 2>/dev/null || echo "unknown")
BUILD_TIME := $(shell date -u '+%Y-%m-%d_%H:%M:%S')

# Container registry
REGISTRY ?= ghcr.io
REGISTRY_USER ?= llm-devops
IMAGE := $(REGISTRY)/$(REGISTRY_USER)/$(APP_NAME)
IMAGE_TAG ?= $(VERSION)
IMAGE_FULL := $(IMAGE):$(IMAGE_TAG)

# Kubernetes
NAMESPACE ?= llm-devops
KUBECONFIG ?= ~/.kube/config
HELM_RELEASE := $(APP_NAME)

# Build flags
CARGO_FLAGS := --release --locked
RUSTFLAGS := -C target-cpu=native

# Colors
COLOR_RESET := \033[0m
COLOR_BOLD := \033[1m
COLOR_GREEN := \033[32m
COLOR_YELLOW := \033[33m
COLOR_BLUE := \033[34m

.PHONY: help
help: ## Show this help message
	@echo "$(COLOR_BOLD)LLM-Simulator Makefile$(COLOR_RESET)"
	@echo ""
	@echo "$(COLOR_BLUE)Usage:$(COLOR_RESET)"
	@echo "  make [target]"
	@echo ""
	@echo "$(COLOR_BLUE)Build Targets:$(COLOR_RESET)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(COLOR_GREEN)%-20s$(COLOR_RESET) %s\n", $$1, $$2}' | sort

# ------------------------------------------------------------------------------
# Development
# ------------------------------------------------------------------------------

.PHONY: setup
setup: ## Install development dependencies
	@echo "$(COLOR_YELLOW)Installing Rust toolchain...$(COLOR_RESET)"
	rustup toolchain install stable
	rustup component add rustfmt clippy
	@echo "$(COLOR_YELLOW)Installing development tools...$(COLOR_RESET)"
	cargo install cargo-watch cargo-audit cargo-deny cargo-tarpaulin
	@echo "$(COLOR_GREEN)Setup complete!$(COLOR_RESET)"

.PHONY: dev
dev: ## Run development server with auto-reload
	cargo watch -x 'run -- serve --config simulator.example.yaml'

.PHONY: fmt
fmt: ## Format code
	cargo fmt --all

.PHONY: fmt-check
fmt-check: ## Check code formatting
	cargo fmt --all -- --check

.PHONY: lint
lint: ## Run linter (clippy)
	cargo clippy --all-targets --all-features -- -D warnings

.PHONY: check
check: fmt-check lint ## Run all code quality checks

# ------------------------------------------------------------------------------
# Building
# ------------------------------------------------------------------------------

.PHONY: build
build: ## Build release binary
	@echo "$(COLOR_YELLOW)Building $(APP_NAME) $(VERSION)...$(COLOR_RESET)"
	cargo build $(CARGO_FLAGS)
	@echo "$(COLOR_GREEN)Build complete: target/release/$(APP_NAME)$(COLOR_RESET)"

.PHONY: build-debug
build-debug: ## Build debug binary
	cargo build

.PHONY: clean
clean: ## Clean build artifacts
	cargo clean
	rm -rf dist/

.PHONY: dist
dist: build ## Create distribution archives
	@echo "$(COLOR_YELLOW)Creating distribution archives...$(COLOR_RESET)"
	mkdir -p dist
	tar czf dist/$(APP_NAME)-$(VERSION)-linux-x86_64.tar.gz -C target/release $(APP_NAME)
	cd dist && sha256sum * > checksums.txt
	@echo "$(COLOR_GREEN)Distribution archives created in dist/$(COLOR_RESET)"

# ------------------------------------------------------------------------------
# Testing
# ------------------------------------------------------------------------------

.PHONY: test
test: ## Run tests
	cargo test --all-features

.PHONY: test-verbose
test-verbose: ## Run tests with verbose output
	cargo test --all-features -- --nocapture

.PHONY: test-integration
test-integration: ## Run integration tests
	cargo test --test '*' --all-features

.PHONY: bench
bench: ## Run benchmarks
	cargo bench --no-fail-fast

.PHONY: coverage
coverage: ## Generate test coverage report
	cargo tarpaulin --verbose --all-features --workspace --timeout 300 --out Html --out Xml

# ------------------------------------------------------------------------------
# Security
# ------------------------------------------------------------------------------

.PHONY: audit
audit: ## Run security audit
	cargo audit

.PHONY: audit-fix
audit-fix: ## Update dependencies with security fixes
	cargo audit fix

.PHONY: deny
deny: ## Check for license/security issues with cargo-deny
	cargo deny check

# ------------------------------------------------------------------------------
# Docker
# ------------------------------------------------------------------------------

.PHONY: docker-build
docker-build: ## Build Docker image
	@echo "$(COLOR_YELLOW)Building Docker image $(IMAGE_FULL)...$(COLOR_RESET)"
	docker build \
		--build-arg VERSION=$(VERSION) \
		--build-arg COMMIT_SHA=$(COMMIT_SHA) \
		--build-arg BUILD_TIME=$(BUILD_TIME) \
		-t $(IMAGE_FULL) \
		-t $(IMAGE):latest \
		.
	@echo "$(COLOR_GREEN)Docker image built: $(IMAGE_FULL)$(COLOR_RESET)"

.PHONY: docker-build-multiarch
docker-build-multiarch: ## Build multi-architecture Docker image
	@echo "$(COLOR_YELLOW)Building multi-arch Docker image...$(COLOR_RESET)"
	docker buildx build \
		--platform linux/amd64,linux/arm64 \
		--build-arg VERSION=$(VERSION) \
		--build-arg COMMIT_SHA=$(COMMIT_SHA) \
		--build-arg BUILD_TIME=$(BUILD_TIME) \
		-t $(IMAGE_FULL) \
		-t $(IMAGE):latest \
		--push \
		.

.PHONY: docker-push
docker-push: ## Push Docker image to registry
	@echo "$(COLOR_YELLOW)Pushing $(IMAGE_FULL) to registry...$(COLOR_RESET)"
	docker push $(IMAGE_FULL)
	docker push $(IMAGE):latest
	@echo "$(COLOR_GREEN)Image pushed successfully!$(COLOR_RESET)"

.PHONY: docker-run
docker-run: ## Run Docker container locally
	docker run --rm -it \
		-p 8080:8080 \
		-p 9090:9090 \
		-v $(PWD)/config:/app/config:ro \
		$(IMAGE_FULL)

.PHONY: docker-scan
docker-scan: ## Scan Docker image for vulnerabilities
	docker scan $(IMAGE_FULL)

# ------------------------------------------------------------------------------
# Docker Compose
# ------------------------------------------------------------------------------

.PHONY: compose-up
compose-up: ## Start services with docker-compose
	docker-compose up -d

.PHONY: compose-down
compose-down: ## Stop services with docker-compose
	docker-compose down

.PHONY: compose-logs
compose-logs: ## View docker-compose logs
	docker-compose logs -f

.PHONY: compose-ps
compose-ps: ## List running docker-compose services
	docker-compose ps

# ------------------------------------------------------------------------------
# Kubernetes
# ------------------------------------------------------------------------------

.PHONY: k8s-apply
k8s-apply: ## Apply Kubernetes manifests
	@echo "$(COLOR_YELLOW)Applying Kubernetes manifests...$(COLOR_RESET)"
	kubectl apply -f deploy/kubernetes/ -n $(NAMESPACE)
	@echo "$(COLOR_GREEN)Manifests applied!$(COLOR_RESET)"

.PHONY: k8s-delete
k8s-delete: ## Delete Kubernetes resources
	kubectl delete -f deploy/kubernetes/ -n $(NAMESPACE)

.PHONY: k8s-status
k8s-status: ## Check Kubernetes deployment status
	kubectl get all -n $(NAMESPACE) -l app=llm-simulator
	kubectl rollout status deployment/llm-simulator -n $(NAMESPACE)

.PHONY: k8s-logs
k8s-logs: ## View Kubernetes pod logs
	kubectl logs -n $(NAMESPACE) -l app=llm-simulator --tail=100 -f

.PHONY: k8s-describe
k8s-describe: ## Describe Kubernetes deployment
	kubectl describe deployment llm-simulator -n $(NAMESPACE)

.PHONY: k8s-port-forward
k8s-port-forward: ## Forward local port to Kubernetes service
	kubectl port-forward -n $(NAMESPACE) service/llm-simulator 8080:8080

# ------------------------------------------------------------------------------
# Helm
# ------------------------------------------------------------------------------

.PHONY: helm-lint
helm-lint: ## Lint Helm chart
	helm lint deploy/helm/$(APP_NAME)

.PHONY: helm-template
helm-template: ## Render Helm templates
	helm template $(HELM_RELEASE) deploy/helm/$(APP_NAME) \
		--namespace $(NAMESPACE) \
		--set image.tag=$(IMAGE_TAG)

.PHONY: helm-install
helm-install: ## Install Helm chart
	@echo "$(COLOR_YELLOW)Installing Helm chart...$(COLOR_RESET)"
	helm upgrade --install $(HELM_RELEASE) deploy/helm/$(APP_NAME) \
		--namespace $(NAMESPACE) \
		--create-namespace \
		--set image.tag=$(IMAGE_TAG) \
		--wait \
		--timeout 5m
	@echo "$(COLOR_GREEN)Helm chart installed!$(COLOR_RESET)"

.PHONY: helm-upgrade
helm-upgrade: ## Upgrade Helm release
	@echo "$(COLOR_YELLOW)Upgrading Helm release...$(COLOR_RESET)"
	helm upgrade $(HELM_RELEASE) deploy/helm/$(APP_NAME) \
		--namespace $(NAMESPACE) \
		--set image.tag=$(IMAGE_TAG) \
		--wait \
		--timeout 5m
	@echo "$(COLOR_GREEN)Helm release upgraded!$(COLOR_RESET)"

.PHONY: helm-uninstall
helm-uninstall: ## Uninstall Helm release
	helm uninstall $(HELM_RELEASE) -n $(NAMESPACE)

.PHONY: helm-status
helm-status: ## Check Helm release status
	helm status $(HELM_RELEASE) -n $(NAMESPACE)

.PHONY: helm-values
helm-values: ## Show Helm release values
	helm get values $(HELM_RELEASE) -n $(NAMESPACE)

.PHONY: helm-package
helm-package: ## Package Helm chart
	helm package deploy/helm/$(APP_NAME) -d dist/

# ------------------------------------------------------------------------------
# CI/CD Helpers
# ------------------------------------------------------------------------------

.PHONY: ci
ci: check test ## Run CI checks (format, lint, test)
	@echo "$(COLOR_GREEN)All CI checks passed!$(COLOR_RESET)"

.PHONY: release-build
release-build: clean build dist docker-build ## Full release build
	@echo "$(COLOR_GREEN)Release build complete!$(COLOR_RESET)"

.PHONY: release-publish
release-publish: docker-push helm-package ## Publish release artifacts
	@echo "$(COLOR_GREEN)Release published!$(COLOR_RESET)"

# ------------------------------------------------------------------------------
# Monitoring and Debugging
# ------------------------------------------------------------------------------

.PHONY: metrics
metrics: ## View Prometheus metrics
	curl -s http://localhost:9090/metrics

.PHONY: health
health: ## Check health endpoint
	curl -f http://localhost:8080/health || exit 1

.PHONY: ready
ready: ## Check readiness endpoint
	curl -f http://localhost:8080/ready || exit 1

.PHONY: load-test
load-test: ## Run simple load test
	@echo "$(COLOR_YELLOW)Running load test...$(COLOR_RESET)"
	@for i in {1..100}; do \
		curl -s -X POST http://localhost:8080/v1/chat/completions \
			-H "Content-Type: application/json" \
			-d '{"model":"gpt-4-turbo","messages":[{"role":"user","content":"test"}]}' \
			> /dev/null & \
	done; \
	wait
	@echo "$(COLOR_GREEN)Load test complete!$(COLOR_RESET)"

# ------------------------------------------------------------------------------
# Documentation
# ------------------------------------------------------------------------------

.PHONY: docs
docs: ## Generate documentation
	cargo doc --no-deps --open

.PHONY: docs-all
docs-all: ## Generate documentation including dependencies
	cargo doc --open

# ------------------------------------------------------------------------------
# Utility
# ------------------------------------------------------------------------------

.PHONY: version
version: ## Show version information
	@echo "Version: $(VERSION)"
	@echo "Commit:  $(COMMIT_SHA)"
	@echo "Built:   $(BUILD_TIME)"

.PHONY: info
info: ## Show build information
	@echo "$(COLOR_BOLD)Build Information$(COLOR_RESET)"
	@echo "  App Name:      $(APP_NAME)"
	@echo "  Version:       $(VERSION)"
	@echo "  Commit SHA:    $(COMMIT_SHA)"
	@echo "  Build Time:    $(BUILD_TIME)"
	@echo "  Image:         $(IMAGE_FULL)"
	@echo "  Namespace:     $(NAMESPACE)"
	@echo "  Helm Release:  $(HELM_RELEASE)"

.PHONY: validate
validate: ## Validate all configurations
	@echo "$(COLOR_YELLOW)Validating configurations...$(COLOR_RESET)"
	@# Validate YAML files
	@for file in $(shell find deploy -name "*.yaml" -o -name "*.yml"); do \
		echo "Validating $$file..."; \
		kubectl apply --dry-run=client -f $$file 2>&1 | grep -v "Warning" || true; \
	done
	@# Validate Helm chart
	helm lint deploy/helm/$(APP_NAME)
	@echo "$(COLOR_GREEN)Validation complete!$(COLOR_RESET)"

# ------------------------------------------------------------------------------
# Default Target
# ------------------------------------------------------------------------------

.DEFAULT_GOAL := help
