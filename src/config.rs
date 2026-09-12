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
        // Dynamic smart default
        aliases.insert("auto".to_string(), "gemini-3.8-flash".to_string());

        // OpenAI mapping
        aliases.insert("gpt-4o".to_string(), "gemini-3.8-flash".to_string());
        aliases.insert("gpt-4o-mini".to_string(), "gemini-3.5-flash-lite".to_string());
        aliases.insert("gpt-4.1".to_string(), "gemini-3.8-flash".to_string());
        aliases.insert("o1".to_string(), "gemini-2.5-pro".to_string());
        aliases.insert("o3-mini".to_string(), "gemini-3.8-flash".to_string());

        // Anthropic & DeepSeek mapping
        aliases.insert("claude-3-5-sonnet".to_string(), "gemini-2.5-pro".to_string());
        aliases.insert("claude-3-7-sonnet".to_string(), "gemini-2.5-pro".to_string());
        aliases.insert("deepseek-chat".to_string(), "gemini-3.8-flash".to_string());
        aliases.insert("deepseek-reasoner".to_string(), "gemini-2.5-pro".to_string());

        // Native Google Gemini models pass-through
        aliases.insert("gemini-3.8-flash".to_string(), "gemini-3.8-flash".to_string());
        aliases.insert("gemini-3.5-flash-lite".to_string(), "gemini-3.5-flash-lite".to_string());
        aliases.insert("gemini-2.5-flash".to_string(), "gemini-2.5-flash".to_string());
        aliases.insert("gemini-2.5-pro".to_string(), "gemini-2.5-pro".to_string());
        aliases.insert("gemini-2.0-flash".to_string(), "gemini-2.0-flash".to_string());
        aliases.insert("gemini-2.0-flash-lite".to_string(), "gemini-3.5-flash-lite".to_string());
        aliases.insert("gemini-1.5-flash".to_string(), "gemini-1.5-flash".to_string());
        aliases.insert("gemini-1.5-pro".to_string(), "gemini-1.5-pro".to_string());

        Self {
            primary: "gemini-3.8-flash".to_string(),
            thinking_level: "high".to_string(),
            force_model: false,
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

fn sanitize_token(s: &str) -> Option<String> {
    let mut val = s.trim();
    if val.is_empty() {
        return None;
    }
    // If it's a key-value pair e.g. "GEMINI_KEYS_POOL = AIza..." or "key: AIza..."
    if let Some((_, rhs)) = val.split_once('=') {
        val = rhs.trim();
    } else if let Some((lhs, rhs)) = val.split_once(':') {
        let lhs_lower = lhs.trim().to_lowercase();
        if lhs_lower.contains("key")
            || lhs_lower.contains("gemini")
            || lhs_lower.contains("token")
            || lhs_lower.contains("pool")
        {
            val = rhs.trim();
        }
    }
    // Strip quotes, brackets, braces, commas, semicolons
    let cleaned = val
        .trim_matches(|c| {
            c == '['
                || c == ']'
                || c == '{'
                || c == '}'
                || c == '"'
                || c == '\''
                || c == ','
                || c == ';'
        })
        .trim();

    if cleaned.is_empty() || cleaned == "replace_me" {
        return None;
    }

    let lower = cleaned.to_lowercase();
    if lower == "gemini_keys_pool"
        || lower == "gemini_key_pool"
        || lower == "gemini_key"
        || lower == "gemini_keys"
        || lower == "key"
        || lower == "api_key"
    {
        return None;
    }

    Some(cleaned.to_string())
}

fn split_raw_into_tokens(raw: &str) -> Vec<String> {
    let mut text = raw.trim();
    // Strip leading env var name e.g. "GEMINI_KEYS_POOL = ..."
    if let Some((lhs, rhs)) = text.split_once('=') {
        let lhs_lower = lhs.trim().to_lowercase();
        if lhs_lower.contains("gemini")
            || lhs_lower.contains("pool")
            || lhs_lower.contains("key")
            || lhs_lower.contains("token")
        {
            text = rhs.trim();
        }
    }

    let mut tokens = Vec::new();
    for part in text.split(|c| c == '\n' || c == '\r' || c == ',' || c == ';' || c == ' ' || c == '\t') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        // If a token still contains multiple keys joined together (e.g. AIza... or AQ....)
        if trimmed.len() > 50 && (trimmed.contains("AIza") || trimmed.contains("AQ.")) {
            let mut remaining = trimmed;
            while !remaining.is_empty() {
                let next_aiza = remaining.find("AIza");
                let next_aq = remaining.find("AQ.");
                let next_start = match (next_aiza, next_aq) {
                    (Some(a), Some(b)) => Some(a.min(b)),
                    (Some(a), None) => Some(a),
                    (None, Some(b)) => Some(b),
                    (None, None) => None,
                };

                match next_start {
                    Some(0) => {
                        let rest = &remaining[3..];
                        let next_delim = match (rest.find("AIza"), rest.find("AQ.")) {
                            (Some(a), Some(b)) => Some(a.min(b)),
                            (Some(a), None) => Some(a),
                            (None, Some(b)) => Some(b),
                            (None, None) => None,
                        };
                        if let Some(cut) = next_delim {
                            let key = &remaining[..cut + 3];
                            if let Some(t) = sanitize_token(key) {
                                tokens.push(t);
                            }
                            remaining = &remaining[cut + 3..];
                        } else {
                            if let Some(t) = sanitize_token(remaining) {
                                tokens.push(t);
                            }
                            break;
                        }
                    }
                    Some(pos) => {
                        let before = &remaining[..pos];
                        if let Some(t) = sanitize_token(before) {
                            tokens.push(t);
                        }
                        remaining = &remaining[pos..];
                    }
                    None => {
                        if let Some(t) = sanitize_token(remaining) {
                            tokens.push(t);
                        }
                        break;
                    }
                }
            }
        } else if let Some(t) = sanitize_token(trimmed) {
            tokens.push(t);
        }
    }
    tokens
}

pub fn parse_keys_pool(raw: &str) -> Vec<AccountConfig> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    #[derive(Deserialize)]
    struct KeyObj {
        key: Option<String>,
        api_key: Option<String>,
        gemini_key: Option<String>,
        domain: Option<String>,
        project: Option<String>,
    }

    // 1. Try parsing as JSON array of objects [{"key":"...", "domain":"..."}]
    if trimmed.starts_with('[') {
        if let Ok(objs) = serde_json::from_str::<Vec<KeyObj>>(trimmed) {
            let res: Vec<AccountConfig> = objs
                .into_iter()
                .enumerate()
                .filter_map(|(idx, obj)| {
                    let raw_key = obj.key.or(obj.api_key).or(obj.gemini_key)?;
                    let clean = sanitize_token(&raw_key)?;
                    let domain = obj
                        .domain
                        .or(obj.project)
                        .unwrap_or_else(|| format!("pool-domain-{:04}", idx + 1));
                    Some(AccountConfig {
                        id: format!("pool_{:04}", idx + 1),
                        secret_name: "GEMINI_KEYS_POOL".to_string(),
                        direct_key: Some(clean),
                        quota_domain: domain,
                        enabled: true,
                        rpm_limit: Some(15),
                        tpm_limit: Some(1_000_000),
                        rpd_limit: Some(1_500),
                    })
                })
                .collect();
            if !res.is_empty() {
                return res;
            }
        }

        // 2. Try parsing as JSON array of strings ["key1", "key2"]
        if let Ok(keys) = serde_json::from_str::<Vec<String>>(trimmed) {
            let res: Vec<AccountConfig> = keys
                .into_iter()
                .enumerate()
                .filter_map(|(idx, k)| {
                    let clean = sanitize_token(&k)?;
                    Some(AccountConfig {
                        id: format!("pool_{:04}", idx + 1),
                        secret_name: "GEMINI_KEYS_POOL".to_string(),
                        direct_key: Some(clean),
                        quota_domain: format!("pool-domain-{:04}", idx + 1),
                        enabled: true,
                        rpm_limit: Some(15),
                        tpm_limit: Some(1_000_000),
                        rpd_limit: Some(1_500),
                    })
                })
                .collect();
            if !res.is_empty() {
                return res;
            }
        }
    }

    // 3. Try parsing as a single JSON object {"key": "..."}
    if trimmed.starts_with('{') {
        if let Ok(obj) = serde_json::from_str::<KeyObj>(trimmed) {
            if let Some(raw_key) = obj.key.or(obj.api_key).or(obj.gemini_key) {
                if let Some(clean) = sanitize_token(&raw_key) {
                    let domain = obj
                        .domain
                        .or(obj.project)
                        .unwrap_or_else(|| "pool-domain-0001".to_string());
                    return vec![AccountConfig {
                        id: "pool_0001".to_string(),
                        secret_name: "GEMINI_KEYS_POOL".to_string(),
                        direct_key: Some(clean),
                        quota_domain: domain,
                        enabled: true,
                        rpm_limit: Some(15),
                        tpm_limit: Some(1_000_000),
                        rpd_limit: Some(1_500),
                    }];
                }
            }
        }
    }

    // 4. Otherwise parse as plain text (separated by newlines, commas, semicolons, spaces, tabs)
    split_raw_into_tokens(trimmed)
        .into_iter()
        .enumerate()
        .map(|(idx, key)| AccountConfig {
            id: format!("pool_{:04}", idx + 1),
            secret_name: "GEMINI_KEYS_POOL".to_string(),
            direct_key: Some(key),
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
