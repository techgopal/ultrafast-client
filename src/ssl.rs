use pyo3::prelude::*;
use std::path::Path;
use std::fs;
use std::collections::HashSet;
use crate::error::UltraFastError;

/// SSL/TLS configuration for the HTTP client
#[pyclass]
#[derive(Clone)]
pub struct SSLConfig {
    #[pyo3(get)]
    pub verify_certificates: bool,
    #[pyo3(get)]
    pub ca_bundle: Option<String>,
    #[pyo3(get)]
    pub client_cert: Option<String>,
    #[pyo3(get)]
    pub client_key: Option<String>,
    #[pyo3(get)]
    pub min_tls_version: Option<String>,
    #[pyo3(get)]
    pub ciphers: Option<Vec<String>>,
    #[pyo3(get)]
    pub check_revocation: bool,
    #[pyo3(get)]
    pub pinned_certs: Option<HashSet<String>>,
    #[pyo3(get)]
    pub ocsp_stapling: bool,
    #[pyo3(get)]
    pub cert_transparency: bool,
    #[pyo3(get)]
    pub max_cert_chain_depth: Option<u32>,
}

#[pymethods]
impl SSLConfig {
    #[new]
    #[pyo3(signature = (
        verify_certificates = true,
        ca_bundle = None,
        client_cert = None,
        client_key = None,
        min_tls_version = None,
        ciphers = None,
        check_revocation = false,
        pinned_certs = None,
        ocsp_stapling = false,
        cert_transparency = false,
        max_cert_chain_depth = None
    ))]
    pub fn new(
        verify_certificates: bool,
        ca_bundle: Option<String>,
        client_cert: Option<String>,
        client_key: Option<String>,
        min_tls_version: Option<String>,
        ciphers: Option<Vec<String>>,
        check_revocation: bool,
        pinned_certs: Option<HashSet<String>>,
        ocsp_stapling: bool,
        cert_transparency: bool,
        max_cert_chain_depth: Option<u32>,
    ) -> Self {
        SSLConfig {
            verify_certificates,
            ca_bundle,
            client_cert,
            client_key,
            min_tls_version,
            ciphers,
            check_revocation,
            pinned_certs,
            ocsp_stapling,
            cert_transparency,
            max_cert_chain_depth,
        }
    }

    /// Validate the SSL configuration
    pub fn validate(&self) -> PyResult<()> {
        // Validate TLS version if specified
        if let Some(version) = &self.min_tls_version {
            match version.as_str() {
                "1.0" | "1.1" | "1.2" | "1.3" => {},
                _ => return Err(pyo3::exceptions::PyValueError::new_err(
                    format!("Invalid TLS version: {}", version)
                )),
            }
        }

        // Validate client certificate and key pair
        if self.client_cert.is_some() != self.client_key.is_some() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "Both client certificate and key must be provided together"
            ));
        }

        // Validate certificate files exist
        if let Some(cert_path) = &self.client_cert {
            if !Path::new(cert_path).exists() {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    format!("Client certificate file not found: {}", cert_path)
                ));
            }
        }

        if let Some(key_path) = &self.client_key {
            if !Path::new(key_path).exists() {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    format!("Client key file not found: {}", key_path)
                ));
            }
        }

        // Validate CA bundle if specified
        if let Some(ca_path) = &self.ca_bundle {
            if !Path::new(ca_path).exists() {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    format!("CA bundle file not found: {}", ca_path)
                ));
            }
        }

        // Validate certificate chain depth
        if let Some(depth) = self.max_cert_chain_depth {
            if depth == 0 {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "Certificate chain depth must be greater than 0"
                ));
            }
        }

        Ok(())
    }

    /// Set SSL verification
    pub fn set_verify_ssl(&mut self, verify: bool) {
        self.verify_certificates = verify;
    }

    /// Set CA bundle path
    pub fn set_ca_bundle(&mut self, path: Option<String>) -> PyResult<()> {
        if let Some(ref ca_path) = path {
            if !Path::new(ca_path).exists() {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    format!("CA bundle file not found: {}", ca_path)
                ));
            }
        }
        self.ca_bundle = path;
        Ok(())
    }

    /// Set client certificate and key
    pub fn set_client_cert(&mut self, cert_path: Option<String>, key_path: Option<String>) -> PyResult<()> {
        if cert_path.is_some() != key_path.is_some() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "Both client certificate and key must be provided together"
            ));
        }

        if let Some(ref cert) = cert_path {
            if !Path::new(cert).exists() {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    format!("Client certificate file not found: {}", cert)
                ));
            }
        }

        if let Some(ref key) = key_path {
            if !Path::new(key).exists() {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    format!("Client key file not found: {}", key)
                ));
            }
        }

        self.client_cert = cert_path;
        self.client_key = key_path;
        Ok(())
    }

    /// Set minimum TLS version
    pub fn set_min_tls_version(&mut self, version: Option<String>) -> PyResult<()> {
        if let Some(ref ver) = version {
            match ver.as_str() {
                "1.0" | "1.1" | "1.2" | "1.3" => {},
                _ => return Err(pyo3::exceptions::PyValueError::new_err(
                    format!("Invalid TLS version: {}", ver)
                )),
            }
        }
        self.min_tls_version = version;
        Ok(())
    }

    /// Set cipher suites
    pub fn set_ciphers(&mut self, ciphers: Option<Vec<String>>) {
        self.ciphers = ciphers;
    }

    /// Set certificate revocation checking
    pub fn set_check_revocation(&mut self, check: bool) {
        self.check_revocation = check;
    }

    /// Set pinned certificates
    pub fn set_pinned_certs(&mut self, certs: Option<HashSet<String>>) {
        self.pinned_certs = certs;
    }

    /// Set OCSP stapling
    pub fn set_ocsp_stapling(&mut self, enable: bool) {
        self.ocsp_stapling = enable;
    }

    /// Set certificate transparency
    pub fn set_cert_transparency(&mut self, enable: bool) {
        self.cert_transparency = enable;
    }

    /// Set maximum certificate chain depth
    pub fn set_max_cert_chain_depth(&mut self, depth: Option<u32>) -> PyResult<()> {
        if let Some(d) = depth {
            if d == 0 {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "Certificate chain depth must be greater than 0"
                ));
            }
        }
        self.max_cert_chain_depth = depth;
        Ok(())
    }

    /// Build a reqwest ClientConfig from this SSL configuration
    pub fn build_client_config(&self) -> Result<reqwest::ClientConfig, UltraFastError> {
        let mut config = reqwest::ClientConfig::new();

        // Configure certificate verification
        if !self.verify_certificates {
            config = config.danger_accept_invalid_certs(true);
        }

        // Configure CA bundle if specified
        if let Some(ca_path) = &self.ca_bundle {
            let ca_data = fs::read_to_string(ca_path)
                .map_err(|e| UltraFastError::SslError(format!("Failed to read CA bundle {}: {}", ca_path, e)))?;
            config = config.add_root_certificate(
                reqwest::Certificate::from_pem(ca_data.as_bytes())
                    .map_err(|e| UltraFastError::SslError(format!("Failed to parse CA bundle: {}", e)))?
            );
        }

        // Configure client certificate if specified
        if let (Some(cert_path), Some(key_path)) = (&self.client_cert, &self.client_key) {
            let cert_data = fs::read_to_string(cert_path)
                .map_err(|e| UltraFastError::SslError(format!("Failed to read client certificate {}: {}", cert_path, e)))?;
            let key_data = fs::read_to_string(key_path)
                .map_err(|e| UltraFastError::SslError(format!("Failed to read client key {}: {}", key_path, e)))?;
            
            config = config.identity(
                reqwest::Identity::from_pem(
                    format!("{}\n{}", cert_data, key_data).as_bytes()
                ).map_err(|e| UltraFastError::SslError(format!("Failed to parse client certificate/key: {}", e)))?
            );
        }

        // Configure TLS version if specified
        if let Some(version) = &self.min_tls_version {
            config = config.min_tls_version(match version.as_str() {
                "1.0" => reqwest::tls::Version::TLS_1_0,
                "1.1" => reqwest::tls::Version::TLS_1_1,
                "1.2" => reqwest::tls::Version::TLS_1_2,
                "1.3" => reqwest::tls::Version::TLS_1_3,
                _ => unreachable!(), // Already validated in validate()
            });
        }

        // Configure cipher suites if specified
        if let Some(ciphers) = &self.ciphers {
            config = config.ciphers(ciphers);
        }

        // Configure certificate revocation checking
        if self.check_revocation {
            config = config.check_revocation(true);
        }

        // Configure certificate pinning if specified
        if let Some(pinned_certs) = &self.pinned_certs {
            for cert in pinned_certs {
                config = config.add_pinned_certificate(
                    reqwest::Certificate::from_pem(cert.as_bytes())
                        .map_err(|e| UltraFastError::SslError(format!("Failed to parse pinned certificate: {}", e)))?
                );
            }
        }

        // Configure OCSP stapling
        if self.ocsp_stapling {
            config = config.ocsp_stapling(true);
        }

        // Configure certificate transparency
        if self.cert_transparency {
            config = config.cert_transparency(true);
        }

        // Configure maximum certificate chain depth
        if let Some(depth) = self.max_cert_chain_depth {
            config = config.max_cert_chain_depth(depth);
        }

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    fn create_temp_cert() -> NamedTempFile {
        let mut cert = NamedTempFile::new().unwrap();
        writeln!(cert, "-----BEGIN CERTIFICATE-----\nMIIB...\n-----END CERTIFICATE-----").unwrap();
        cert
    }

    fn create_temp_key() -> NamedTempFile {
        let mut key = NamedTempFile::new().unwrap();
        writeln!(key, "-----BEGIN PRIVATE KEY-----\nMIIB...\n-----END PRIVATE KEY-----").unwrap();
        key
    }

    #[test]
    fn test_ssl_config_default() {
        let config = SSLConfig::new(true, None, None, None, None, None, false, None, false, false, None);
        assert!(config.verify_certificates);
        assert!(config.ca_bundle.is_none());
        assert!(config.client_cert.is_none());
        assert!(config.client_key.is_none());
        assert!(config.min_tls_version.is_none());
        assert!(config.ciphers.is_none());
        assert!(!config.check_revocation);
        assert!(config.pinned_certs.is_none());
        assert!(!config.ocsp_stapling);
        assert!(!config.cert_transparency);
        assert!(config.max_cert_chain_depth.is_none());
    }

    #[test]
    fn test_ssl_config_validation() {
        let mut config = SSLConfig::new(true, None, None, None, None, None, false, None, false, false, None);
        
        // Test valid TLS version
        assert!(config.set_min_tls_version(Some("1.2".to_string())).is_ok());
        
        // Test invalid TLS version
        assert!(config.set_min_tls_version(Some("2.0".to_string())).is_err());
        
        // Test valid certificate chain depth
        assert!(config.set_max_cert_chain_depth(Some(5)).is_ok());
        
        // Test invalid certificate chain depth
        assert!(config.set_max_cert_chain_depth(Some(0)).is_err());
    }

    #[test]
    fn test_ssl_config_with_certificates() {
        let cert = create_temp_cert();
        let key = create_temp_key();
        
        let mut config = SSLConfig::new(
            true,
            None,
            Some(cert.path().to_str().unwrap().to_string()),
            Some(key.path().to_str().unwrap().to_string()),
            None,
            None,
            false,
            None,
            false,
            false,
            None
        );
        
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_ssl_config_tls_version() {
        let mut config = SSLConfig::new(true, None, None, None, None, None, false, None, false, false, None);
        
        // Test all valid TLS versions
        for version in &["1.0", "1.1", "1.2", "1.3"] {
            assert!(config.set_min_tls_version(Some(version.to_string())).is_ok());
        }
        
        // Test invalid TLS version
        assert!(config.set_min_tls_version(Some("2.0".to_string())).is_err());
    }

    #[test]
    fn test_ssl_config_certificate_pinning() {
        let mut pinned_certs = HashSet::new();
        pinned_certs.insert("-----BEGIN CERTIFICATE-----\nMIIB...\n-----END CERTIFICATE-----".to_string());
        
        let config = SSLConfig::new(
            true,
            None,
            None,
            None,
            None,
            None,
            false,
            Some(pinned_certs),
            false,
            false,
            None
        );
        
        assert!(config.pinned_certs.is_some());
        assert_eq!(config.pinned_certs.unwrap().len(), 1);
    }

    #[test]
    fn test_ssl_config_advanced_features() {
        let config = SSLConfig::new(
            true,
            None,
            None,
            None,
            Some("1.2".to_string()),
            None,
            true,
            None,
            true,
            true,
            Some(5)
        );
        
        assert!(config.check_revocation);
        assert!(config.ocsp_stapling);
        assert!(config.cert_transparency);
        assert_eq!(config.max_cert_chain_depth, Some(5));
    }
} 