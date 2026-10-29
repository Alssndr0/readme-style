//! Model backends for `readme-style suggest`: the Anthropic Messages API, and any server that
//! speaks the `OpenAI` chat-completions API (local vLLM, Ollama, LM Studio, hosted gateways).
//!
//! Rust has no official Anthropic SDK, so both backends are plain HTTP with `ureq`.

use std::fmt;
use std::thread;
use std::time::Duration;

use serde_json::{Value, json};

/// How hard the model should think. Mapped to Anthropic's `effort`; not sent to other servers,
/// whose accepted values differ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Effort {
    Low,
    Medium,
}

impl Effort {
    fn id(self) -> &'static str {
        match self {
            Effort::Low => "low",
            Effort::Medium => "medium",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Backend {
    Anthropic {
        base_url: String,
        credential: Credential,
    },
    OpenAiCompatible {
        base_url: String,
        api_key: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Credential {
    ApiKey(String),
    /// An OAuth access token, sent as a bearer token.
    AuthToken(String),
}

/// A model to ask, and where.
#[derive(Debug, Clone)]
pub(crate) struct Model {
    pub(crate) backend: Backend,
    pub(crate) name: String,
}

impl Model {
    pub(crate) fn provider(&self) -> &'static str {
        match self.backend {
            Backend::Anthropic { .. } => "Anthropic",
            Backend::OpenAiCompatible { .. } => "OpenAI-compatible server",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LlmError {
    /// The credentials were rejected (401, 403).
    Auth(String),
    /// The model or endpoint does not exist (404).
    NotFound(String),
    /// The request was invalid (400, 422).
    BadRequest(String),
    /// Still rate limited or failing after retries.
    Unavailable(String),
    /// The server could not be reached.
    Connection(String),
    /// The model declined to answer.
    Refused(String),
    /// The answer was cut off at the output limit.
    Truncated,
    /// The response was not what the API documents.
    Malformed(String),
}

impl fmt::Display for LlmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LlmError::Auth(m) => write!(f, "the credentials were rejected: {m}"),
            LlmError::NotFound(m) => write!(f, "the model or endpoint was not found: {m}"),
            LlmError::BadRequest(m) => write!(f, "the request was rejected: {m}"),
            LlmError::Unavailable(m) => write!(f, "the service is unavailable: {m}"),
            LlmError::Connection(m) => write!(f, "could not reach the server: {m}"),
            LlmError::Refused(m) => write!(f, "the model declined to answer: {m}"),
            LlmError::Truncated => f.write_str("the answer was cut off at the output limit"),
            LlmError::Malformed(m) => write!(f, "unexpected response: {m}"),
        }
    }
}

/// Retries for rate limits, overload, server errors and dropped connections.
const RETRIES: u32 = 2;
const MAX_RETRY_WAIT: Duration = Duration::from_secs(60);
/// Thinking models can take minutes on a long README.
const TIMEOUT: Duration = Duration::from_secs(300);
const MAX_TOKENS: u32 = 16_000;
const ANTHROPIC_VERSION: &str = "2023-06-01";
/// Server-side refusal fallback: a declined request is re-run on a suitable model.
const FALLBACK_BETA: &str = "server-side-fallback-2026-07-01";
const OAUTH_BETA: &str = "oauth-2025-04-20";

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .http_status_as_error(false)
        .user_agent(concat!("readme-style/", env!("CARGO_PKG_VERSION")))
        .build()
        .into()
}

struct Reply {
    status: u16,
    retry_after: Option<Duration>,
    body: Value,
}

fn post(url: &str, headers: &[(&str, String)], payload: &Value) -> Result<Reply, LlmError> {
    let mut request = agent().post(url).header("content-type", "application/json");
    for (name, value) in headers {
        request = request.header(*name, value.as_str());
    }
    let mut response = request
        .send(payload.to_string())
        .map_err(|e| LlmError::Connection(e.to_string()))?;
    let status = response.status().as_u16();
    let retry_after = response
        .headers()
        .get("retry-after")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.trim().parse::<u64>().ok())
        .map(Duration::from_secs);
    let text = response
        .body_mut()
        .read_to_string()
        .map_err(|e| LlmError::Connection(e.to_string()))?;
    let body = serde_json::from_str(&text).unwrap_or(Value::String(text));
    Ok(Reply {
        status,
        retry_after,
        body,
    })
}

fn api_message(body: &Value) -> String {
    body.pointer("/error/message")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .or_else(|| body.as_str().map(|s| s.chars().take(300).collect()))
        .unwrap_or_else(|| body.to_string().chars().take(300).collect())
}

/// POST with retries on 408, 409, 429, 5xx (including Anthropic's 529) and connection errors.
fn post_with_retries(url: &str, headers: &[(&str, String)], payload: &Value) -> Result<Value, LlmError> {
    let mut attempt = 0;
    loop {
        let wait = |attempt: u32, hint: Option<Duration>| {
            hint.unwrap_or(Duration::from_secs(1 << attempt))
                .min(MAX_RETRY_WAIT)
        };
        match post(url, headers, payload) {
            Ok(reply) if (200..300).contains(&reply.status) => return Ok(reply.body),
            Ok(reply) => {
                let message = format!("HTTP {}: {}", reply.status, api_message(&reply.body));
                match reply.status {
                    401 | 403 => return Err(LlmError::Auth(message)),
                    404 => return Err(LlmError::NotFound(message)),
                    408 | 409 | 429 | 500..=599 if attempt < RETRIES => {
                        thread::sleep(wait(attempt, reply.retry_after));
                    }
                    408 | 409 | 429 | 500..=599 => return Err(LlmError::Unavailable(message)),
                    _ => return Err(LlmError::BadRequest(message)),
                }
            }
            Err(LlmError::Connection(_)) if attempt < RETRIES => thread::sleep(wait(attempt, None)),
            Err(error) => return Err(error),
        }
        attempt += 1;
    }
}

/// Ask `model` one question and return its text answer.
pub(crate) fn complete(model: &Model, system: &str, user: &str, effort: Effort) -> Result<String, LlmError> {
    match &model.backend {
        Backend::Anthropic { base_url, credential } => {
            anthropic(base_url, credential, &model.name, system, user, effort)
        }
        Backend::OpenAiCompatible { base_url, api_key } => {
            openai(base_url, api_key.as_deref(), &model.name, system, user)
        }
    }
}

fn anthropic(
    base_url: &str,
    credential: &Credential,
    model: &str,
    system: &str,
    user: &str,
    effort: Effort,
) -> Result<String, LlmError> {
    let mut headers = vec![("anthropic-version", ANTHROPIC_VERSION.to_owned())];
    let mut betas = vec![FALLBACK_BETA];
    match credential {
        Credential::ApiKey(key) => headers.push(("x-api-key", key.clone())),
        Credential::AuthToken(token) => {
            headers.push(("authorization", format!("Bearer {token}")));
            betas.push(OAUTH_BETA);
        }
    }
    headers.push(("anthropic-beta", betas.join(",")));
    // Thinking is adaptive by default on current models; effort sets how much.
    let payload = json!({
        "model": model,
        "max_tokens": MAX_TOKENS,
        "system": system,
        "messages": [{ "role": "user", "content": user }],
        "output_config": { "effort": effort.id() },
        "fallbacks": "default",
    });
    let url = format!("{}/v1/messages", base_url.trim_end_matches('/'));
    let body = post_with_retries(&url, &headers, &payload)?;
    anthropic_text(&body)
}

/// The text of a Messages API response, after checking why it stopped.
pub(crate) fn anthropic_text(body: &Value) -> Result<String, LlmError> {
    match body.get("stop_reason").and_then(Value::as_str) {
        Some("refusal") => {
            let details = body.get("stop_details");
            let category = details
                .and_then(|d| d.get("category"))
                .and_then(Value::as_str)
                .unwrap_or("unspecified");
            let explanation = details
                .and_then(|d| d.get("explanation"))
                .and_then(Value::as_str)
                .unwrap_or("");
            return Err(LlmError::Refused(
                format!("{category} {explanation}").trim().to_owned(),
            ));
        }
        Some("max_tokens") => return Err(LlmError::Truncated),
        _ => {}
    }
    let blocks = body
        .get("content")
        .and_then(Value::as_array)
        .ok_or_else(|| LlmError::Malformed("no content".into()))?;
    let text: Vec<&str> = blocks
        .iter()
        .filter(|b| b.get("type").and_then(Value::as_str) == Some("text"))
        .filter_map(|b| b.get("text").and_then(Value::as_str))
        .collect();
    if text.is_empty() {
        return Err(LlmError::Malformed("no text in the answer".into()));
    }
    Ok(text.join(""))
}

fn openai(
    base_url: &str,
    api_key: Option<&str>,
    model: &str,
    system: &str,
    user: &str,
) -> Result<String, LlmError> {
    let headers: Vec<(&str, String)> = api_key
        .map(|k| ("authorization", format!("Bearer {k}")))
        .into_iter()
        .collect();
    let payload = json!({
        "model": model,
        "max_tokens": MAX_TOKENS,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": user },
        ],
    });
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let body = post_with_retries(&url, &headers, &payload)?;
    openai_text(&body)
}

/// The text of a chat-completions response, without any `<think>` section a model inlines.
pub(crate) fn openai_text(body: &Value) -> Result<String, LlmError> {
    let choice = body
        .pointer("/choices/0")
        .ok_or_else(|| LlmError::Malformed("no choices".into()))?;
    if choice.get("finish_reason").and_then(Value::as_str) == Some("length") {
        return Err(LlmError::Truncated);
    }
    let content = choice
        .pointer("/message/content")
        .and_then(Value::as_str)
        .ok_or_else(|| LlmError::Malformed("no message content".into()))?;
    let content = match content.rfind("</think>") {
        Some(end) => &content[end + "</think>".len()..],
        None => content,
    };
    Ok(content.trim().to_owned())
}

/// The model a server offers, when it offers exactly one (typical of a local server).
pub(crate) fn only_model(base_url: &str, api_key: Option<&str>) -> Result<Option<String>, LlmError> {
    let mut request = agent().get(&format!("{}/models", base_url.trim_end_matches('/')));
    if let Some(key) = api_key {
        request = request.header("authorization", format!("Bearer {key}"));
    }
    let mut response = request.call().map_err(|e| LlmError::Connection(e.to_string()))?;
    if !response.status().is_success() {
        return Ok(None);
    }
    let text = response
        .body_mut()
        .read_to_string()
        .map_err(|e| LlmError::Connection(e.to_string()))?;
    let body: Value = serde_json::from_str(&text).map_err(|e| LlmError::Malformed(e.to_string()))?;
    let ids: Vec<&str> = body
        .get("data")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|m| m.get("id")?.as_str())
        .collect();
    Ok(match ids.as_slice() {
        [only] => Some((*only).to_owned()),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anthropic_text_checks_the_stop_reason() {
        let ok = json!({"stop_reason": "end_turn", "content": [{"type": "thinking", "thinking": ""}, {"type": "text", "text": "Hi"}]});
        assert_eq!(anthropic_text(&ok), Ok("Hi".into()));
        let refused = json!({"stop_reason": "refusal", "stop_details": {"type": "refusal", "category": "cyber", "explanation": "x"}, "content": []});
        assert_eq!(anthropic_text(&refused), Err(LlmError::Refused("cyber x".into())));
        let cut = json!({"stop_reason": "max_tokens", "content": [{"type": "text", "text": "partial"}]});
        assert_eq!(anthropic_text(&cut), Err(LlmError::Truncated));
    }

    #[test]
    fn openai_text_drops_inline_reasoning() {
        let body = json!({"choices": [{"finish_reason": "stop", "message": {"content": "<think>hmm</think>\n\nAnswer"}}]});
        assert_eq!(openai_text(&body), Ok("Answer".into()));
        let cut = json!({"choices": [{"finish_reason": "length", "message": {"content": "par"}}]});
        assert_eq!(openai_text(&cut), Err(LlmError::Truncated));
    }
}
