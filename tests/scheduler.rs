use gemini_openai_gateway::config::AccountConfig;
use gemini_openai_gateway::scheduler::{AccountScheduler, LruQuotaScheduler};

fn sample_accounts() -> Vec<AccountConfig> {
    vec![
        AccountConfig {
            id: "p01".to_string(),
            secret_name: "GEMINI_KEY_01".to_string(),
            direct_key: None,
            quota_domain: "project-a".to_string(),
            enabled: true,
            rpm_limit: None,
            tpm_limit: None,
            rpd_limit: None,
        },
        AccountConfig {
            id: "p02".to_string(),
            secret_name: "GEMINI_KEY_02".to_string(),
            direct_key: None,
            quota_domain: "project-a".to_string(), // Shares quota domain with p01
            enabled: true,
            rpm_limit: None,
            tpm_limit: None,
            rpd_limit: None,
        },
        AccountConfig {
            id: "p03".to_string(),
            secret_name: "GEMINI_KEY_03".to_string(),
            direct_key: None,
            quota_domain: "project-b".to_string(), // Independent quota domain
            enabled: true,
            rpm_limit: None,
            tpm_limit: None,
            rpd_limit: None,
        },
    ]
}

#[test]
fn test_lru_selection_and_success_rotation() {
    let accounts = sample_accounts();
    let scheduler = LruQuotaScheduler::new(&accounts);

    // Initial select picks p01
    let acc1 = scheduler.select_account(1000).expect("Should pick account");
    assert_eq!(acc1.id, "p01");
    scheduler.report_success(&acc1.id, 1000);

    // Next select picks least recently used (p02 or p03)
    let acc2 = scheduler.select_account(1001).expect("Should pick account");
    assert_ne!(acc2.id, "p01");
}

#[test]
fn test_quota_domain_cooldown_isolates_sister_accounts() {
    let accounts = sample_accounts();
    let scheduler = LruQuotaScheduler::new(&accounts);

    // p01 encounters a 429 rate limit at t=1000 with 30s cooldown
    scheduler.report_cooldown("p01", "Rate limit", 30_000, 1000, "RATE_LIMIT_429");

    // At t=1001, both p01 and p02 (sharing 'project-a') should be excluded!
    let acc = scheduler.select_account(1001).expect("Should pick available project-b account");
    assert_eq!(acc.id, "p03");
    assert_eq!(acc.quota_domain, "project-b");
}

#[test]
fn test_cooldown_expiration_restores_health() {
    let accounts = sample_accounts();
    let scheduler = LruQuotaScheduler::new(&accounts);

    // Put p01 in cooldown for 10s at t=1000
    scheduler.report_cooldown("p01", "503 Error", 10_000, 1000, "SERVER_ERROR_5XX");

    // At t=5000, p01 is in cooldown
    // At t=12000, cooldown is expired, p01 should be restored
    let _ = scheduler.select_account(12_000).expect("Should succeed");
    let states = scheduler.get_account_states();
    let p01 = states.iter().find(|a| a.id == "p01").unwrap();
    assert_eq!(p01.health, gemini_openai_gateway::accounts::AccountHealth::Healthy);
}

#[test]
fn test_all_accounts_exhausted() {
    let accounts = vec![AccountConfig {
        id: "only_one".to_string(),
        secret_name: "KEY".to_string(),
        direct_key: None,
        quota_domain: "domain".to_string(),
        enabled: true,
        rpm_limit: None,
        tpm_limit: None,
        rpd_limit: None,
    }];
    let scheduler = LruQuotaScheduler::new(&accounts);

    scheduler.report_cooldown("only_one", "Rate limit", 60_000, 1000, "RATE_LIMIT_429");

    let res = scheduler.select_account(1001);
    assert!(res.is_err());
}
