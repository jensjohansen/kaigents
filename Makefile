
.PHONY: help fmt lint test ci docs-build docs-serve version

help: ## Show this help message
	@echo "Kaigents Platform"
	@echo ""
	@echo "Available commands:"
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-20s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

fmt: ## Format code (Rust/Go/TS when present)
	@if [ -f engine/Cargo.toml ]; then (cd engine && cargo fmt --all); else echo "skip: no engine/Cargo.toml"; fi
	@if [ -f operator/go.mod ]; then (cd operator && gofmt -w $$(find . -type f -name '*.go')); else echo "skip: no operator/go.mod"; fi
	@if [ -f package.json ]; then npm run -s fmt || true; else echo "skip: no package.json"; fi

lint: ## Lint code (Rust/Go/TS when present)
	@if [ -f engine/Cargo.toml ]; then (cd engine && cargo clippy --all-targets --all-features -- -D warnings); else echo "skip: no engine/Cargo.toml"; fi
	@if [ -f operator/go.mod ]; then \
		GOLANGCI_LINT_BIN="$$(command -v golangci-lint 2>/dev/null)"; \
		if [ -z "$$GOLANGCI_LINT_BIN" ] && [ -x "$$HOME/go/bin/golangci-lint" ]; then GOLANGCI_LINT_BIN="$$HOME/go/bin/golangci-lint"; fi; \
		if [ -z "$$GOLANGCI_LINT_BIN" ]; then echo "error: golangci-lint not found; install with 'go install github.com/golangci/golangci-lint/cmd/golangci-lint@latest'"; exit 1; fi; \
		(cd operator && "$$GOLANGCI_LINT_BIN" run ./...); \
	else echo "skip: no operator/go.mod"; fi
	@if [ -f package.json ]; then npm run -s lint || true; else echo "skip: no package.json"; fi

test: ## Run unit tests (Rust/Go/TS when present)
	@if [ -f engine/Cargo.toml ]; then (cd engine && cargo test --all); else echo "skip: no engine/Cargo.toml"; fi
	@if [ -f operator/go.mod ]; then (cd operator && go test ./...); else echo "skip: no operator/go.mod"; fi
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
