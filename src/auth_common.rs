use crate::config::{AuthConfig, AuthType};
use reqwest::RequestBuilder;

/// Shared authentication logic for both sync and async clients
/// This eliminates code duplication between HttpClient and AsyncHttpClient
pub fn apply_authentication(
    mut request: RequestBuilder,
    auth_config: Option<&AuthConfig>,
) -> RequestBuilder {
    if let Some(auth) = auth_config {
        match auth.auth_type {
            AuthType::Bearer => {
                if let Some(token) = auth.get_credential("token") {
                    request = request.header("Authorization", format!("Bearer {}", token));
                }
            }
            AuthType::Basic => {
                if let Some(encoded) = auth.get_credential("encoded") {
                    request = request.header("Authorization", format!("Basic {}", encoded));
                }
            }
            AuthType::ApiKeyHeader => {
                if let (Some(key), Some(header_name)) = (
                    auth.get_credential("key"),
                    auth.get_credential("header_name"),
                ) {
                    request = request.header(&header_name, key);
                }
            }
            AuthType::ApiKeyQuery => {
                if let (Some(key), Some(param_name)) = (
                    auth.get_credential("key"),
                    auth.get_credential("param_name"),
                ) {
                    request = request.query(&[(param_name, key)]);
                }
            }
            AuthType::OAuth2 => {
                if let Some(token) = auth.get_credential("access_token") {
                    let token_type = auth
                        .get_credential("token_type")
                        .unwrap_or_else(|| "Bearer".to_string());
                    request = request.header("Authorization", format!("{} {}", token_type, token));
                }
            }
            AuthType::Custom => {
                if let Some(custom_type) = auth.get_credential("custom_type") {
                    match custom_type.as_str() {
                        "jwt" | "JWT" => {
                            if let (Some(token), Some(header)) =
                                (auth.get_credential("token"), auth.get_credential("header"))
                            {
                                let prefix = auth
                                    .get_credential("prefix")
                                    .unwrap_or_else(|| "Bearer".to_string());
                                request = request.header(&header, format!("{} {}", prefix, token));
                            }
                        }
                        _ => {
                            // Generic custom auth - apply all non-meta credentials as headers
                            for (key, value) in &auth.credentials {
                                if !key.starts_with("custom_") {
                                    request = request.header(key, value);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    request
}

#[cfg(test)]
mod tests {

    use crate::config::{AuthConfig, AuthType};

    #[test]
    fn test_bearer_auth() {
        let auth = AuthConfig::bearer("test-token".to_string());
        // This test would need a mock RequestBuilder to fully test
        // For now, just verify the auth config is created correctly
        assert_eq!(auth.auth_type, AuthType::Bearer);
        assert_eq!(auth.get_credential("token"), Some("test-token".to_string()));
    }

    #[test]
    fn test_basic_auth() {
        let auth = AuthConfig::basic("user".to_string(), "pass".to_string());
        assert_eq!(auth.auth_type, AuthType::Basic);
        // encoded credentials are stored internally
        assert!(auth.get_credential("encoded").is_some());
    }

    #[test]
    fn test_api_key_header() {
        let auth = AuthConfig::api_key_header("X-API-Key".to_string(), "secret-key".to_string());
        assert_eq!(auth.auth_type, AuthType::ApiKeyHeader);
        assert_eq!(auth.get_credential("key"), Some("secret-key".to_string()));
        assert_eq!(
            auth.get_credential("header_name"),
            Some("X-API-Key".to_string())
        );
    }

    #[test]
    fn test_api_key_query() {
        let auth = AuthConfig::api_key_query("api_key".to_string(), "secret-key".to_string());
        assert_eq!(auth.auth_type, AuthType::ApiKeyQuery);
        assert_eq!(auth.get_credential("key"), Some("secret-key".to_string()));
        assert_eq!(
            auth.get_credential("param_name"),
            Some("api_key".to_string())
        );
    }
}
