use gemini_cursor_proxy::config::ModelConfig;
use gemini_cursor_proxy::models::{build_model_list, normalize_request, ChatCompletionRequest};
use std::collections::HashMap;

#[test]
fn test_force_model_maps_all_inputs_to_primary() {
    let cfg = ModelConfig::default(); // primary = "gemini-3.8-flash", force_model = true

    let req1 = ChatCompletionRequest {
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
    let normalized1 = normalize_request(req1, &cfg);
    assert_eq!(normalized1.model, "gemini-3.8-flash");

    let req2 = ChatCompletionRequest {
        model: "auto".to_string(),
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
    let normalized2 = normalize_request(req2, &cfg);
    assert_eq!(normalized2.model, "gemini-3.8-flash");
}

#[test]
fn test_thinking_level_default_high_injection() {
    let cfg = ModelConfig::default(); // thinking_level = "high"

    let req = ChatCompletionRequest {
        model: "gemini-3.8-flash".to_string(),
        messages: vec![],
        stream: None,
        tools: None,
        tool_choice: None,
        reasoning_effort: None, // unspecified by client
        temperature: None,
        top_p: None,
        max_tokens: None,
        max_completion_tokens: None,
        extra_fields: HashMap::new(),
    };
    let normalized = normalize_request(req, &cfg);
    assert_eq!(normalized.reasoning_effort, Some("high".to_string()));
}

#[test]
fn test_build_model_list_contains_primary() {
    let cfg = ModelConfig::default();
    let list = build_model_list(&cfg);

    assert_eq!(list.object, "list");
    let model_ids: Vec<String> = list.data.into_iter().map(|m| m.id).collect();
    assert!(model_ids.contains(&"gemini-3.8-flash".to_string()));
}
