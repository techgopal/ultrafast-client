use pyo3::exceptions::PyException;
use pyo3::PyErr;
use std::fmt;
use thiserror::Error;

/// Comprehensive error type for UltraFast HTTP client
#[derive(Error, Debug, Clone)]
pub enum UltraFastError {
    #[error("HTTP request failed: {0}")]
    HttpError(String),
    
    #[error("HTTP/3 error: {0}")]
    Http3Error(String),
    
    #[error("Protocol negotiation failed: {0}")]
    ProtocolError(String),
    
    #[error("Connection error: {0}")]
    ConnectionError(String),
    
    #[error("Timeout error: {0}")]
    TimeoutError(String),
    
    #[error("Authentication error: {0}")]
    AuthError(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("I/O error: {0}")]
    IoError(String),
    
    #[error("Generic client error: {0}")]
    ClientError(String),
    
    #[error("Rate limit exceeded: {0}")]
    RateLimitError(String),
    
    #[error("Rate limit queue full: {0}")]
    RateLimitQueueFullError(String),
}

impl From<UltraFastError> for PyErr {
    fn from(err: UltraFastError) -> PyErr {
        match err {
            UltraFastError::TimeoutError(msg) => {
                pyo3::exceptions::PyTimeoutError::new_err(msg)
            }
            UltraFastError::ConnectionError(msg) | UltraFastError::Http3Error(msg) => {
                pyo3::exceptions::PyConnectionError::new_err(msg)
            }
            UltraFastError::AuthError(msg) => {
                pyo3::exceptions::PyPermissionError::new_err(msg)
            }
            UltraFastError::ConfigError(msg) | UltraFastError::ProtocolError(msg) => {
                pyo3::exceptions::PyValueError::new_err(msg)
            }
            UltraFastError::SerializationError(msg) | UltraFastError::IoError(msg) => {
                pyo3::exceptions::PyIOError::new_err(msg)
            }
            UltraFastError::RateLimitError(msg) | UltraFastError::RateLimitQueueFullError(msg) => {
                pyo3::exceptions::PyRuntimeError::new_err(format!("Rate limit error: {}", msg))
            }
            _ => {
                pyo3::exceptions::PyRuntimeError::new_err(err.to_string())
            }
        }
    }
}

impl From<reqwest::Error> for UltraFastError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            UltraFastError::TimeoutError(err.to_string())
        } else if err.is_connect() {
            UltraFastError::ConnectionError(err.to_string())
        } else {
            UltraFastError::HttpError(err.to_string())
        }
    }
}

impl From<std::io::Error> for UltraFastError {
    fn from(err: std::io::Error) -> Self {
        UltraFastError::IoError(err.to_string())
    }
}

/// Legacy error type for backwards compatibility
#[derive(Debug, Clone)]
pub struct ClientError {
    pub message: String,
    pub code: Option<String>,
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ClientError: {}", self.message)
    }
}

impl std::error::Error for ClientError {}

impl From<ClientError> for PyErr {
    fn from(err: ClientError) -> PyErr {
        PyException::new_err(err.message)
    }
}

impl From<ClientError> for UltraFastError {
    fn from(err: ClientError) -> Self {
        UltraFastError::ClientError(err.message)
    }
}

/// Result type for client operations
pub type ClientResult<T> = Result<T, ClientError>;

/// Map reqwest errors to PyO3 exceptions
pub fn map_reqwest_error(error: &reqwest::Error) -> PyErr {
    if error.is_timeout() {
        pyo3::exceptions::PyTimeoutError::new_err(format!("Request timeout: {}", error))
    } else if error.is_connect() {
        pyo3::exceptions::PyConnectionError::new_err(format!("Connection failed: {}", error))
    } else if error.is_request() {
        pyo3::exceptions::PyIOError::new_err(format!("Request error: {}", error))
    } else if error.is_decode() {
        pyo3::exceptions::PyValueError::new_err(format!("Decode error: {}", error))
    } else {
        pyo3::exceptions::PyRuntimeError::new_err(format!("HTTP error: {}", error))
    }
}
