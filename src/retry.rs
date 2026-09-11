use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorClassification {
    RetryableRateLimit { retry_after_ms: Option<u64> },
    RetryableServerError { status: u16 },
    RetryableNetworkError { message: String },
    PermanentAuthError { status: u16, message: String },
    PermanentClientError { status: u16, message: String },
    Unknown { status: u16 },
}

impl ErrorClassification {
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            ErrorClassification::RetryableRateLimit { .. }
                | ErrorClassification::RetryableServerError { .. }
                | ErrorClassification::RetryableNetworkError { .. }
        )
    }

    pub fn is_permanent_auth(&self) -> bool {
        matches!(self, ErrorClassification::PermanentAuthError { .. })
    }
}

pub fn parse_retry_after(header_val: Option<&str>, cap_ms: u64) -> Option<u64> {
    let val = header_val?.trim();
    if let Ok(seconds) = val.parse::<u64>() {
        let ms = seconds.saturating_mul(1000);
        Some(ms.min(cap_ms))
    } else {
        None
    }
}

pub fn classify_upstream_response(
    status: u16,
    headers: &HashMap<String, String>,
    cap_ms: u64,
) -> ErrorClassification {
    match status {
        429 => {
            let retry_after_header = headers.get("retry-after").or_else(|| headers.get("Retry-After"));
            let retry_after_ms = parse_retry_after(retry_after_header.map(|s| s.as_str()), cap_ms);
            ErrorClassification::RetryableRateLimit { retry_after_ms }
        }
        500 | 502 | 503 | 504 => ErrorClassification::RetryableServerError { status },
        401 | 403 => ErrorClassification::PermanentAuthError {
            status,
            message: "Authentication failure from upstream Gemini API".to_string(),
        },
        400 | 404 | 413 | 422 => ErrorClassification::PermanentClientError {
            status,
            message: "Client request error from upstream".to_string(),
        },
        _ => ErrorClassification::Unknown { status },
    }
}

#[derive(Debug, Clone)]
pub struct FirstByteGuard {
    pub request_started: bool,
    pub upstream_headers_received: bool,
    pub first_byte_sent: bool,
    pub stream_started: bool,
}

impl Default for FirstByteGuard {
    fn default() -> Self {
        Self {
            request_started: true,
            upstream_headers_received: false,
            first_byte_sent: false,
            stream_started: false,
        }
    }
}

impl FirstByteGuard {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn mark_headers_received(&mut self) {
        self.upstream_headers_received = true;
    }

    pub fn mark_stream_started(&mut self) {
        self.stream_started = true;
        self.first_byte_sent = true;
    }

    pub fn can_retry(&self) -> bool {
        !self.first_byte_sent
    }
}
