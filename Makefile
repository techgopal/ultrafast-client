# UltraFast HTTP Client Makefile

.PHONY: help build dev test clean install lint format docs benchmark

help: ## Show this help message
	@echo "UltraFast HTTP Client - Development Commands"
	@echo "==========================================="
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

install: ## Install development dependencies
	@echo "Installing Rust and Python dependencies..."
	pip install -U pip maturin
	pip install -e ".[dev]"
	rustup update
	cargo update

build: ## Build the Rust extension
	@echo "Building Rust extension..."
	maturin build

dev: ## Build and install in development mode
	@echo "Building and installing in development mode..."
	maturin develop

test: ## Run tests
	@echo "Running Rust tests..."
	cargo test
	@echo "Running Python tests..."
	pytest tests/ -v

test-rust: ## Run only Rust tests
	cargo test

test-python: ## Run only Python tests
	pytest tests/ -v

lint: ## Run linting
	@echo "Running Rust linting..."
	cargo clippy -- -D warnings
	@echo "Running Python linting..."
	ruff check python/ tests/ examples/
	mypy python/ultrafast_client/

format: ## Format code
	@echo "Formatting Rust code..."
	cargo fmt
	@echo "Formatting Python code..."
	black python/ tests/ examples/
	isort python/ tests/ examples/

clean: ## Clean build artifacts
	@echo "Cleaning build artifacts..."
	cargo clean
	rm -rf target/
	rm -rf python/ultrafast_client.egg-info/
	rm -rf build/
	rm -rf dist/
	find . -type d -name __pycache__ -exec rm -rf {} +
	find . -name "*.pyc" -delete

docs: ## Generate documentation
	@echo "Generating documentation..."
	cargo doc --open
	cd docs && make html

benchmark: ## Run benchmarks
	@echo "Running Rust benchmarks..."
	cargo bench
	@echo "Running Python benchmarks..."
	python benchmarks/run_benchmarks.py

release-build: ## Build for release
	@echo "Building for release..."
	maturin build --release

build-all: ## Build all distribution packages
	@echo "📦 Building all distribution packages..."
	rm -rf target/wheels/*
	maturin build --release --strip
	maturin sdist
	@echo "✅ Built packages:"
	@ls -la target/wheels/

publish-test: ## Publish to test PyPI
	@echo "🧪 Publishing to Test PyPI..."
	$(MAKE) build-all
	python -m twine upload --repository testpypi target/wheels/*
	@echo "✅ Published to Test PyPI"
	@echo "💡 Test with: pip install --index-url https://test.pypi.org/simple/ ultrafast-client"

publish: ## Publish to PyPI
	@echo "🚀 Publishing to PyPI..."
	$(MAKE) build-all
	python -m twine upload target/wheels/*
	@echo "✅ Published to PyPI"
	@echo "💡 Install with: pip install ultrafast-client"
	@echo "💡 Install with uv: uv add ultrafast-client"

publish-all: ## Publish to both Test PyPI and PyPI
	@echo "🎉 Publishing to both repositories..."
	$(MAKE) publish-test
	$(MAKE) publish
	@echo "🎉 Published to both Test PyPI and PyPI!"

check-publish: ## Check package for PyPI upload
	@echo "🔍 Checking package for PyPI upload..."
	python -m twine check target/wheels/*

install-from-pypi: ## Test installation from PyPI
	@echo "🧪 Testing installation from PyPI..."
	pip install ultrafast-client
	python -c "import ultrafast_client; print(f'✅ Installed v{ultrafast_client.__version__}')"

install-from-test-pypi: ## Test installation from Test PyPI
	@echo "🧪 Testing installation from Test PyPI..."
	pip install --index-url https://test.pypi.org/simple/ ultrafast-client
	python -c "import ultrafast_client; print(f'✅ Installed v{ultrafast_client.__version__}')"

check: ## Run all checks (format, lint, test)
	@echo "Running all checks..."
	make format
	make lint
	make test

setup: ## Initial project setup
	@echo "Setting up development environment..."
	@echo "Installing Rust..."
	curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
	@echo "Installing Python dependencies..."
	pip install -U pip maturin
	pip install -e ".[dev]"
	@echo "Installing pre-commit hooks..."
	pre-commit install
	@echo "Building development version..."
	maturin develop
	@echo "Setup complete! Try running: python examples/basic_usage.py"

ci: ## Run CI checks
	@echo "Running CI checks..."
	cargo fmt --check
	cargo clippy -- -D warnings
	cargo test
	black --check python/ tests/ examples/
	isort --check python/ tests/ examples/
	ruff check python/ tests/ examples/
	mypy python/ultrafast_client/
	pytest tests/ -v

watch: ## Watch for changes and rebuild
	@echo "Watching for changes..."
	cargo watch -x 'build'

example: ## Run basic example
	python examples/basic_usage.py

profile: ## Profile the library
	@echo "Profiling..."
	cargo build --release
	python benchmarks/profile.py
