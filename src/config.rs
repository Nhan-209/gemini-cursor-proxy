use crate::error::GatewayError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServerConfig {
    pub base_path: String,
    pub max_body_bytes: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            base_path: "/v1".to_string(),
            max_body_bytes: 10_000_000, // 10MB
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UpstreamConfig {
    pub base_url: String,
    pub connect_timeout_ms: u64,
    pub first_byte_timeout_ms: u64,
}

impl Default for UpstreamConfig {
    fn default() -> Self {
        Self {
            base_url: "https://generativelanguage.googleapis.com/v1beta/openai".to_string(),
            connect_timeout_ms: 10_000,
            first_byte_timeout_ms: 30_000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelConfig {
    pub primary: String,
    pub thinking_level: String,
    pub force_model: bool,
    #[serde(default)]
    pub aliases: HashMap<String, String>,
}

impl Default for ModelConfig {
    fn default() -> Self {
        let mut aliases = HashMap::new();
        aliases.insert("auto".to_string(), "gemini-3.8-flash".to_string());
        aliases.insert("gpt-4o".to_string(), "gemini-3.8-flash".to_string());
        aliases.insert("gpt-4.1".to_string(), "gemini-3.8-flash".to_string());
        aliases.insert("gemini-3.8-flash".to_string(), "gemini-3.8-flash".to_string());

        Self {
            primary: "gemini-3.8-flash".to_string(),
            thinking_level: "high".to_string(),
            force_model: true,
            aliases,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetryConfig {
    pub max_attempts: usize,
    pub retry_429: bool,
    pub retry_5xx: bool,
    pub retry_network: bool,
    pub retry_after_cap_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            retry_429: true,
            retry_5xx: true,
            retry_network: true,
            retry_after_cap_ms: 10_000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CooldownConfig {
    pub after_429_ms: u64,
    pub after_5xx_ms: u64,
    pub after_network_error_ms: u64,
    pub max_cooldown_ms: u64,
}

impl Default for CooldownConfig {
    fn default() -> Self {
        Self {
            after_429_ms: 30_000,
            after_5xx_ms: 10_000,
            after_network_error_ms: 5_000,
            max_cooldown_ms: 300_000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StreamingConfig {
    pub enabled: bool,
    pub preserve_sse: bool,
    pub disable_buffering: bool,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            preserve_sse: true,
            disable_buffering: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SecurityConfig {
    pub require_proxy_token: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            require_proxy_token: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AccountConfig {
    pub id: String,
    pub secret_name: String,
    #[serde(default)]
    pub direct_key: Option<String>,
    pub quota_domain: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    pub rpm_limit: Option<u32>,
    pub tpm_limit: Option<u32>,
    pub rpd_limit: Option<u32>,
}

pub fn parse_keys_pool(raw: &str) -> Vec<AccountConfig> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    #[derive(Deserialize)]
    struct KeyObj {
        key: String,
        domain: Option<String>,
        project: Option<String>,
    }

    // 1. Try parsing as JSON array of objects [{"key":"...", "domain":"..."}]
    if trimmed.starts_with('[') {
        if let Ok(objs) = serde_json::from_str::<Vec<KeyObj>>(trimmed) {
            return objs
                .into_iter()
                .enumerate()
                .filter(|(_, obj)| !obj.key.trim().is_empty() && obj.key.trim() != "replace_me")
                .map(|(idx, obj)| {
                    let key = obj.key.trim().to_string();
                    let domain = obj
                        .domain
                        .or(obj.project)
                        .unwrap_or_else(|| format!("pool-domain-{:04}", idx + 1));
                    AccountConfig {
                        id: format!("pool_{:04}", idx + 1),
                        secret_name: "GEMINI_KEYS_POOL".to_string(),
                        direct_key: Some(key),
                        quota_domain: domain,
                        enabled: true,
                        rpm_limit: Some(15),
                        tpm_limit: Some(1_000_000),
                        rpd_limit: Some(1_500),
                    }
                })
                .collect();
        }

        // 2. Try parsing as JSON array of strings ["key1", "key2"]
        if let Ok(keys) = serde_json::from_str::<Vec<String>>(trimmed) {
            return keys
                .into_iter()
                .enumerate()
                .filter(|(_, k)| !k.trim().is_empty() && k.trim() != "replace_me")
                .map(|(idx, k)| {
                    let key = k.trim().to_string();
                    AccountConfig {
                        id: format!("pool_{:04}", idx + 1),
                        secret_name: "GEMINI_KEYS_POOL".to_string(),
                        direct_key: Some(key),
                        quota_domain: format!("pool-domain-{:04}", idx + 1),
                        enabled: true,
                        rpm_limit: Some(15),
                        tpm_limit: Some(1_000_000),
                        rpd_limit: Some(1_500),
                    }
                })
                .collect();
        }
    }

    // 3. Otherwise parse as plain text (separated by newlines, commas, or semicolons)
    trimmed
        .split(|c| c == '\n' || c == '\r' || c == ',' || c == ';')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && *s != "replace_me")
        .enumerate()
        .map(|(idx, key)| AccountConfig {
            id: format!("pool_{:04}", idx + 1),
            secret_name: "GEMINI_KEYS_POOL".to_string(),
            direct_key: Some(key.to_string()),
            quota_domain: format!("pool-domain-{:04}", idx + 1),
            enabled: true,
            rpm_limit: Some(15),
            tpm_limit: Some(1_000_000),
            rpd_limit: Some(1_500),
        })
        .collect()
}

fn default_enabled() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppConfig {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub upstream: UpstreamConfig,
    #[serde(default)]
    pub model: ModelConfig,
    #[serde(default)]
    pub smart_router: crate::classifier::SmartRouterConfig,
    #[serde(default)]
    pub retry: RetryConfig,
    #[serde(default)]
    pub cooldown: CooldownConfig,
    #[serde(default)]
    pub streaming: StreamingConfig,
    #[serde(default)]
    pub security: SecurityConfig,
    #[serde(default = "default_accounts")]
    pub accounts: Vec<AccountConfig>,
}

fn default_accounts() -> Vec<AccountConfig> {
    (1..=20)
        .map(|i| {
            let id = format!("p{:02}", i);
            let secret_name = format!("GEMINI_KEY_{:02}", i);
            let quota_domain = format!("project-{:02}", i);
            AccountConfig {
                id,
                secret_name,
                direct_key: None,
                quota_domain,
                enabled: true,
                rpm_limit: Some(15),
                tpm_limit: Some(1_000_000),
                rpd_limit: Some(1_500),
            }
        })
        .collect()
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            upstream: UpstreamConfig::default(),
            model: ModelConfig::default(),
            smart_router: crate::classifier::SmartRouterConfig::default(),
            retry: RetryConfig::default(),
            cooldown: CooldownConfig::default(),
            streaming: StreamingConfig::default(),
            security: SecurityConfig::default(),
            accounts: default_accounts(),
        }
    }
}

impl AppConfig {
    pub fn from_toml_str(s: &str) -> Result<Self, GatewayError> {
        toml::from_str(s).map_err(|e| GatewayError::ConfigError(format!("Failed to parse config TOML: {}", e)))
    }

    pub fn apply_env_overrides(&mut self, env: &worker::Env) {
        if let Ok(val) = env.var("PRIMARY_MODEL") {
            self.model.primary = val.to_string();
        }
        if let Ok(val) = env.var("THINKING_LEVEL") {
            self.model.thinking_level = val.to_string();
        }
        if let Ok(val) = env.var("FORCE_MODEL") {
            self.model.force_model = val.to_string().parse().unwrap_or(self.model.force_model);
        }
        if let Ok(val) = env.var("SMART_ROUTER_ENABLED") {
            self.smart_router.enabled = val.to_string().parse().unwrap_or(self.smart_router.enabled);
        }
        if let Ok(val) = env.var("BASE_PATH") {
            self.server.base_path = val.to_string();
        }
        if let Ok(val) = env.var("ACCOUNTS_CONFIG_TOML") {
            if let Ok(parsed) = toml::from_str::<AppConfig>(&val.to_string()) {
                if !parsed.accounts.is_empty() {
                    self.accounts = parsed.accounts;
                }
            }
        }
    }
}
