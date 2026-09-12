use gemini_openai_gateway::retry::{
    classify_upstream_response, parse_retry_after, FirstByteGuard,
};
use std::collections::HashMap;

#[test]
fn test_classify_retryable_statuses() {
    let headers = HashMap::new();
    let cap_ms = 10_000;

    assert!(classify_upstream_response(429, &headers, cap_ms).is_retryable());
    assert!(classify_upstream_response(500, &headers, cap_ms).is_retryable());
    assert!(classify_upstream_response(502, &headers, cap_ms).is_retryable());
    assert!(classify_upstream_response(503, &headers, cap_ms).is_retryable());
    assert!(classify_upstream_response(504, &headers, cap_ms).is_retryable());
}

#[test]
fn test_classify_permanent_errors() {
    let headers = HashMap::new();
    let cap_ms = 10_000;

    let auth_401 = classify_upstream_response(401, &headers, cap_ms);
    assert!(!auth_401.is_retryable());
    assert!(auth_401.is_permanent_auth());

    let auth_403 = classify_upstream_response(403, &headers, cap_ms);
    assert!(!auth_403.is_retryable());
    assert!(auth_403.is_permanent_auth());

    let client_400 = classify_upstream_response(400, &headers, cap_ms);
    assert!(!client_400.is_retryable());
}

#[test]
fn test_parse_retry_after_header_and_capping() {
    // 5 seconds -> 5000ms
    assert_eq!(parse_retry_after(Some("5"), 10_000), Some(5000));

    // 30 seconds -> capped at 10,000ms
    assert_eq!(parse_retry_after(Some("30"), 10_000), Some(10_000));

    // Invalid header
    assert_eq!(parse_retry_after(Some("invalid"), 10_000), None);
    assert_eq!(parse_retry_after(None, 10_000), None);
}

#[test]
fn test_first_byte_guard_semantics() {
    let mut guard = FirstByteGuard::new();

    // Before any communication: retry allowed
    assert!(guard.can_retry());

    // When headers arrive: retry still allowed if status failed
    guard.mark_headers_received();
    assert!(guard.can_retry());

    // Once streaming begins: FIRST BYTE SENT!
    guard.mark_stream_started();
    assert!(!guard.can_retry()); // STRICTLY FORBIDDEN TO RETRY OR REPLAY!
}
