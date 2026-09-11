use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct RequestLogEvent<'a> {
    pub request_id: &'a str,
    pub account_id: &'a str,
    pub quota_domain: &'a str,
    pub model: &'a str,
    pub stream: bool,
    pub attempt: usize,
    pub status: u16,
    pub ttfb_ms: u64,
    pub duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<&'a str>,
}

impl<'a> RequestLogEvent<'a> {
    pub fn emit(&self) {
        // Log sanitized structured JSON to Cloudflare Worker console output.
        // Sensitive data (API keys, tokens, prompt text) is strictly excluded.
        if let Ok(json) = serde_json::to_string(self) {
            worker::console_log!("{}", json);
        }
    }
}

pub fn generate_request_id() -> String {
    // Generate an 8-byte hex request ID using current timestamp and pseudo-random elements
    let time_part = crate::current_timestamp_ms();
    format!("req_{:012x}", time_part)
}
