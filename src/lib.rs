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

    // 2. Resolve accounts: combine GEMINI_KEYS_POOL with discrete GEMINI_KEY_01..20
    let active_accounts: Vec<config::AccountConfig> = {
        let pool_names = [
            "GEMINI_KEYS_POOL",
            "GEMINI_KEY_POOL",
            "GEMINI_POOL",
            "GEMINI_KEYS",
            "GEMINI_API_KEY",
            "GEMINI_KEY",
        ];

        let mut all_accounts = Vec::new();

        for name in pool_names {
            if let Ok(s) = env.secret(name) {
                let v = s.to_string();
                if !v.trim().is_empty() {
                    let mut parsed = config::parse_keys_pool(&v);
                    all_accounts.append(&mut parsed);
                    break;
                }
            }
            if let Ok(v) = env.var(name) {
                let s = v.to_string();
                if !s.trim().is_empty() {
                    let mut parsed = config::parse_keys_pool(&s);
                    all_accounts.append(&mut parsed);
                    break;
                }
            }
        }

        let discrete = resolve_discrete_accounts(&config.accounts, &env);
        for acc in discrete {
            all_accounts.push(acc);
        }

        if all_accounts.is_empty() {
            // Fallback to first 5 template accounts if nothing configured
            config.accounts.iter().take(5).cloned().collect()
        } else {
            all_accounts
                .into_iter()
                .enumerate()
                .map(|(idx, mut acc)| {
                    acc.id = format!("acc_{:04}", idx + 1);
                    acc
                })
                .collect()
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
    accounts
        .iter()
        .filter(|acc| {
            if !acc.enabled {
                return false;
            }
            if let Ok(s) = env.secret(&acc.secret_name) {
                let v = s.to_string();
                let t = v.trim();
                !t.is_empty() && t != "replace_me"
            } else if let Ok(v) = env.var(&acc.secret_name) {
                let s = v.to_string();
                let t = s.trim();
                !t.is_empty() && t != "replace_me"
            } else {
                false
            }
        })
        .cloned()
        .collect()
}
