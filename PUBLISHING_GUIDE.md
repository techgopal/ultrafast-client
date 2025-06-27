# 📦 UltraFast HTTP Client - Publishing & CI/CD Guide

This guide covers the complete publishing pipeline for the UltraFast HTTP Client, including CI/CD automation, PyPI distribution, and package manager support.

## 🚀 Quick Publishing

### For Maintainers
```bash
# Build all packages
make build-all

# Publish to Test PyPI first
make publish-test

# Publish to PyPI
make publish

# Or publish to both at once
make publish-all
```

### Using the Publishing Script
```bash
# Build only
python scripts/publish.py --build-only

# Publish to Test PyPI
python scripts/publish.py --test

# Publish to PyPI (production)
python scripts/publish.py

# Publish and test installation
python scripts/publish.py --test-install
```

## 🔧 CI/CD Pipeline

### GitHub Actions Workflows

#### 1. Main CI/CD Pipeline (`.github/workflows/ci.yml`)
- **Triggers:** Push to main/develop, pull requests
- **Jobs:**
  - Lint and format checking (Rust + Python)
  - Multi-platform testing (Ubuntu, Windows, macOS)
  - Security audit with cargo-audit
  - Wheel building for multiple platforms
  - Source distribution creation
  - Integration testing

#### 2. PyPI Publishing (`.github/workflows/publish.yml`)
- **Triggers:** GitHub releases, manual dispatch
- **Features:**
  - Automated publishing to PyPI/Test PyPI
  - Multi-platform wheel building
  - Installation testing post-publish
  - GitHub release asset upload

#### 3. Release Creation (`.github/workflows/release.yml`)
- **Triggers:** Git tags (`v*`), manual dispatch
- **Features:**
  - Automated GitHub release creation
  - Release notes generation
  - Multi-platform asset building
  - Changelog integration

### Setting Up CI/CD

1. **Repository Secrets** (Required for publishing):
   ```
   PYPI_API_TOKEN        # PyPI API token
   TEST_PYPI_API_TOKEN   # Test PyPI API token
   ```

2. **Environment Setup** (GitHub repository settings):
   - Create `release` environment
   - Add required reviewers for production deployments
   - Configure protection rules

## �� Package Distribution

### PyPI (Python Package Index)

#### Production Publishing
```bash
# Automated via GitHub Actions on release
git tag v0.1.0
git push origin v0.1.0

# Manual publishing
export TWINE_PASSWORD="your-pypi-token"
make publish
```

#### Test PyPI
```bash
# Test before production
export TWINE_PASSWORD="your-test-pypi-token"
make publish-test

# Test installation
pip install --index-url https://test.pypi.org/simple/ ultrafast-client
```

### pip (Package Manager)
Users can install via pip once published to PyPI:
```bash
# Standard installation
pip install ultrafast-client

# Upgrade to latest
pip install --upgrade ultrafast-client

# Install specific version
pip install ultrafast-client==0.1.0
```

### uv (Fast Python Package Manager)
Full support for uv package manager:
```bash
# Add to project
uv add ultrafast-client

# Install specific version
uv add ultrafast-client==0.1.0

# Install with extras
uv add "ultrafast-client[dev]"
```

## 🛠️ Manual Publishing Process

### Step 1: Build Packages
```bash
# Clean previous builds
rm -rf target/wheels/*

# Build optimized wheel
maturin build --release --strip

# Build source distribution
maturin sdist

# Verify packages
ls -la target/wheels/
```

### Step 2: Test Locally
```bash
# Install in fresh environment
python -m venv test_env
source test_env/bin/activate
pip install target/wheels/*.whl

# Test functionality
python -c "import ultrafast_client; print('✅ Works!')"
deactivate
rm -rf test_env
```

### Step 3: Publish
```bash
# Install publishing tools
pip install twine

# Check package
twine check target/wheels/*

# Upload to Test PyPI first
twine upload --repository testpypi target/wheels/*

# Test installation from Test PyPI
pip install --index-url https://test.pypi.org/simple/ ultrafast-client

# Upload to PyPI
twine upload target/wheels/*
```

## 🎉 Installation for Users

Once published, users can install the package using multiple methods:

### pip (Traditional)
```bash
pip install ultrafast-client
```

### uv (Modern & Fast)
```bash
uv add ultrafast-client
```

### pipx (Isolated)
```bash
pipx install ultrafast-client
```

### Poetry
```bash
poetry add ultrafast-client
```

### pipenv
```bash
pipenv install ultrafast-client
```

---

**Repository:** https://github.com/techgopal/ultrafast-client  
**PyPI:** https://pypi.org/project/ultrafast-client/ (when published)
