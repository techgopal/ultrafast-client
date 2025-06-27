#!/usr/bin/env python3
"""
Publishing script for UltraFast HTTP Client to PyPI, pip, and uv.
Supports both test and production publishing.
"""

import argparse
import os
import subprocess
import sys


def run_command(cmd, check=True):
    """Run a command and optionally check for errors."""
    print(f"🔧 Running: {cmd}")
    result = subprocess.run(cmd, shell=True, capture_output=True, text=True)

    if result.stdout:
        print(result.stdout)
    if result.stderr:
        print(result.stderr, file=sys.stderr)

    if check and result.returncode != 0:
        print(f"❌ Command failed with exit code {result.returncode}")
        sys.exit(1)

    return result


def build_packages():
    """Build both wheels and source distribution."""
    print("📦 Building packages...")

    # Clean previous builds
    run_command("rm -rf target/wheels/*")

    # Build wheels
    run_command("maturin build --release --strip")

    # Build source distribution
    run_command("maturin sdist")

    # List built packages
    print("📋 Built packages:")
    run_command("ls -la target/wheels/")


def publish_to_test_pypi():
    """Publish to Test PyPI."""
    print("🧪 Publishing to Test PyPI...")
    run_command("twine upload --repository testpypi target/wheels/*")


def publish_to_pypi():
    """Publish to PyPI."""
    print("🚀 Publishing to PyPI...")
    run_command("twine upload target/wheels/*")


def test_installation(repository="pypi"):
    """Test installation from the specified repository."""
    print(f"🧪 Testing installation from {repository}...")

    # Create a temporary virtual environment
    run_command("python -m venv test_env")

    if repository == "testpypi":
        install_cmd = "test_env/bin/pip install --index-url https://test.pypi.org/simple/ ultrafast-client"
    else:
        install_cmd = "test_env/bin/pip install ultrafast-client"

    run_command(install_cmd)

    # Test the installation
    test_script = """
import ultrafast_client
print(f"✅ Successfully installed ultrafast-client v{ultrafast_client.__version__}")

# Test basic functionality
client = ultrafast_client.HttpClient()
print("✅ HttpClient created successfully")

async_client = ultrafast_client.AsyncHttpClient()
print("✅ AsyncHttpClient created successfully")

session = ultrafast_client.Session()
print("✅ Session created successfully")

print("🎉 Installation test passed!")
"""

    run_command(f"test_env/bin/python -c '{test_script}'")

    # Clean up
    run_command("rm -rf test_env")


def setup_uv_support():
    """Set up support for uv package manager."""
    print("📦 Setting up uv support...")

    # uv automatically works with PyPI packages, but we can optimize pyproject.toml
    print("✅ uv support is automatic via PyPI publication")
    print("💡 Users can install with: uv add ultrafast-client")


def main():
    parser = argparse.ArgumentParser(description="Publish UltraFast HTTP Client")
    parser.add_argument(
        "--test", action="store_true", help="Publish to Test PyPI instead of PyPI"
    )
    parser.add_argument(
        "--build-only", action="store_true", help="Only build packages, don't publish"
    )
    parser.add_argument(
        "--test-install", action="store_true", help="Test installation after publishing"
    )

    args = parser.parse_args()

    print("🚀 UltraFast HTTP Client Publishing Script")
    print("=" * 50)

    # Always build packages first
    build_packages()

    if args.build_only:
        print("✅ Build complete. Packages ready in target/wheels/")
        return

    # Check for required environment variables
    if not os.getenv("TWINE_PASSWORD") and not os.getenv("PYPI_API_TOKEN"):
        print("❌ Missing TWINE_PASSWORD or PYPI_API_TOKEN environment variable")
        print("💡 Set one of these with your PyPI API token")
        sys.exit(1)

    # Publish to the appropriate repository
    if args.test:
        publish_to_test_pypi()
        if args.test_install:
            test_installation("testpypi")
    else:
        publish_to_pypi()
        if args.test_install:
            test_installation("pypi")

    # Set up uv support
    setup_uv_support()

    print("🎉 Publishing complete!")
    print("\n📋 Installation instructions:")
    print("  pip install ultrafast-client")
    print("  uv add ultrafast-client")
    print(
        "  conda install -c conda-forge ultrafast-client  # (if conda package is created)"
    )


if __name__ == "__main__":
    main()
