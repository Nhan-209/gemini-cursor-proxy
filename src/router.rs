use crate::auth::verify_bearer_token;
use crate::config::AppConfig;
use crate::error::GatewayError;
use crate::metrics::{generate_request_id, RequestLogEvent};
use crate::models::{build_model_list, normalize_request, ChatCompletionRequest};
use crate::retry::{classify_upstream_response, FirstByteGuard};
use crate::scheduler::{AccountScheduler, LruQuotaScheduler};
use crate::streaming::{create_cors_preflight_response, create_json_forward_response, create_sse_response};
use crate::upstream::UpstreamClient;
use std::collections::HashMap;
use std::sync::Arc;
use worker::{Headers, Method, Request, Response};

pub async fn handle_request(
    mut req: Request,
    env: worker::Env,
    config: &AppConfig,
    scheduler: &Arc<LruQuotaScheduler>,
) -> Result<Response, GatewayError> {
    let method = req.method();
    let path = req.path();

    // 1. Handle CORS Preflight
    if method == Method::Options {
        return create_cors_preflight_response();
    }

    // 2. Health check endpoint (public, unauthenticated)
    if (path == "/health" || path == "/") && method == Method::Get {
        let headers = Headers::from_iter([
            ("Content-Type", "application/json"),
            ("Access-Control-Allow-Origin", "*"),
        ]);
        let response = Response::builder()
            .with_status(200)
            .with_headers(headers)
            .from_bytes(
                r#"{"status":"healthy","service":"gemini-cursor-proxy","version":"0.1.0"}"#.as_bytes().to_vec(),
            )
            .map_err(|e| GatewayError::Internal(e.to_string()))?;
        return Ok(response);
    }

    // 3. Authenticate Bearer token for API endpoints
    if config.security.require_proxy_token {
        let proxy_token = env
            .secret("PROXY_TOKEN")
            .map(|s| s.to_string())
            .or_else(|_| env.var("PROXY_TOKEN").map(|v| v.to_string()))
            .map_err(|_| GatewayError::Internal("Cloudflare Secret 'PROXY_TOKEN' is not configured".to_string()))?;

        let auth_header = req.headers().get("authorization").ok().flatten();
        verify_bearer_token(auth_header.as_deref(), &proxy_token)?;
    }

    // 4. Dispatch routes
    let models_endpoint = format!("{}/models", config.server.base_path.trim_end_matches('/'));
    let chat_endpoint = format!("{}/chat/completions", config.server.base_path.trim_end_matches('/'));
    let responses_endpoint = format!("{}/responses", config.server.base_path.trim_end_matches('/'));

    if path == models_endpoint && method == Method::Get {
        handle_get_models(config)
    } else if (path == chat_endpoint || path == responses_endpoint) && method == Method::Post {
        handle_chat_completions(&mut req, &env, config, scheduler).await
    } else {
        Err(GatewayError::NotFound(format!("Route {} {} not found", method, path)))
    }
}

fn handle_get_models(config: &AppConfig) -> Result<Response, GatewayError> {
    let model_list = build_model_list(&config.model);
    let json_bytes = serde_json::to_vec(&model_list)
        .map_err(|e| GatewayError::Internal(format!("Failed to serialize model list: {}", e)))?;

    let headers = Headers::from_iter([
        ("Content-Type", "application/json"),
        ("Access-Control-Allow-Origin", "*"),
    ]);

    let response = Response::builder()
        .with_status(200)
        .with_headers(headers)
        .from_bytes(json_bytes)
        .map_err(|e| GatewayError::Internal(e.to_string()))?;

    Ok(response)
}

async fn handle_chat_completions(
    req: &mut Request,
    env: &worker::Env,
    config: &AppConfig,
    scheduler: &Arc<LruQuotaScheduler>,
) -> Result<Response, GatewayError> {
    let request_id = generate_request_id();
    let start_time = crate::current_timestamp_ms();

    // 1. Read request body once and enforce max_body_bytes limit
    let body_bytes = req
        .bytes()
        .await
        .map_err(|e| GatewayError::BadRequest(format!("Failed to read request body: {}", e)))?;

    if body_bytes.len() > config.server.max_body_bytes {
        return Err(GatewayError::PayloadTooLarge {
            max_bytes: config.server.max_body_bytes,
            received_bytes: body_bytes.len(),
        });
    }

    // 2. Parse payload
    let chat_req: ChatCompletionRequest = serde_json::from_slice(&body_bytes)
        .map_err(|e| GatewayError::BadRequest(format!("Invalid ChatCompletionRequest JSON: {}", e)))?;

    let is_streaming = chat_req.stream.unwrap_or(false);

    // 3. Smart Task Classification & Model Routing (Gemini 3.5 Flash-Lite Classifier)
    let normalized_req = if config.smart_router.enabled {
        let user_query = crate::classifier::SmartRouter::extract_user_query(&chat_req.messages);
        let classification = if !user_query.is_empty() {
            if let Ok(acc) = scheduler.select_account(crate::current_timestamp_ms()) {
                if let Ok(key) = UpstreamClient::resolve_api_key(env, &acc) {
                    let client = UpstreamClient::new(&config.upstream);
                    crate::classifier::SmartRouter::classify(
                        &user_query,
                        &client,
                        &key,
                        &config.smart_router.classifier_model,
                    )
                    .await
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        if let Some(ref res) = classification {
            if res.confidence >= config.smart_router.min_confidence {
                let mut req = chat_req;
                match res.route {
                    crate::classifier::TaskDifficulty::Easy => {
                        req.model = config.smart_router.lite_model.clone();
                        req.reasoning_effort = None;
                    }
                    crate::classifier::TaskDifficulty::Normal => {
                        req.model = config.smart_router.primary_model.clone();
                        req.reasoning_effort = Some("medium".to_string());
                    }
                    crate::classifier::TaskDifficulty::Hard => {
                        req.model = config.smart_router.primary_model.clone();
                        req.reasoning_effort = Some("high".to_string());
                    }
                }
                req
            } else {
                normalize_request(chat_req, &config.model)
            }
        } else {
            normalize_request(chat_req, &config.model)
        }
    } else {
        normalize_request(chat_req, &config.model)
    };

    let target_model = normalized_req.model.clone();

    // 4. Re-serialize payload once
    let serialized_payload = serde_json::to_string(&normalized_req)
        .map_err(|e| GatewayError::Internal(format!("Failed to serialize normalized request: {}", e)))?;

    let upstream_client = UpstreamClient::new(&config.upstream);
    let mut guard = FirstByteGuard::new();

    let max_attempts = config.retry.max_attempts.max(1);
    let mut last_error = GatewayError::Internal("No attempt made".to_string());

    // 5. Account scheduling & retry loop with first-byte semantics
    for attempt in 1..=max_attempts {
        let now = crate::current_timestamp_ms();

        // Check if we are still allowed to retry
        if !guard.can_retry() {
            // First byte was already dispatched; strict rule forbids replay
            break;
        }

        // Select healthy candidate according to LRU + Quota Domain rules
        let account = match scheduler.select_account(now) {
            Ok(acc) => acc,
            Err(e) => {
                last_error = e;
                break;
            }
        };

        // Resolve API key for chosen account (supports both GEMINI_KEYS_POOL and GEMINI_KEY_*)
        let api_key = match UpstreamClient::resolve_api_key(env, &account) {
            Ok(key) => key,
            Err(e) => {
                // If key is missing in Cloudflare Secrets, disable account and try next
                scheduler.report_permanent_auth_failure(&account.id, "Secret missing in Worker runtime");
                last_error = e;
                continue;
            }
        };

        let upstream_send_start = crate::current_timestamp_ms();

        // Dispatch outbound request to Google Gemini API
        match upstream_client.send_chat_completion(&api_key, &serialized_payload).await {
            Ok(upstream_res) => {
                let status = upstream_res.status_code();
                let ttfb = crate::current_timestamp_ms().saturating_sub(upstream_send_start);
                guard.mark_headers_received();

                if (200..300).contains(&status) {
                    // Success! Record account success
                    scheduler.report_success(&account.id, crate::current_timestamp_ms());
                    guard.mark_stream_started();

                    let duration = crate::current_timestamp_ms().saturating_sub(start_time);
                    RequestLogEvent {
                        request_id: &request_id,
                        account_id: &account.id,
                        quota_domain: &account.quota_domain,
                        model: &target_model,
                        stream: is_streaming,
                        attempt,
                        status,
                        ttfb_ms: ttfb,
                        duration_ms: duration,
                        event: Some("success"),
                    }
                    .emit();

                    if is_streaming {
                        return create_sse_response(upstream_res);
                    } else {
                        return create_json_forward_response(upstream_res);
                    }
                } else {
                    // Upstream returned HTTP error status
                    let mut resp_headers = HashMap::new();
                    for (k, v) in upstream_res.headers() {
                        resp_headers.insert(k.to_lowercase(), v);
                    }

                    let classification = classify_upstream_response(status, &resp_headers, config.retry.retry_after_cap_ms);

                    // Structured logging of attempt failure
                    let duration = crate::current_timestamp_ms().saturating_sub(start_time);
                    RequestLogEvent {
                        request_id: &request_id,
                        account_id: &account.id,
                        quota_domain: &account.quota_domain,
                        model: &target_model,
                        stream: is_streaming,
                        attempt,
                        status,
                        ttfb_ms: ttfb,
                        duration_ms: duration,
                        event: Some("attempt_failed"),
                    }
                    .emit();

                    // Apply cooldowns and classify
                    match classification {
                        crate::retry::ErrorClassification::RetryableRateLimit { retry_after_ms } => {
                            let duration = retry_after_ms.unwrap_or(config.cooldown.after_429_ms);
                            scheduler.report_cooldown(&account.id, "429 Rate Limit", duration, crate::current_timestamp_ms(), "RATE_LIMIT_429");
                            last_error = GatewayError::UpstreamError {
                                status: 429,
                                message: format!("Upstream rate limit (429) on account {}", account.id),
                            };
                        }
                        crate::retry::ErrorClassification::RetryableServerError { status } => {
                            scheduler.report_cooldown(&account.id, "5xx Server Error", config.cooldown.after_5xx_ms, crate::current_timestamp_ms(), "SERVER_ERROR_5XX");
                            last_error = GatewayError::UpstreamError {
                                status,
                                message: format!("Upstream server error ({}) on account {}", status, account.id),
                            };
                        }
                        crate::retry::ErrorClassification::PermanentAuthError { status, message } => {
                            scheduler.report_permanent_auth_failure(&account.id, &message);
                            return Err(GatewayError::UpstreamError { status, message });
                        }
                        crate::retry::ErrorClassification::PermanentClientError { status, message } => {
                            return Err(GatewayError::UpstreamError { status, message });
                        }
                        _ => {
                            last_error = GatewayError::UpstreamError {
                                status,
                                message: format!("Upstream error ({}) on account {}", status, account.id),
                            };
                        }
                    }
                }
            }
            Err(e) => {
                // Network or connection error before receiving headers
                scheduler.report_cooldown(
                    &account.id,
                    "Network error",
                    config.cooldown.after_network_error_ms,
                    crate::current_timestamp_ms(),
                    "NETWORK_ERROR",
                );
                last_error = e;
            }
        }
    }

    Err(last_error)
}
