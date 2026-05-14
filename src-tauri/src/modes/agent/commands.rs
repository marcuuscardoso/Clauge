use crate::modes::agent::models::{AgentContext, AgentSession};
use crate::shared::cli::{registry::runner_for, runner::CliRunner};
use crate::shared::repos::sessions as sessions_repo;
use sqlx::SqlitePool;
use std::path::PathBuf;
use tauri::{Manager, State};

fn project_name_from_path(path: &str) -> String {
    std::path::Path::new(path).file_name().and_then(|n| n.to_str()).unwrap_or("Unknown").to_string()
}

#[tauri::command]
pub async fn agent_list_sessions(pool: State<'_, SqlitePool>) -> Result<Vec<AgentSession>, String> {
    sessions_repo::list_sessions(pool.inner()).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn agent_create_session(
    pool: State<'_, SqlitePool>,
    title: String, purpose: String, project_path: String,
    skip_permissions: Option<bool>, custom_prompt: Option<String>,
    git_name: Option<String>, git_email: Option<String>,
    provider: Option<String>,
) -> Result<AgentSession, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let project_name = project_name_from_path(&project_path);
    let context_prompt = custom_prompt.unwrap_or_default();
    let skip = if skip_permissions.unwrap_or(false) { 1 } else { 0 };
    // Default to Claude when the frontend doesn't pass a provider —
    // preserves behaviour for legacy callers; unknown ids also fall
    // back via `runner_for`. The string is persisted as-is so future
    // providers slot in without a column change.
    let provider = provider
        .filter(|p| !p.trim().is_empty())
        .unwrap_or_else(|| "claude".to_string());
    sessions_repo::insert_session(
        pool.inner(),
        &id,
        &title,
        &purpose,
        &project_path,
        &project_name,
        &context_prompt,
        skip,
        git_name.as_deref(),
        git_email.as_deref(),
        &now,
        &now,
        &provider,
    )
    .await
    .map_err(|e| e.to_string())?;
    // Sessions are machine-local — only agent_contexts travel through
    // cloud sync, so no `bump("agent")` here.

    // Lazy MCP registration for non-Claude providers. Boot does NOT
    // auto-register these (would touch ~/.codex/config.toml or
    // ~/.config/opencode/opencode.json for every alpha tester who has
    // those CLIs but uses Clauge as Claude-only). Triggered here so the
    // user has explicitly opted in by creating a session in that
    // provider. Best-effort; silent on failure.
    crate::modes::workspace::commands::ensure_provider_mcp_registered(
        pool.inner(),
        &provider,
    ).await;

    sessions_repo::get_session_by_id(pool.inner(), &id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn agent_update_session(
    pool: State<'_, SqlitePool>, id: String,
    title: Option<String>, skip_permissions: Option<bool>,
    git_name: Option<String>, git_email: Option<String>, context_prompt: Option<String>,
) -> Result<(), String> {
    if let Some(t) = title {
        sessions_repo::update_session_title(pool.inner(), &id, &t).await.map_err(|e| e.to_string())?;
    }
    if let Some(sp) = skip_permissions {
        let val = if sp { 1 } else { 0 };
        sessions_repo::update_session_skip_permissions(pool.inner(), &id, val).await.map_err(|e| e.to_string())?;
    }
    if let Some(ref name) = git_name {
        sessions_repo::update_session_git_name(pool.inner(), &id, name).await.map_err(|e| e.to_string())?;
    }
    if let Some(ref email) = git_email {
        sessions_repo::update_session_git_email(pool.inner(), &id, email).await.map_err(|e| e.to_string())?;
    }
    if let Some(ref prompt) = context_prompt {
        sessions_repo::update_session_context_prompt(pool.inner(), &id, prompt).await.map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn agent_delete_session(pool: State<'_, SqlitePool>, id: String) -> Result<(), String> {
    sessions_repo::delete_session(pool.inner(), &id).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn agent_update_session_id(pool: State<'_, SqlitePool>, id: String, claude_session_id: String) -> Result<(), String> {
    let val = if claude_session_id.is_empty() { None } else { Some(claude_session_id) };
    sessions_repo::update_session_claude_id(pool.inner(), &id, val.as_deref()).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn agent_update_last_used(pool: State<'_, SqlitePool>, id: String) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    sessions_repo::update_session_last_used(pool.inner(), &id, &now).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn agent_update_worktree(pool: State<'_, SqlitePool>, id: String, worktree_path: Option<String>, worktree_branch: Option<String>) -> Result<(), String> {
    sessions_repo::update_session_worktree(pool.inner(), &id, worktree_path.as_deref(), worktree_branch.as_deref()).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn agent_list_contexts(pool: State<'_, SqlitePool>) -> Result<Vec<AgentContext>, String> {
    sessions_repo::list_contexts(pool.inner()).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn agent_save_context(pool: State<'_, SqlitePool>, id: Option<String>, name: String, content: String) -> Result<AgentContext, String> {
    let now = chrono::Utc::now().to_rfc3339();
    let ctx_id = id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    sessions_repo::upsert_context(pool.inner(), &ctx_id, &name, &content, &now, &now)
        .await
        .map_err(|e| e.to_string())?;
    crate::cloud::scheduler::bump("agent");
    sessions_repo::get_context_by_id(pool.inner(), &ctx_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn agent_delete_context(pool: State<'_, SqlitePool>, id: String) -> Result<(), String> {
    sessions_repo::delete_context(pool.inner(), &id).await.map_err(|e| e.to_string())?;
    crate::cloud::scheduler::bump("agent");
    Ok(())
}

#[tauri::command]
pub async fn agent_get_session_contexts(pool: State<'_, SqlitePool>, session_id: String) -> Result<Vec<AgentContext>, String> {
    sessions_repo::list_contexts_for_session(pool.inner(), &session_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn agent_attach_context(pool: State<'_, SqlitePool>, session_id: String, context_id: String) -> Result<(), String> {
    sessions_repo::attach_context_to_session(pool.inner(), &session_id, &context_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn agent_detach_context(pool: State<'_, SqlitePool>, session_id: String, context_id: String) -> Result<(), String> {
    sessions_repo::detach_context_from_session(pool.inner(), &session_id, &context_id).await.map_err(|e| e.to_string())
}

// Each CLI reads its own project-level context file. Inject writes to
// exactly one file based on the session's provider so we don't pollute
// the user's repo with files no agent will read. Remove cleans the
// marker block from every known file defensively — handles the case
// where the user attached the same context to sessions under
// different providers in the same project.
const CTX_MARKER_START: &str = "<!-- CLAUGE-CONTEXT-START -->";
const CTX_MARKER_END: &str = "<!-- CLAUGE-CONTEXT-END -->";
const ALL_CONTEXT_FILES: &[&str] = &["CLAUDE.md", "AGENTS.md", "GEMINI.md"];

fn context_file_for(provider: &str) -> &'static str {
    match provider {
        "codex" | "opencode" => "AGENTS.md",
        "gemini" => "GEMINI.md",
        _ => "CLAUDE.md",
    }
}

fn write_injected_context(path: &PathBuf, contexts: &[(String, String)]) -> Result<(), String> {
    let existing_content = if path.exists() {
        let raw = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        if let (Some(start), Some(_end)) = (raw.find(CTX_MARKER_START), raw.find(CTX_MARKER_END)) {
            raw[..start].trim_end().to_string()
        } else {
            raw
        }
    } else {
        String::new()
    };

    // Filter out snippets whose content already exists in the file
    let mut filtered = String::new();
    for (name, content) in contexts {
        if !existing_content.contains(content.trim()) {
            if !filtered.is_empty() { filtered.push_str("\n\n---\n\n"); }
            filtered.push_str(&format!("## {}\n\n{}", name, content));
        }
    }
    if filtered.is_empty() { return Ok(()); }

    let injected = format!("\n\n{}\n{}\n{}\n", CTX_MARKER_START, filtered, CTX_MARKER_END);
    if !existing_content.is_empty() {
        std::fs::write(path, format!("{}{}", existing_content.trim_end(), injected)).map_err(|e| e.to_string())?;
    } else {
        std::fs::write(path, filtered).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn agent_inject_contexts(
    pool: State<'_, SqlitePool>,
    project_path: String,
    context_ids: Vec<String>,
    provider: Option<String>,
) -> Result<(), String> {
    if context_ids.is_empty() { return Ok(()); }

    // Fetch context content from DB by ID
    let mut contexts: Vec<(String, String)> = Vec::new();
    for id in &context_ids {
        let row = sessions_repo::get_context_name_and_content(pool.inner(), id)
            .await
            .map_err(|e| e.to_string())?;
        if let Some(ctx) = row {
            contexts.push(ctx);
        }
    }
    if contexts.is_empty() { return Ok(()); }

    // Write to the single file the session's CLI actually reads.
    // Claude → CLAUDE.md, Codex / OpenCode → AGENTS.md.
    let filename = context_file_for(provider.as_deref().unwrap_or("claude"));
    let path = PathBuf::from(&project_path).join(filename);
    write_injected_context(&path, &contexts)
}

#[tauri::command]
pub fn agent_remove_injected_contexts(project_path: String) -> Result<(), String> {
    // Defensive sweep: if the user attached contexts under one provider
    // then later switched the session's provider, the previous file
    // would otherwise be orphaned. Clean every known context file's
    // marker block — no-op on files that don't exist.
    for filename in ALL_CONTEXT_FILES {
        let path = PathBuf::from(&project_path).join(filename);
        if !path.exists() { continue; }

        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        if let (Some(start), Some(end)) = (content.find(CTX_MARKER_START), content.find(CTX_MARKER_END)) {
            let cleaned = format!("{}{}", &content[..start].trim_end(), &content[end + CTX_MARKER_END.len()..]);
            if cleaned.trim().is_empty() {
                let _ = std::fs::remove_file(&path);
            } else {
                std::fs::write(&path, cleaned.trim_end().to_string() + "\n").map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub fn agent_update_tray_title(app_handle: tauri::AppHandle, title: String) -> Result<(), String> {
    if let Some(tray) = app_handle.tray_by_id(&tauri::tray::TrayIconId::new("main-tray")) {
        tray.set_title(Some(&title)).map_err(|e| format!("Tray error: {}", e))?;
    }
    Ok(())
}

#[tauri::command]
pub fn agent_check_claude_installed() -> bool {
    // Preserved as a Claude-specific name for the original callers; the
    // provider-aware variant below is the new shape going forward.
    let cli: &dyn CliRunner = runner_for("claude");
    cli.resolve_binary_path() != cli.binary_name()
}

#[tauri::command]
pub fn agent_check_cli_installed(provider: String) -> bool {
    // Used by the provider picker (NewSessionModal / coworker modal) to
    // grey out CLIs that aren't on PATH. `resolve_binary_path` returns
    // the bare binary name when `which` / `where.exe` fails, so a
    // distinct absolute path means the binary is installed.
    let cli: &dyn CliRunner = runner_for(&provider);
    cli.resolve_binary_path() != cli.binary_name()
}

#[tauri::command]
pub fn agent_get_claude_plan() -> Result<String, String> {
    let output = std::process::Command::new("security")
        .args(["find-generic-password", "-s", "Claude Code-credentials", "-w"])
        .output()
        .map_err(|e| format!("Keychain error: {}", e))?;
    if !output.status.success() {
        return Ok(String::new());
    }
    let json_str = String::from_utf8(output.stdout).map_err(|e| e.to_string())?;
    let parsed: serde_json::Value =
        serde_json::from_str(json_str.trim()).map_err(|e| e.to_string())?;
    Ok(parsed
        .get("claudeAiOauth")
        .and_then(|o| o.get("subscriptionType").and_then(|v| v.as_str()))
        .unwrap_or("")
        .to_string())
}
