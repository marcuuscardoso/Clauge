use reqwest::header::{HeaderValue, CONTENT_TYPE};
use sqlx::SqlitePool;
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncBufReadExt;
use tokio_stream::StreamExt;
use uuid::Uuid;

use crate::shared::ai::types::ChatContext;

pub async fn stream_clauge_ai(
    client: &reqwest::Client,
    app: &AppHandle,
    _pool: &SqlitePool,
    cloud_token: &str,
    conversation_msgs: Vec<serde_json::Value>,
    context: &ChatContext,
    session_id: &str,
) -> Result<(), String> {
    let _ = context;
    let request_id = Uuid::new_v4().to_string();
    let body = serde_json::json!({
        "messages": conversation_msgs,
        "request_id": request_id,
    });

    let response = client
        .post("https://clauge.in/api/ai/chat")
        .header(
            "Authorization",
            HeaderValue::from_str(&format!("Bearer {}", cloud_token))
                .map_err(|e| e.to_string())?,
        )
        .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            let msg = format!("Connection failed: {}", e);
            let _ = app.emit(
                &format!("ai:error:{}", session_id),
                serde_json::json!({"error": msg}),
            );
            msg
        })?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let error_body = response.text().await.unwrap_or_default();
        let msg = match status {
            401 => "Not signed in or session expired — please sign in again".to_string(),
            402 => "Insufficient Clauge AI credits".to_string(),
            429 => "Rate limited — please try again in a moment".to_string(),
            _ => {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&error_body) {
                    parsed["error"]
                        .as_str()
                        .unwrap_or("Unknown error")
                        .to_string()
                } else {
                    format!("Clauge AI error ({})", status)
                }
            }
        };
        let _ = app.emit(
            &format!("ai:error:{}", session_id),
            serde_json::json!({"error": msg}),
        );
        return Err(msg);
    }

    let byte_stream = response.bytes_stream();
    let stream_reader = tokio_util::io::StreamReader::new(
        byte_stream.map(|r| r.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))),
    );
    let mut lines = tokio::io::BufReader::new(stream_reader).lines();

    let mut current_event: Option<String> = None;

    while let Some(line) = lines.next_line().await.map_err(|e| e.to_string())? {
        let line = line.trim().to_string();

        if line.is_empty() {
            current_event = None;
            continue;
        }

        if let Some(rest) = line.strip_prefix("event: ") {
            current_event = Some(rest.to_string());
            continue;
        }

        if let Some(rest) = line.strip_prefix("data: ") {
            let data = rest;
            if data == "[DONE]" {
                let _ = app.emit(
                    &format!("ai:done:{}", session_id),
                    serde_json::json!({"inputTokens": 0, "outputTokens": 0}),
                );
                break;
            }

            match current_event.as_deref() {
                Some("balance") => {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                        if let Some(remaining) = parsed.get("remaining").and_then(|v| v.as_i64()) {
                            let _ = app.emit(
                                "clauge_ai:balance",
                                serde_json::json!({"remaining": remaining}),
                            );
                        }
                    }
                }
                _ => {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                        let choice = &parsed["choices"][0];
                        let delta = &choice["delta"];
                        if let Some(text) = delta["content"].as_str() {
                            if !text.is_empty() {
                                let _ = app.emit(
                                    &format!("ai:text:{}", session_id),
                                    serde_json::json!({"text": text}),
                                );
                            }
                        }
                        if choice["finish_reason"].as_str() == Some("stop") {
                            let input_tokens = parsed["usage"]["prompt_tokens"].as_u64().unwrap_or(0);
                            let output_tokens = parsed["usage"]["completion_tokens"].as_u64().unwrap_or(0);
                            let _ = app.emit(
                                &format!("ai:done:{}", session_id),
                                serde_json::json!({
                                    "inputTokens": input_tokens,
                                    "outputTokens": output_tokens,
                                }),
                            );
                        }
                    }
                }
            }

            current_event = None;
        }
    }

    Ok(())
}
