use crate::config::ModelConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u32>,
    #[serde(flatten)]
    pub extra_fields: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelItem {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub owned_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelListResponse {
    pub object: String,
    pub data: Vec<ModelItem>,
}

pub fn normalize_request(mut req: ChatCompletionRequest, model_config: &ModelConfig) -> ChatCompletionRequest {
    // 1. Model Resolution & Normalization
    let normalized_model = if model_config.force_model {
        model_config.primary.clone()
    } else if let Some(target) = model_config.aliases.get(&req.model) {
        target.clone()
    } else {
        req.model
    };
    req.model = normalized_model;

    // 2. Thinking / Reasoning Normalization
    // Native Gemini 3.8 reasoning_effort parameter ("high", "medium", "low", "none")
    // If not specified or if forcing high reasoning, default to model_config.thinking_level
    let reasoning = match req.reasoning_effort.as_deref() {
        Some("high") | Some("medium") | Some("low") | Some("none") => {
            if model_config.force_model {
                model_config.thinking_level.clone()
            } else {
                req.reasoning_effort.unwrap()
            }
        }
        _ => model_config.thinking_level.clone(),
    };
    req.reasoning_effort = Some(reasoning);

    req
}

pub fn build_model_list(model_config: &ModelConfig) -> ModelListResponse {
    let now = 1_726_000_000; // Reference timestamp
    let mut data = vec![
        ModelItem {
            id: model_config.primary.clone(),
            object: "model".to_string(),
            created: now,
            owned_by: "google".to_string(),
        },
    ];

    for alias in model_config.aliases.keys() {
        if alias != &model_config.primary {
            data.push(ModelItem {
                id: alias.clone(),
                object: "model".to_string(),
                created: now,
                owned_by: "google".to_string(),
            });
        }
    }

    ModelListResponse {
        object: "list".to_string(),
        data,
    }
}
