use gemini_cursor_proxy::config::AppConfig;

#[test]
fn test_default_config_values() {
    let cfg = AppConfig::default();

    assert_eq!(cfg.server.base_path, "/v1");
    assert_eq!(cfg.server.max_body_bytes, 10_000_000);
    assert_eq!(cfg.model.primary, "gemini-3.8-flash");
    assert_eq!(cfg.model.thinking_level, "high");
    assert!(cfg.model.force_model);
    assert_eq!(cfg.retry.max_attempts, 3);
    assert_eq!(cfg.cooldown.after_429_ms, 30_000);
    assert_eq!(cfg.accounts.len(), 5);
    assert_eq!(cfg.accounts[0].id, "p01");
    assert_eq!(cfg.accounts[0].quota_domain, "project-a");
}

#[test]
fn test_config_parsing_from_toml() {
    let toml_str = r#"
        [server]
        base_path = "/v1"
        max_body_bytes = 5000000

        [upstream]
        base_url = "https://generativelanguage.googleapis.com/v1beta/openai"
        connect_timeout_ms = 8000
        first_byte_timeout_ms = 25000

        [model]
        primary = "gemini-3.8-flash"
        thinking_level = "high"
        force_model = true

        [model.aliases]
        auto = "gemini-3.8-flash"
        gpt-4o = "gemini-3.8-flash"

        [retry]
        max_attempts = 5
        retry_429 = true
        retry_5xx = true
        retry_network = true
        retry_after_cap_ms = 15000

        [cooldown]
        after_429_ms = 45000
        after_5xx_ms = 12000
        after_network_error_ms = 6000
        max_cooldown_ms = 300000

        [streaming]
        enabled = true
        preserve_sse = true
        disable_buffering = true

        [security]
        require_proxy_token = true

        [[accounts]]
        id = "p01"
        secret_name = "GEMINI_KEY_01"
        quota_domain = "project-custom"
        enabled = true
    "#;

    let parsed = AppConfig::from_toml_str(toml_str).expect("Valid TOML should parse");
    assert_eq!(parsed.server.max_body_bytes, 5_000_000);
    assert_eq!(parsed.retry.max_attempts, 5);
    assert_eq!(parsed.accounts.len(), 1);
    assert_eq!(parsed.accounts[0].id, "p01");
    assert_eq!(parsed.accounts[0].quota_domain, "project-custom");
}

#[test]
fn test_invalid_toml_returns_error() {
    let invalid_toml = "this is not valid toml = [[";
    let res = AppConfig::from_toml_str(invalid_toml);
    assert!(res.is_err());
}
