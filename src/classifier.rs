use crate::upstream::UpstreamClient;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskDifficulty {
    Easy,
    Normal,
    Hard,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationResult {
    pub route: TaskDifficulty,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SmartRouterConfig {
    pub enabled: bool,
    pub classifier_model: String,
    pub lite_model: String,
    pub primary_model: String,
    pub min_confidence: f32,
}

impl Default for SmartRouterConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            classifier_model: "gemini-3.5-flash-lite".to_string(),
            lite_model: "gemini-3.5-flash-lite".to_string(),
            primary_model: "gemini-3.8-flash".to_string(),
            min_confidence: 0.6,
        }
    }
}

pub struct SmartRouter;

impl SmartRouter {
    pub fn extract_user_query(messages: &[serde_json::Value]) -> String {
        for msg in messages.iter().rev() {
            if let Some(role) = msg.get("role").and_then(|r| r.as_str()) {
                if role == "user" {
                    if let Some(content) = msg.get("content").and_then(|c| c.as_str()) {
                        let trimmed = content.trim();
                        if !trimmed.is_empty() {
                            return trimmed.chars().take(1200).collect();
                        }
                    }
                }
            }
        }
        String::new()
    }

    pub async fn classify(
        user_query: &str,
        client: &UpstreamClient,
        api_key: &str,
        classifier_model: &str,
    ) -> Option<ClassificationResult> {
        let trimmed = user_query.trim();
        if trimmed.is_empty() {
            return None;
        }

        // Fast-path heuristic: short greetings, formatting, or simple renames
        let q_lower = trimmed.to_lowercase();
        if trimmed.len() <= 35
            && (q_lower.contains("hi")
                || q_lower.contains("hello")
                || q_lower.contains("typo")
                || q_lower.contains("format")
                || q_lower.contains("rename"))
        {
            return Some(ClassificationResult {
                route: TaskDifficulty::Easy,
                confidence: 0.95,
            });
        }

        // Fast-path heuristic: obvious complex architectural / multi-file prompts
        if q_lower.contains("architecture")
            || q_lower.contains("architect")
            || q_lower.contains("multi-file")
            || q_lower.contains("entire project")
            || q_lower.contains("refactor whole")
        {
            return Some(ClassificationResult {
                route: TaskDifficulty::Hard,
                confidence: 0.95,
            });
        }

        let system_prompt = "Classify software task difficulty:\n- easy: typo, formatting, rename, simple extraction, tiny code change, simple question\n- normal: ordinary feature implementation, regular bug fix, moderate refactoring\n- hard: architecture, multi-file implementation, deep debugging, complex reasoning, autonomous workflow\nReturn JSON only: {\"route\":\"easy\"|\"normal\"|\"hard\",\"confidence\":0.0-1.0}";

        let payload = serde_json::json!({
            "model": classifier_model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": trimmed}
            ],
            "stream": false,
            "temperature": 0.0,
            "max_tokens": 40,
            "response_format": {"type": "json_object"}
        });

        let payload_str = payload.to_string();
        if let Ok(mut res) = client.send_chat_completion(api_key, &payload_str).await {
            if (200..300).contains(&res.status_code()) {
                if let Ok(body_text) = res.text().await {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&body_text) {
                        if let Some(content) = val
                            .get("choices")
                            .and_then(|c| c.get(0))
                            .and_then(|ch| ch.get("message"))
                            .and_then(|m| m.get("content"))
                            .and_then(|t| t.as_str())
                        {
                            if let Ok(parsed) = serde_json::from_str::<ClassificationResult>(content.trim()) {
                                return Some(parsed);
                            }
                        }
                    }
                }
            }
        }

        None
    }
}
