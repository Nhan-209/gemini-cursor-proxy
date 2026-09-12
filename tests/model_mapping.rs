use gemini_openai_gateway::config::ModelConfig;
use gemini_openai_gateway::models::{build_model_list, normalize_request, ChatCompletionRequest};
use std::collections::HashMap;

#[test]
fn test_force_model_maps_all_inputs_to_primary() {
    let mut cfg = ModelConfig::default(); // primary = "gemini-2.5-flash", force_model = false
    cfg.force_model = true;

    let req1 = ChatCompletionRequest {
        model: "unknown-model".to_string(),
        messages: vec![],
        stream: None,
        tools: None,
        tool_choice: None,
        reasoning_effort: None,
        temperature: None,
        top_p: None,
        max_tokens: None,
        max_completion_tokens: None,
        extra_fields: HashMap::new(),
    };
    let normalized1 = normalize_request(req1, &cfg);
    assert_eq!(normalized1.model, "gemini-2.5-flash");
}

#[test]
fn test_alias_model_mapping() {
    let cfg = ModelConfig::default();

    let req_gpt4o = ChatCompletionRequest {
        model: "gpt-4o".to_string(),
        messages: vec![],
        stream: None,
        tools: None,
        tool_choice: None,
        reasoning_effort: None,
        temperature: None,
        top_p: None,
        max_tokens: None,
        max_completion_tokens: None,
        extra_fields: HashMap::new(),
    };
    assert_eq!(normalize_request(req_gpt4o, &cfg).model, "gemini-2.5-flash");

    let req_claude = ChatCompletionRequest {
        model: "claude-3-7-sonnet".to_string(),
        messages: vec![],
        stream: None,
        tools: None,
        tool_choice: None,
        reasoning_effort: None,
        temperature: None,
        top_p: None,
        max_tokens: None,
        max_completion_tokens: None,
        extra_fields: HashMap::new(),
    };
    assert_eq!(normalize_request(req_claude, &cfg).model, "gemini-2.5-pro");

    let req_mini = ChatCompletionRequest {
        model: "gpt-4o-mini".to_string(),
        messages: vec![],
        stream: None,
        tools: None,
        tool_choice: None,
        reasoning_effort: None,
        temperature: None,
        top_p: None,
        max_tokens: None,
        max_completion_tokens: None,
        extra_fields: HashMap::new(),
    };
    assert_eq!(normalize_request(req_mini, &cfg).model, "gemini-2.0-flash-lite");
}

#[test]
fn test_reasoning_effort_stripped_for_upstream_safety() {
    let cfg = ModelConfig::default();

    let req = ChatCompletionRequest {
        model: "gemini-2.5-flash".to_string(),
        messages: vec![],
        stream: None,
        tools: None,
        tool_choice: None,
        reasoning_effort: Some("high".to_string()),
        temperature: None,
        top_p: None,
        max_tokens: None,
        max_completion_tokens: None,
        extra_fields: HashMap::new(),
    };
    let normalized = normalize_request(req, &cfg);
    assert_eq!(normalized.reasoning_effort, None);
}

#[test]
fn test_build_model_list_contains_primary() {
    let cfg = ModelConfig::default();
    let list = build_model_list(&cfg);

    assert_eq!(list.object, "list");
    let model_ids: Vec<String> = list.data.into_iter().map(|m| m.id).collect();
    assert!(model_ids.contains(&"gemini-2.5-flash".to_string()));
    assert!(model_ids.contains(&"gemini-2.5-pro".to_string()));
    assert!(model_ids.contains(&"gemini-2.0-flash".to_string()));
    assert!(model_ids.contains(&"gemini-2.0-flash-lite".to_string()));
}
