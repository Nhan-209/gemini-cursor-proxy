use serde::Serialize;
use std::fmt;

#[derive(Debug, Clone, Serialize)]
pub struct OpenAiErrorDetail {
    pub message: String,
    #[serde(rename = "type")]
    pub error_type: String,
    pub param: Option<String>,
    pub code: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenAiErrorResponse {
    pub error: OpenAiErrorDetail,
}

#[derive(Debug)]
pub enum GatewayError {
    Unauthorized(String),
    PayloadTooLarge { max_bytes: usize, received_bytes: usize },
    BadRequest(String),
    NotFound(String),
    AllAccountsExhausted(String),
    UpstreamError { status: u16, message: String },
    StreamAborted(String),
    Internal(String),
    ConfigError(String),
}

impl fmt::Display for GatewayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GatewayError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
            GatewayError::PayloadTooLarge { max_bytes, received_bytes } => {
                write!(f, "Payload too large: received {} bytes, max allowed is {}", received_bytes, max_bytes)
            }
            GatewayError::BadRequest(msg) => write!(f, "Bad request: {}", msg),
            GatewayError::NotFound(msg) => write!(f, "Not found: {}", msg),
            GatewayError::AllAccountsExhausted(msg) => write!(f, "All upstream accounts exhausted: {}", msg),
            GatewayError::UpstreamError { status, message } => {
                write!(f, "Upstream returned HTTP {}: {}", status, message)
            }
            GatewayError::StreamAborted(msg) => write!(f, "Stream aborted: {}", msg),
            GatewayError::Internal(msg) => write!(f, "Internal server error: {}", msg),
            GatewayError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
        }
    }
}

impl std::error::Error for GatewayError {}

impl GatewayError {
    pub fn status_code(&self) -> u16 {
        match self {
            GatewayError::Unauthorized(_) => 401,
            GatewayError::PayloadTooLarge { .. } => 413,
            GatewayError::BadRequest(_) => 400,
            GatewayError::NotFound(_) => 404,
            GatewayError::AllAccountsExhausted(_) => 503,
            GatewayError::UpstreamError { status, .. } => *status,
            GatewayError::StreamAborted(_) => 502,
            GatewayError::Internal(_) => 500,
            GatewayError::ConfigError(_) => 500,
        }
    }

    pub fn to_openai_response(&self) -> OpenAiErrorResponse {
        let (error_type, code) = match self {
            GatewayError::Unauthorized(_) => ("invalid_request_error", Some("invalid_api_key".to_string())),
            GatewayError::PayloadTooLarge { .. } => ("invalid_request_error", Some("payload_too_large".to_string())),
            GatewayError::BadRequest(_) => ("invalid_request_error", Some("bad_request".to_string())),
            GatewayError::NotFound(_) => ("invalid_request_error", Some("not_found".to_string())),
            GatewayError::AllAccountsExhausted(_) => ("server_error", Some("upstream_unavailable".to_string())),
            GatewayError::UpstreamError { status, .. } => ("upstream_error", Some(format!("http_{}", status))),
            GatewayError::StreamAborted(_) => ("stream_error", Some("stream_aborted".to_string())),
            GatewayError::Internal(_) | GatewayError::ConfigError(_) => ("server_error", Some("internal_error".to_string())),
        };

        OpenAiErrorResponse {
            error: OpenAiErrorDetail {
                message: self.to_string(),
                error_type: error_type.to_string(),
                param: None,
                code,
            },
        }
    }

    pub fn to_worker_response(&self) -> worker::Result<worker::Response> {
        let status = self.status_code();
        let payload = self.to_openai_response();
        let json_body = serde_json::to_string(&payload)
            .unwrap_or_else(|_| "{\"error\":{\"message\":\"Internal error formatting error response\"}}".to_string());

        let headers = worker::Headers::from_iter([
            ("Content-Type", "application/json"),
            ("Access-Control-Allow-Origin", "*"),
            ("Access-Control-Allow-Methods", "GET, POST, OPTIONS"),
            ("Access-Control-Allow-Headers", "Content-Type, Authorization"),
        ]);

        let response = worker::Response::builder()
            .with_status(status)
            .with_headers(headers)
            .from_bytes(json_body.into_bytes())?;

        Ok(response)
    }
}
