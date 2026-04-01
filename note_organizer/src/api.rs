use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::env;

const API_URL: &str = "https://api.anthropic.com/v1/messages";
// Haiku is the lowest-cost Claude model ($1.00 input / $5.00 output per 1M tokens).
// All four agents use it to keep per-operation costs in the fractions-of-a-cent range.
const MODEL: &str = "claude-haiku-4-5";

#[derive(Debug)]
pub enum ApiError {
    Http(String),
    Parse(String),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::Http(s) => write!(f, "HTTP error: {}", s),
            ApiError::Parse(s) => write!(f, "Parse error: {}", s),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Message {
    pub role: String,
    pub content: String,
}

/// Send a message to Claude and return the text response.
pub async fn call_claude(
    system: &str,
    messages: Vec<Message>,
    max_tokens: u32,
) -> Result<String, ApiError> {
    let api_key = env::var("ANTHROPIC_API_KEY")
        .map_err(|_| ApiError::Http("ANTHROPIC_API_KEY not set".to_string()))?;

    let client = Client::new();

    let msgs: Vec<Value> = messages
        .into_iter()
        .map(|m| json!({"role": m.role, "content": m.content}))
        .collect();

    let body = json!({
        "model": MODEL,
        "max_tokens": max_tokens,
        "system": system,
        "messages": msgs
    });

    let resp = client
        .post(API_URL)
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| ApiError::Http(e.to_string()))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(ApiError::Http(format!("{}: {}", status, text)));
    }

    let data: Value = resp.json().await.map_err(|e| ApiError::Parse(e.to_string()))?;

    // Extract text from content blocks (skip thinking blocks)
    let text = data["content"]
        .as_array()
        .and_then(|blocks| {
            blocks.iter()
                .filter(|b| b["type"].as_str() == Some("text"))
                .filter_map(|b| b["text"].as_str())
                .next()
        })
        .ok_or_else(|| ApiError::Parse("No text content in response".to_string()))?;

    Ok(text.to_string())
}

/// Convenience wrapper for a single user message.
pub async fn ask(system: &str, user_message: &str, max_tokens: u32) -> Result<String, ApiError> {
    call_claude(
        system,
        vec![Message {
            role: "user".to_string(),
            content: user_message.to_string(),
        }],
        max_tokens,
    )
    .await
}
