# UltraFast HTTP Client Makefile

.PHONY: help build dev test clean install lint format docs benchmark version-bump version-patch version-minor version-major publish-version check-version

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

# Version management and publishing commands
check-version: ## Check current version across all files
	@echo "🔍 Current versions:"
	@echo "Cargo.toml:     $(shell grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)"
	@echo "pyproject.toml: $(shell grep '^version' pyproject.toml | head -1 | cut -d'"' -f2)"
	@echo "Python __init__: $(shell grep '__version__' python/ultrafast_client/__init__.py | cut -d'"' -f2)"
	@echo "Git tags:       $(shell git tag --sort=-version:refname | head -3 | tr '\n' ' ')"

version-patch: ## Bump patch version (0.1.2 -> 0.1.3)
	@echo "🔧 Bumping patch version..."
	@$(MAKE) _version-bump TYPE=patch

version-minor: ## Bump minor version (0.1.2 -> 0.2.0)
	@echo "🔧 Bumping minor version..."
	@$(MAKE) _version-bump TYPE=minor

version-major: ## Bump major version (0.1.2 -> 1.0.0)
	@echo "🔧 Bumping major version..."
	@$(MAKE) _version-bump TYPE=major

_version-bump: ## Internal: Bump version and update all files
	@if [ -z "$(TYPE)" ]; then echo "❌ TYPE not specified"; exit 1; fi
	@echo "📝 Updating version files..."
	@python3 -c "\
import re, sys; \
current = open('Cargo.toml').read(); \
version_match = re.search(r'version = \"([^\"]+)\"', current); \
if not version_match: sys.exit('No version found'); \
old_version = version_match.group(1); \
parts = [int(x) for x in old_version.split('.')]; \
if '$(TYPE)' == 'patch': parts[2] += 1; \
elif '$(TYPE)' == 'minor': parts[1] += 1; parts[2] = 0; \
elif '$(TYPE)' == 'major': parts[0] += 1; parts[1] = 0; parts[2] = 0; \
new_version = '.'.join(map(str, parts)); \
print(f'Updating {old_version} -> {new_version}'); \
# Update Cargo.toml \
cargo_content = re.sub(r'version = \"[^\"]+\"', f'version = \"{new_version}\"', current, count=1); \
open('Cargo.toml', 'w').write(cargo_content); \
# Update pyproject.toml \
pyproject_content = open('pyproject.toml').read(); \
pyproject_content = re.sub(r'version = \"[^\"]+\"', f'version = \"{new_version}\"', pyproject_content, count=1); \
open('pyproject.toml', 'w').write(pyproject_content); \
# Update Python __init__.py \
init_content = open('python/ultrafast_client/__init__.py').read(); \
init_content = re.sub(r'__version__ = \"[^\"]+\"', f'__version__ = \"{new_version}\"', init_content); \
open('python/ultrafast_client/__init__.py', 'w').write(init_content); \
open('.new_version', 'w').write(new_version); \
"
	@NEW_VERSION=$$(cat .new_version); \
	echo "✅ Updated all files to version $$NEW_VERSION"; \
	rm .new_version

publish-version: ## Publish new version (bump patch, commit, tag, push)
	@echo "🚀 Publishing new version..."
	@$(MAKE) check-git-clean
	@$(MAKE) version-patch
	@NEW_VERSION=$$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2); \
	echo "📝 Committing version $$NEW_VERSION..."; \
	git add Cargo.toml pyproject.toml python/ultrafast_client/__init__.py; \
	git commit -m "bump: Update version to $$NEW_VERSION"; \
	echo "🏷️  Creating and pushing tag v$$NEW_VERSION..."; \
	git tag "v$$NEW_VERSION"; \
	git push origin main; \
	git push origin "v$$NEW_VERSION"; \
	echo "🎯 Version $$NEW_VERSION published!"; \
	echo "🔄 GitHub Actions will now:"; \
	echo "   1. Create GitHub release"; \
	echo "   2. Build and publish to PyPI"; \
	echo "🔍 Monitor at: https://github.com/techgopal/ultrafast-client/actions"

publish-version-minor: ## Publish new minor version (bump minor, commit, tag, push)
	@echo "🚀 Publishing new minor version..."
	@$(MAKE) check-git-clean
	@$(MAKE) version-minor
	@NEW_VERSION=$$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2); \
	echo "📝 Committing version $$NEW_VERSION..."; \
	git add Cargo.toml pyproject.toml python/ultrafast_client/__init__.py; \
	git commit -m "bump: Update version to $$NEW_VERSION"; \
	echo "🏷️  Creating and pushing tag v$$NEW_VERSION..."; \
	git tag "v$$NEW_VERSION"; \
	git push origin main; \
	git push origin "v$$NEW_VERSION"; \
	echo "🎯 Version $$NEW_VERSION published!"; \
	echo "🔄 GitHub Actions will now:"; \
	echo "   1. Create GitHub release"; \
	echo "   2. Build and publish to PyPI"; \
	echo "🔍 Monitor at: https://github.com/techgopal/ultrafast-client/actions"

publish-version-major: ## Publish new major version (bump major, commit, tag, push)
	@echo "🚀 Publishing new major version..."
	@$(MAKE) check-git-clean
	@$(MAKE) version-major
	@NEW_VERSION=$$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2); \
	echo "📝 Committing version $$NEW_VERSION..."; \
	git add Cargo.toml pyproject.toml python/ultrafast_client/__init__.py; \
	git commit -m "bump: Update version to $$NEW_VERSION"; \
	echo "🏷️  Creating and pushing tag v$$NEW_VERSION..."; \
	git tag "v$$NEW_VERSION"; \
	git push origin main; \
	git push origin "v$$NEW_VERSION"; \
	echo "🎯 Version $$NEW_VERSION published!"; \
	echo "🔄 GitHub Actions will now:"; \
	echo "   1. Create GitHub release"; \
	echo "   2. Build and publish to PyPI"; \
	echo "🔍 Monitor at: https://github.com/techgopal/ultrafast-client/actions"

check-git-clean: ## Check if git working directory is clean
	@if [ -n "$$(git status --porcelain)" ]; then \
		echo "❌ Git working directory is not clean. Please commit or stash changes first."; \
		git status --short; \
		exit 1; \
	fi
	@echo "✅ Git working directory is clean"

publish-hotfix: ## Publish hotfix version with custom message
	@echo "🔥 Publishing hotfix version..."
	@if [ -z "$(MSG)" ]; then echo "❌ Please provide MSG=your_message"; exit 1; fi
	@$(MAKE) check-git-clean
	@$(MAKE) version-patch
	@NEW_VERSION=$$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2); \
	echo "📝 Committing hotfix $$NEW_VERSION..."; \
	git add Cargo.toml pyproject.toml python/ultrafast_client/__init__.py; \
	git commit -m "hotfix: $(MSG) (v$$NEW_VERSION)"; \
	echo "🏷️  Creating and pushing tag v$$NEW_VERSION..."; \
	git tag "v$$NEW_VERSION"; \
	git push origin main; \
	git push origin "v$$NEW_VERSION"; \
	echo "🔥 Hotfix $$NEW_VERSION published!"; \
	echo "🔍 Monitor at: https://github.com/techgopal/ultrafast-client/actions"
