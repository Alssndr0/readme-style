//! `readme-style suggest` against a local mock server: the requests it sends and how it reads
//! answers, refusals, retries and errors. No network is used.

use std::collections::VecDeque;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::{Value, json};
use tempfile::TempDir;

#[derive(Debug, Clone)]
struct Request {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: Value,
}

impl Request {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }
}

/// A server that answers each request with the next canned response and records what it got.
struct Mock {
    url: String,
    requests: Arc<Mutex<Vec<Request>>>,
}

fn mock(responses: Vec<(u16, Value)>) -> Mock {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let url = format!("http://{}", listener.local_addr().expect("address"));
    let requests = Arc::new(Mutex::new(Vec::new()));
    let recorded = Arc::clone(&requests);
    let mut queue: VecDeque<(u16, Value)> = responses.into();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { break };
            let mut reader = BufReader::new(stream.try_clone().expect("clone"));
            let mut line = String::new();
            reader.read_line(&mut line).expect("request line");
            let mut parts = line.split_whitespace();
            let method = parts.next().unwrap_or_default().to_owned();
            let path = parts.next().unwrap_or_default().to_owned();
            let mut headers = Vec::new();
            loop {
                let mut header = String::new();
                reader.read_line(&mut header).expect("header");
                let header = header.trim_end();
                if header.is_empty() {
                    break;
                }
                if let Some((k, v)) = header.split_once(':') {
                    headers.push((k.trim().to_ascii_lowercase(), v.trim().to_owned()));
                }
            }
            let length: usize = headers
                .iter()
                .find(|(k, _)| k == "content-length")
                .and_then(|(_, v)| v.parse().ok())
                .unwrap_or(0);
            let mut body = vec![0; length];
            reader.read_exact(&mut body).expect("body");
            let body = serde_json::from_slice(&body).unwrap_or(Value::Null);
            recorded.lock().expect("lock").push(Request {
                method,
                path,
                headers,
                body,
            });
            let (status, reply) = queue
                .pop_front()
                .unwrap_or((500, json!({"error": {"message": "no more responses"}})));
            let reply = reply.to_string();
            let _ = write!(
                stream,
                "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\nretry-after: 0\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{reply}",
                reply.len()
            );
        }
    });
    Mock { url, requests }
}

fn project() -> TempDir {
    let dir = TempDir::new().expect("temp dir");
    fs::create_dir(dir.path().join(".github")).expect("root marker");
    fs::write(
        dir.path().join("README.md"),
        "# demo\n\nA thing that parses configs.\n",
    )
    .expect("readme");
    dir
}

fn cli(dir: &TempDir) -> Command {
    let mut command = Command::cargo_bin("readme-style").expect("binary");
    command.current_dir(dir.path()).arg("--color").arg("never");
    for name in [
        "ANTHROPIC_API_KEY",
        "ANTHROPIC_AUTH_TOKEN",
        "ANTHROPIC_BASE_URL",
        "OPENAI_API_KEY",
        "OPENAI_BASE_URL",
    ] {
        command.env_remove(name);
    }
    command
}

fn anthropic_answer(text: &str) -> Value {
    json!({"type": "message", "stop_reason": "end_turn", "content": [{"type": "thinking", "thinking": ""}, {"type": "text", "text": text}]})
}

#[test]
fn anthropic_requests_follow_the_messages_api() {
    let dir = project();
    let server = mock(vec![(
        200,
        anthropic_answer("1. Parse configs fast\n2. Configs without surprises\n3. One parser"),
    )]);
    cli(&dir)
        .args(["suggest", "tagline"])
        .env("ANTHROPIC_API_KEY", "test-key")
        .env("ANTHROPIC_BASE_URL", &server.url)
        .assert()
        .success()
        .stdout(
            predicate::str::contains("1. Parse configs fast")
                .and(predicate::str::contains("claude-opus-5-5 via Anthropic")),
        );
    let requests = server.requests.lock().expect("lock");
    let request = &requests[0];
    assert_eq!(
        (request.method.as_str(), request.path.as_str()),
        ("POST", "/v1/messages")
    );
    assert_eq!(request.header("x-api-key"), Some("test-key"));
    assert_eq!(request.header("anthropic-version"), Some("2023-06-01"));
    assert_eq!(
        request.header("anthropic-beta"),
        Some("server-side-fallback-2026-07-01")
    );
    assert_eq!(request.body["model"], "claude-opus-5-5");
    assert_eq!(request.body["fallbacks"], "default");
    assert_eq!(request.body["output_config"]["effort"], "low");
    assert!(
        request.body.get("thinking").is_none(),
        "adaptive thinking is the default"
    );
    assert!(
        request.body["messages"][0]["content"]
            .as_str()
            .is_some_and(|c| c.contains("A thing that parses configs."))
    );
}

#[test]
fn refusals_and_auth_errors_are_reported() {
    let dir = project();
    let refusal = json!({"stop_reason": "refusal", "stop_details": {"type": "refusal", "category": "cyber", "explanation": null}, "content": []});
    let server = mock(vec![(200, refusal)]);
    cli(&dir)
        .args(["suggest", "tagline"])
        .env("ANTHROPIC_API_KEY", "k")
        .env("ANTHROPIC_BASE_URL", &server.url)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("declined to answer: cyber"));
    let server = mock(vec![(
        401,
        json!({"type": "error", "error": {"type": "authentication_error", "message": "invalid x-api-key"}}),
    )]);
    cli(&dir)
        .args(["suggest", "tagline"])
        .env("ANTHROPIC_API_KEY", "bad")
        .env("ANTHROPIC_BASE_URL", &server.url)
        .assert()
        .code(2)
        .stderr(predicate::str::contains(
            "credentials were rejected: HTTP 401: invalid x-api-key",
        ));
}

#[test]
fn rate_limits_are_retried() {
    let dir = project();
    let server = mock(vec![
        (429, json!({"error": {"message": "slow down"}})),
        (529, json!({"error": {"message": "overloaded"}})),
        (200, anthropic_answer("Parse configs fast")),
    ]);
    cli(&dir)
        .args(["suggest", "tagline", "--format", "json"])
        .env("ANTHROPIC_API_KEY", "k")
        .env("ANTHROPIC_BASE_URL", &server.url)
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "\"suggestions\": [\n    \"Parse configs fast\"",
        ));
    assert_eq!(server.requests.lock().expect("lock").len(), 3);
}

#[test]
fn openai_compatible_servers_and_their_only_model() {
    let dir = project();
    let models = json!({"object": "list", "data": [{"id": "local/qwen"}]});
    let answer = json!({"choices": [{"finish_reason": "stop", "message": {"content": "<think>ok</think>\n```mermaid\nflowchart LR\n  Parser --> Dict\n```"}}]});
    let server = mock(vec![(200, models), (200, answer)]);
    cli(&dir)
        .args(["suggest", "diagram", "--base-url", &format!("{}/v1", server.url)])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("```mermaid\nflowchart LR\n  Parser --> Dict\n```")
                .and(predicate::str::contains("local/qwen")),
        );
    let requests = server.requests.lock().expect("lock");
    assert_eq!(requests[0].path, "/v1/models");
    assert_eq!(requests[1].path, "/v1/chat/completions");
    assert_eq!(requests[1].body["model"], "local/qwen");
    assert_eq!(requests[1].body["messages"][0]["role"], "system");
    assert!(requests[1].header("authorization").is_none(), "no key, no header");
}

#[test]
fn without_a_model_it_says_how_to_configure_one() {
    let dir = project();
    cli(&dir)
        .args(["suggest", "tagline"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("no model configured"));
}

#[test]
fn the_ai_table_chooses_provider_and_model_only() {
    let dir = project();
    let server = mock(vec![(
        200,
        json!({"choices": [{"finish_reason": "stop", "message": {"content": "Configs, parsed"}}]}),
    )]);
    fs::write(
        dir.path().join(".readme-style.toml"),
        "[ai]\nprovider = \"openai\"\nmodel = \"m\"\n",
    )
    .expect("config");
    cli(&dir)
        .args(["suggest", "tagline"])
        .env("OPENAI_BASE_URL", format!("{}/v1", server.url))
        .env("OPENAI_API_KEY", "secret")
        .assert()
        .success()
        .stdout(predicate::str::contains("1. Configs, parsed"));
    let requests = server.requests.lock().expect("lock");
    assert_eq!(requests[0].body["model"], "m");
    assert_eq!(requests[0].header("authorization"), Some("Bearer secret"));
    drop(requests);
    // The [ai] table does not disturb styling.
    cli(&dir).arg("--check").assert().success();
}

#[test]
fn a_repository_cannot_redirect_requests_or_pick_the_key() {
    let dir = project();
    for key in [
        "base-url = \"https://example.invalid\"",
        "api-key-env = \"GITHUB_TOKEN\"",
    ] {
        fs::write(
            dir.path().join(".readme-style.toml"),
            format!("[ai]\nprovider = \"openai\"\n{key}\n"),
        )
        .expect("config");
        cli(&dir)
            .args(["suggest", "tagline"])
            .env("GITHUB_TOKEN", "do-not-send")
            .assert()
            .code(2)
            .stderr(predicate::str::contains(
                "cannot be set in a project's configuration",
            ));
    }
}
