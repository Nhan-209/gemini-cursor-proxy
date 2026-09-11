pub mod accounts;
pub mod auth;
pub mod config;
pub mod error;
pub mod metrics;
pub mod models;
pub mod retry;
pub mod router;
pub mod scheduler;
pub mod streaming;
pub mod upstream;

use config::AppConfig;
use router::handle_request;
use scheduler::LruQuotaScheduler;
use std::sync::Arc;
use worker::*;

pub fn current_timestamp_ms() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        Date::now().as_millis()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }
}

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    // 1. Load configuration and apply runtime environment overrides
    let mut config = AppConfig::default();
    config.apply_env_overrides(&env);

    // 2. Initialize the LRU Quota Scheduler with configured accounts
    let scheduler = Arc::new(LruQuotaScheduler::new(&config.accounts));

    // 3. Dispatch incoming request through the gateway router
    match handle_request(req, env, &config, &scheduler).await {
        Ok(res) => Ok(res),
        Err(gateway_err) => gateway_err.to_worker_response(),
    }
}
