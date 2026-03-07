
.PHONY: help fmt lint test ci docs-build docs-serve version

help: ## Show this help message
	@echo "Kaigents Platform"
	@echo ""
	@echo "Available commands:"
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-20s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

fmt: ## Format code (Rust/Go/TS when present)
	@if [ -f Cargo.toml ]; then cargo fmt --all; else echo "skip: no Cargo.toml"; fi
	@if [ -f go.mod ]; then gofmt -w $$(find . -type f -name '*.go'); else echo "skip: no go.mod"; fi
	@if [ -f package.json ]; then npm run -s fmt || true; else echo "skip: no package.json"; fi

lint: ## Lint code (Rust/Go/TS when present)
	@if [ -f Cargo.toml ]; then cargo clippy --all-targets --all-features -- -D warnings; else echo "skip: no Cargo.toml"; fi
	@if [ -f go.mod ]; then golangci-lint run ./...; else echo "skip: no go.mod"; fi
	@if [ -f package.json ]; then npm run -s lint || true; else echo "skip: no package.json"; fi

test: ## Run unit tests (Rust/Go/TS when present)
	@if [ -f Cargo.toml ]; then cargo test --all; else echo "skip: no Cargo.toml"; fi
	@if [ -f go.mod ]; then go test ./...; else echo "skip: no go.mod"; fi
	@if [ -f package.json ]; then npm run -s test || true; else echo "skip: no package.json"; fi

ci: ## Run CI checks locally
	@$(MAKE) fmt
	@$(MAKE) lint
	@$(MAKE) test

docs-build: ## Build docs (mkdocs when present)
	@if [ -f mkdocs.yml ]; then mkdocs build; else echo "skip: no mkdocs.yml"; fi

docs-serve: ## Serve docs locally (mkdocs when present)
	@if [ -f mkdocs.yml ]; then mkdocs serve; else echo "skip: no mkdocs.yml"; fi

version: ## Show current version
	@echo "Version: $$(cat VERSION 2>/dev/null || echo 'missing VERSION file')"
