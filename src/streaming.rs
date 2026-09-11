use crate::error::GatewayError;
use worker::{Headers, Response};

pub fn create_sse_response(mut upstream_res: Response) -> Result<Response, GatewayError> {
    // Obtain the raw ByteStream from the upstream Gemini response without buffering in memory
    let stream = upstream_res
        .stream()
        .map_err(|e| GatewayError::Internal(format!("Failed to acquire upstream byte stream: {}", e)))?;

    let mut response = Response::from_stream(stream)
        .map_err(|e| GatewayError::Internal(format!("Failed to create streaming response: {}", e)))?;

    // Set streaming and CORS headers
    let headers = response.headers_mut();
    headers
        .set("Content-Type", "text/event-stream")
        .map_err(|e| GatewayError::Internal(e.to_string()))?;
    headers
        .set("Cache-Control", "no-cache, no-transform")
        .map_err(|e| GatewayError::Internal(e.to_string()))?;
    headers
        .set("Connection", "keep-alive")
        .map_err(|e| GatewayError::Internal(e.to_string()))?;
    headers
        .set("X-Accel-Buffering", "no")
        .map_err(|e| GatewayError::Internal(e.to_string()))?;
    headers
        .set("Access-Control-Allow-Origin", "*")
        .map_err(|e| GatewayError::Internal(e.to_string()))?;

    Ok(response)
}

pub fn create_json_forward_response(mut upstream_res: Response) -> Result<Response, GatewayError> {
    let stream = upstream_res
        .stream()
        .map_err(|e| GatewayError::Internal(format!("Failed to acquire response stream: {}", e)))?;

    let mut response = Response::from_stream(stream)
        .map_err(|e| GatewayError::Internal(format!("Failed to forward JSON response: {}", e)))?;

    let headers = response.headers_mut();
    headers
        .set("Content-Type", "application/json")
        .map_err(|e| GatewayError::Internal(e.to_string()))?;
    headers
        .set("Access-Control-Allow-Origin", "*")
        .map_err(|e| GatewayError::Internal(e.to_string()))?;

    Ok(response)
}

pub fn create_cors_preflight_response() -> Result<Response, GatewayError> {
    let headers = Headers::from_iter([
        ("Access-Control-Allow-Origin", "*"),
        ("Access-Control-Allow-Methods", "GET, POST, OPTIONS"),
        ("Access-Control-Allow-Headers", "Content-Type, Authorization, X-Requested-With"),
        ("Access-Control-Max-Age", "86400"),
    ]);

    let response = Response::builder()
        .with_status(204)
        .with_headers(headers)
        .empty();

    Ok(response)
}
