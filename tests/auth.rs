use gemini_openai_gateway::auth::{constant_time_eq, verify_bearer_token};

#[test]
fn test_valid_bearer_token() {
    let expected = "secret_proxy_token_12345";
    let header = "Bearer secret_proxy_token_12345";
    assert!(verify_bearer_token(Some(header), expected).is_ok());
}

#[test]
fn test_lowercase_bearer_token() {
    let expected = "secret_proxy_token_12345";
    let header = "bearer secret_proxy_token_12345";
    assert!(verify_bearer_token(Some(header), expected).is_ok());
}

#[test]
fn test_missing_auth_header() {
    let expected = "secret_proxy_token_12345";
    assert!(verify_bearer_token(None, expected).is_err());
    assert!(verify_bearer_token(Some(""), expected).is_err());
}

#[test]
fn test_invalid_bearer_token() {
    let expected = "secret_proxy_token_12345";
    let header = "Bearer wrong_token";
    let res = verify_bearer_token(Some(header), expected);
    assert!(res.is_err());
}

#[test]
fn test_invalid_auth_scheme() {
    let expected = "secret_proxy_token_12345";
    let header = "Basic dXNlcjpwYXNz";
    let res = verify_bearer_token(Some(header), expected);
    assert!(res.is_err());
}

#[test]
fn test_constant_time_equality() {
    assert!(constant_time_eq(b"hello", b"hello"));
    assert!(!constant_time_eq(b"hello", b"world"));
    assert!(!constant_time_eq(b"short", b"longer_string"));
}
