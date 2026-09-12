use crate::config::UpstreamConfig;
use crate::error::GatewayError;
use worker::wasm_bindgen::JsValue;
use worker::{Fetch, Headers, Method, Request, RequestInit, Response};

pub struct UpstreamClient {
    pub base_url: String,
}

impl UpstreamClient {
    pub fn new(cfg: &UpstreamConfig) -> Self {
        Self {
            base_url: cfg.base_url.trim_end_matches('/').to_string(),
        }
    }

    pub fn resolve_api_key(
        env: &worker::Env,
        account: &crate::scheduler::AccountSnapshot,
    ) -> Result<String, GatewayError> {
        // 1. If account has a direct key from GEMINI_KEYS_POOL, use it directly
        if let Some(ref direct) = account.direct_key {
            let val = direct.trim().trim_matches('"').trim_matches('\'').trim();
            if !val.is_empty() && val != "replace_me" {
                return Ok(val.to_string());
            }
        }

        // 2. Look up secret in Cloudflare Secrets using secret_name
        if let Ok(secret) = env.secret(&account.secret_name) {
            let val = secret.to_string();
            let trimmed = val.trim().trim_matches('"').trim_matches('\'').trim();
            if !trimmed.is_empty() && trimmed != "replace_me" {
                return Ok(trimmed.to_string());
            }
        }
        // 3. Fallback to worker::Env vars (e.g. for .dev.vars in local development)
        if let Ok(var) = env.var(&account.secret_name) {
            let val = var.to_string();
            let trimmed = val.trim().trim_matches('"').trim_matches('\'').trim();
            if !trimmed.is_empty() && trimmed != "replace_me" {
                return Ok(trimmed.to_string());
            }
        }

        Err(GatewayError::Internal(format!(
            "Cloudflare Secret '{}' is missing or unconfigured in Worker runtime",
            account.secret_name
        )))
    }

    pub async fn send_chat_completion(
        &self,
        api_key: &str,
        body_json: &str,
    ) -> Result<Response, GatewayError> {
        let clean_key = api_key.trim().trim_matches('"').trim_matches('\'').trim();

        // Pass key via query param and standard headers for maximum upstream compatibility
        let endpoint_url = format!("{}/chat/completions?key={}", self.base_url, clean_key);

        let headers = Headers::new();
        headers
            .set("Content-Type", "application/json")
            .map_err(|e| GatewayError::Internal(e.to_string()))?;
        headers
            .set("Authorization", &format!("Bearer {}", clean_key))
            .map_err(|e| GatewayError::Internal(e.to_string()))?;
        headers
            .set("x-goog-api-key", clean_key)
            .map_err(|e| GatewayError::Internal(e.to_string()))?;
        headers
            .set("User-Agent", "gemini-openai-gateway/0.1.0")
            .map_err(|e| GatewayError::Internal(e.to_string()))?;

        let mut init = RequestInit::new();
        init.with_method(Method::Post);
        init.with_headers(headers);
        init.with_body(Some(JsValue::from_str(body_json)));

        let request = Request::new_with_init(&endpoint_url, &init)
            .map_err(|e| GatewayError::Internal(format!("Failed to construct upstream request: {}", e)))?;

        let response = Fetch::Request(request)
            .send()
            .await
            .map_err(|e| GatewayError::UpstreamError {
                status: 502,
                message: format!("Failed to reach upstream Gemini API: {}", e),
            })?;

        Ok(response)
    }
}
