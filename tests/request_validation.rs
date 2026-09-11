use gemini_openai_gateway::models::ChatCompletionRequest;

#[test]
fn test_chat_completion_request_deserialization() {
    let payload = r#"{
        "model": "gpt-4o",
        "messages": [
            {"role": "system", "content": "You are a coding assistant."},
            {"role": "user", "content": "Refactor this function."},
            {"role": "assistant", "content": "Here is the refactored code."},
            {"role": "tool", "tool_call_id": "call_123", "content": "{\"status\":\"ok\"}"}
        ],
        "stream": true,
        "temperature": 0.2,
        "tools": [
            {
                "type": "function",
                "function": {
                    "name": "edit_file",
                    "description": "Edits a file"
                }
            }
        ]
    }"#;

    let req: ChatCompletionRequest = serde_json::from_str(payload).expect("Should deserialize");
    assert_eq!(req.model, "gpt-4o");
    assert_eq!(req.messages.len(), 4);
    assert_eq!(req.stream, Some(true));
    assert_eq!(req.temperature, Some(0.2));
    assert!(req.tools.is_some());
}

#[test]
fn test_extra_unknown_fields_preserved() {
    let payload = r#"{
        "model": "auto",
        "messages": [
            {"role": "user", "content": "Hello"}
        ],
        "custom_client_metadata": {
            "ide": "cursor",
            "version": "0.45"
        }
    }"#;

    let req: ChatCompletionRequest = serde_json::from_str(payload).expect("Should deserialize with extra fields");
    assert!(req.extra_fields.contains_key("custom_client_metadata"));

    let re_serialized = serde_json::to_string(&req).expect("Should re-serialize");
    assert!(re_serialized.contains("custom_client_metadata"));
}
