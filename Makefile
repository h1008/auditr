# See https://marmelab.com/blog/2016/02/29/auto-documented-makefile.html

.PHONY: help build build/release test lint clean

.DEFAULT_GOAL := help

help: ## This help
	@grep -E '^[a-zA-Z_/-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

build: ## Build the executable
	@cargo build --target x86_64-unknown-linux-musl

build/release: ## Build the executable as a release
	@cargo build --target x86_64-unknown-linux-musl --release

test: ## Run all tests
	@cargo test

lint: ## Run the linter
	@cargo clippy

clean: ## Remove build artifacts
	@rm -r target/
