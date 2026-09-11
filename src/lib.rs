pub mod accounts;
pub mod auth;
pub mod classifier;
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

    // 2. Resolve accounts: prioritize GEMINI_KEYS_POOL (supports 100 - 1,000+ keys in one secret)
    let active_accounts: Vec<config::AccountConfig> = {
        let pool_val = env
            .secret("GEMINI_KEYS_POOL")
            .map(|s| s.to_string())
            .or_else(|_| env.var("GEMINI_KEYS_POOL").map(|v| v.to_string()))
            .ok();

        if let Some(raw_pool) = pool_val {
            let parsed = config::parse_keys_pool(&raw_pool);
            if !parsed.is_empty() {
                parsed
            } else {
                resolve_discrete_accounts(&config.accounts, &env)
            }
        } else {
            resolve_discrete_accounts(&config.accounts, &env)
        }
    };

    // 3. Initialize the LRU Quota Scheduler with active accounts
    let scheduler = Arc::new(LruQuotaScheduler::new(&active_accounts));

    // 4. Dispatch incoming request through the gateway router
    match handle_request(req, env, &config, &scheduler).await {
        Ok(res) => Ok(res),
        Err(gateway_err) => gateway_err.to_worker_response(),
    }
}

fn resolve_discrete_accounts(accounts: &[config::AccountConfig], env: &Env) -> Vec<config::AccountConfig> {
    let configured: Vec<config::AccountConfig> = accounts
        .iter()
        .filter(|acc| {
            if !acc.enabled {
                return false;
            }
            env.secret(&acc.secret_name).is_ok() || env.var(&acc.secret_name).is_ok()
        })
        .cloned()
        .collect();

    if configured.is_empty() {
        // Fallback to first 5 accounts if secrets not yet populated
        accounts.iter().take(5).cloned().collect()
    } else {
        configured
    }
}
