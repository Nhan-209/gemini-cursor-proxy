use crate::accounts::AccountRuntimeState;
use crate::config::AccountConfig;
use crate::error::GatewayError;
use std::collections::HashMap;
use std::sync::RwLock;

#[derive(Debug, Clone, PartialEq)]
pub struct AccountSnapshot {
    pub id: String,
    pub secret_name: String,
    pub quota_domain: String,
}

pub trait AccountScheduler: Send + Sync {
    fn select_account(&self, current_time_ms: u64) -> Result<AccountSnapshot, GatewayError>;
    fn report_success(&self, account_id: &str, current_time_ms: u64);
    fn report_cooldown(&self, account_id: &str, reason: &str, duration_ms: u64, current_time_ms: u64, error_class: &str);
    fn report_quota_domain_cooldown(&self, quota_domain: &str, reason: &str, duration_ms: u64, current_time_ms: u64);
    fn report_permanent_auth_failure(&self, account_id: &str, reason: &str);
}

#[derive(Debug)]
struct InnerSchedulerState {
    accounts: Vec<AccountRuntimeState>,
    domain_cooldowns: HashMap<String, u64>, // quota_domain -> until_ms
}

pub struct LruQuotaScheduler {
    inner: RwLock<InnerSchedulerState>,
}

impl LruQuotaScheduler {
    pub fn new(configs: &[AccountConfig]) -> Self {
        let accounts = configs.iter().map(AccountRuntimeState::from_config).collect();
        Self {
            inner: RwLock::new(InnerSchedulerState {
                accounts,
                domain_cooldowns: HashMap::new(),
            }),
        }
    }

    pub fn get_account_states(&self) -> Vec<AccountRuntimeState> {
        let state = self.inner.read().expect("Lock poisoned");
        state.accounts.clone()
    }
}

impl AccountScheduler for LruQuotaScheduler {
    fn select_account(&self, current_time_ms: u64) -> Result<AccountSnapshot, GatewayError> {
        let mut state = self.inner.write().map_err(|e| GatewayError::Internal(format!("Lock error: {}", e)))?;

        // 1. Expire individual account cooldowns
        for acc in state.accounts.iter_mut() {
            acc.check_cooldown_expiry(current_time_ms);
        }

        // 2. Clean expired domain cooldowns
        state.domain_cooldowns.retain(|_, until_ms| current_time_ms < *until_ms);

        // 3. Filter candidates
        let mut candidates: Vec<&AccountRuntimeState> = state
            .accounts
            .iter()
            .filter(|acc| {
                if !acc.is_available(current_time_ms) {
                    return false;
                }
                // Check if its quota domain is cooling down
                if let Some(domain_until) = state.domain_cooldowns.get(&acc.quota_domain) {
                    if current_time_ms < *domain_until {
                        return false;
                    }
                }
                true
            })
            .collect();

        if candidates.is_empty() {
            return Err(GatewayError::AllAccountsExhausted(
                "All upstream accounts or quota domains are currently in cooldown or disabled".to_string(),
            ));
        }

        // 4. Sort by LRU (least recently used) then lowest consecutive failures
        candidates.sort_by(|a, b| {
            a.last_used_ms
                .cmp(&b.last_used_ms)
                .then_with(|| a.consecutive_failures.cmp(&b.consecutive_failures))
                .then_with(|| a.failure_count.cmp(&b.failure_count))
        });

        let chosen = candidates[0];
        Ok(AccountSnapshot {
            id: chosen.id.clone(),
            secret_name: chosen.secret_name.clone(),
            quota_domain: chosen.quota_domain.clone(),
        })
    }

    fn report_success(&self, account_id: &str, current_time_ms: u64) {
        if let Ok(mut state) = self.inner.write() {
            if let Some(acc) = state.accounts.iter_mut().find(|a| a.id == account_id) {
                acc.record_success(current_time_ms);
            }
        }
    }

    fn report_cooldown(&self, account_id: &str, reason: &str, duration_ms: u64, current_time_ms: u64, error_class: &str) {
        if let Ok(mut state) = self.inner.write() {
            let mut domain_to_cool: Option<String> = None;
            if let Some(acc) = state.accounts.iter_mut().find(|a| a.id == account_id) {
                acc.record_cooldown(reason, duration_ms, current_time_ms, error_class);
                if error_class == "RATE_LIMIT_429" {
                    domain_to_cool = Some(acc.quota_domain.clone());
                }
            }

            // If 429 rate limit, cool down entire quota domain
            if let Some(domain) = domain_to_cool {
                let until = current_time_ms.saturating_add(duration_ms);
                state.domain_cooldowns.insert(domain, until);
            }
        }
    }

    fn report_quota_domain_cooldown(&self, quota_domain: &str, _reason: &str, duration_ms: u64, current_time_ms: u64) {
        if let Ok(mut state) = self.inner.write() {
            let until = current_time_ms.saturating_add(duration_ms);
            state.domain_cooldowns.insert(quota_domain.to_string(), until);
        }
    }

    fn report_permanent_auth_failure(&self, account_id: &str, reason: &str) {
        if let Ok(mut state) = self.inner.write() {
            if let Some(acc) = state.accounts.iter_mut().find(|a| a.id == account_id) {
                acc.disable(reason);
            }
        }
    }
}
