use crate::config::AccountConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AccountHealth {
    Healthy,
    Cooldown { until_ms: u64, reason: String },
    Disabled { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountRuntimeState {
    pub id: String,
    pub secret_name: String,
    pub quota_domain: String,
    pub enabled: bool,
    pub rpm_limit: Option<u32>,
    pub tpm_limit: Option<u32>,
    pub rpd_limit: Option<u32>,
    pub health: AccountHealth,
    pub consecutive_failures: u32,
    pub success_count: u64,
    pub failure_count: u64,
    pub last_used_ms: u64,
    pub last_error: Option<String>,
    pub last_error_class: Option<String>,
}

impl AccountRuntimeState {
    pub fn from_config(cfg: &AccountConfig) -> Self {
        let health = if cfg.enabled {
            AccountHealth::Healthy
        } else {
            AccountHealth::Disabled {
                reason: "Configured disabled".to_string(),
            }
        };

        Self {
            id: cfg.id.clone(),
            secret_name: cfg.secret_name.clone(),
            quota_domain: cfg.quota_domain.clone(),
            enabled: cfg.enabled,
            rpm_limit: cfg.rpm_limit,
            tpm_limit: cfg.tpm_limit,
            rpd_limit: cfg.rpd_limit,
            health,
            consecutive_failures: 0,
            success_count: 0,
            failure_count: 0,
            last_used_ms: 0,
            last_error: None,
            last_error_class: None,
        }
    }

    pub fn is_available(&self, current_time_ms: u64) -> bool {
        if !self.enabled {
            return false;
        }
        match &self.health {
            AccountHealth::Healthy => true,
            AccountHealth::Cooldown { until_ms, .. } => current_time_ms >= *until_ms,
            AccountHealth::Disabled { .. } => false,
        }
    }

    pub fn check_cooldown_expiry(&mut self, current_time_ms: u64) {
        if let AccountHealth::Cooldown { until_ms, .. } = &self.health {
            if current_time_ms >= *until_ms {
                self.health = AccountHealth::Healthy;
                self.consecutive_failures = 0;
            }
        }
    }

    pub fn record_success(&mut self, current_time_ms: u64) {
        self.health = AccountHealth::Healthy;
        self.consecutive_failures = 0;
        self.success_count += 1;
        self.last_used_ms = current_time_ms;
        self.last_error = None;
        self.last_error_class = None;
    }

    pub fn record_cooldown(&mut self, reason: &str, duration_ms: u64, current_time_ms: u64, error_class: &str) {
        self.consecutive_failures += 1;
        self.failure_count += 1;
        self.last_error = Some(reason.to_string());
        self.last_error_class = Some(error_class.to_string());
        self.health = AccountHealth::Cooldown {
            until_ms: current_time_ms.saturating_add(duration_ms),
            reason: reason.to_string(),
        };
    }

    pub fn disable(&mut self, reason: &str) {
        self.enabled = false;
        self.health = AccountHealth::Disabled {
            reason: reason.to_string(),
        };
        self.last_error = Some(reason.to_string());
        self.last_error_class = Some("PERMANENT_AUTH_FAILURE".to_string());
    }
}
