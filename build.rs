use pyo3_build_config::PythonVersion;

fn main() {
    // Always configure for Python extension module
    let config = pyo3_build_config::get();
    
    // Print interpreter configuration for debugging
    println!("cargo:warning=Python version: {:?}", config.version);
    println!("cargo:warning=Python executable: {:?}", config.executable);
    
    // Handle macOS Python symbol linking
    // On macOS, Python extension modules need undefined dynamic lookup
    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-arg=-undefined");
        println!("cargo:rustc-link-arg=dynamic_lookup");
    }
    
    // Additional configuration for extension modules
    if config.version >= (PythonVersion { major: 3, minor: 8 }) {
        // Set proper linking flags for Python extension modules
        println!("cargo:rustc-cdylib-link-arg=-Wl,-install_name,@rpath/ultrafast_client.so");
    }
}
