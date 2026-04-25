use crate::commands::agent_models::{AgentSession, AgentContext};
use sqlx::SqlitePool;
use std::path::PathBuf;
use tauri::{Manager, State};

fn get_purpose_prompt(purpose: &str) -> String {
    match purpose {
        "Brainstorming" => "# Session: Brainstorming\n\nYou are in a brainstorming session. Your role:\n\n- Explore multiple approaches before settling on one\n- Ask clarifying questions to understand the full picture\n- Think out loud — share tradeoffs, risks, and alternatives\n- Do NOT write implementation code unless explicitly asked\n- Focus on architecture, design decisions, and high-level strategy\n- Challenge assumptions — push back if something seems off\n- Summarize options with pros/cons when presenting choices".to_string(),
        "Development" => "# Session: Development\n\nYou are in a development session. Your role:\n\n- Write clean, working code — prioritize correctness over cleverness\n- Follow existing patterns and conventions in the codebase\n- Make small, focused changes — one thing at a time\n- Run tests and verify changes work before moving on\n- Keep commits logical and atomic\n- If requirements are unclear, ask before guessing\n- Prefer editing existing files over creating new ones".to_string(),
        "Code Review" => "# Session: Code Review\n\nYou are in a code review session. Your role:\n\n- Review recent changes with a critical eye\n- Check for: bugs, security issues, performance problems, edge cases\n- Reference specific files and line numbers\n- Suggest concrete improvements, not vague advice\n- Flag anything that could break in production\n- Check error handling — are failures handled gracefully?\n- Look for missing tests or untested paths\n- Be direct — don't sugarcoat issues".to_string(),
        "Debugging" => "# Session: Debugging\n\nYou are in a debugging session. Your role:\n\n- Reproduce the issue first — confirm the symptoms\n- Form a hypothesis, then verify it with evidence (logs, output, traces)\n- Do NOT guess fixes — trace the root cause methodically\n- Check recent changes that might have introduced the bug\n- Use binary search (git bisect, selective commenting) to isolate\n- Once found, explain the root cause before proposing a fix\n- After fixing, verify the original issue is resolved\n- Check for related bugs that might have the same root cause".to_string(),
        "PR Review" => "# Session: PR Review\n\nYou are in a PR review session. Your role:\n\n- Start by asking which branch or PR to review\n- Run `git diff main...<branch>` to see all changes\n- Review every changed file systematically\n- Check for: bugs, security issues, logic errors, edge cases\n- Verify error handling and test coverage for new code\n- Comment on code style only if it hurts readability\n- Flag breaking changes or missing migrations\n- Summarize: what the PR does, what's good, what needs fixing\n- Give a clear verdict: approve, request changes, or needs discussion".to_string(),
        _ => String::new(),
    }
}

fn project_name_from_path(path: &str) -> String {
    std::path::Path::new(path).file_name().and_then(|n| n.to_str()).unwrap_or("Unknown").to_string()
}

#[tauri::command]
pub async fn agent_list_sessions(pool: State<'_, SqlitePool>) -> Result<Vec<AgentSession>, String> {
    sqlx::query_as::<_, AgentSession>("SELECT * FROM agent_sessions ORDER BY last_used_at DESC")
        .fetch_all(pool.inner()).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn agent_create_session(
    pool: State<'_, SqlitePool>,
    title: String, purpose: String, project_path: String,
    skip_permissions: Option<bool>, custom_prompt: Option<String>,
    git_name: Option<String>, git_email: Option<String>,
) -> Result<AgentSession, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let project_name = project_name_from_path(&project_path);
    let context_prompt = custom_prompt.unwrap_or_else(|| get_purpose_prompt(&purpose));
    let skip = if skip_permissions.unwrap_or(false) { 1 } else { 0 };
    sqlx::query("INSERT INTO agent_sessions (id, title, purpose, project_path, project_name, context_prompt, skip_permissions, git_name, git_email, created_at, last_used_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(&id).bind(&title).bind(&purpose).bind(&project_path).bind(&project_name)
        .bind(&context_prompt).bind(skip).bind(&git_name).bind(&git_email).bind(&now).bind(&now)
        .execute(pool.inner()).await.map_err(|e| e.to_string())?;
    sqlx::query_as::<_, AgentSession>("SELECT * FROM agent_sessions WHERE id = ?")
        .bind(&id).fetch_one(pool.inner()).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn agent_update_session(
    pool: State<'_, SqlitePool>, id: String,
    title: Option<String>, skip_permissions: Option<bool>,
    git_name: Option<String>, git_email: Option<String>, context_prompt: Option<String>,
) -> Result<(), String> {
    if let Some(t) = title { sqlx::query("UPDATE agent_sessions SET title = ? WHERE id = ?").bind(&t).bind(&id).execute(pool.inner()).await.map_err(|e| e.to_string())?; }
    if let Some(sp) = skip_permissions { let val = if sp { 1 } else { 0 }; sqlx::query("UPDATE agent_sessions SET skip_permissions = ? WHERE id = ?").bind(val).bind(&id).execute(pool.inner()).await.map_err(|e| e.to_string())?; }
    if let Some(ref name) = git_name { sqlx::query("UPDATE agent_sessions SET git_name = ? WHERE id = ?").bind(name).bind(&id).execute(pool.inner()).await.map_err(|e| e.to_string())?; }
    if let Some(ref email) = git_email { sqlx::query("UPDATE agent_sessions SET git_email = ? WHERE id = ?").bind(email).bind(&id).execute(pool.inner()).await.map_err(|e| e.to_string())?; }
    if let Some(ref prompt) = context_prompt { sqlx::query("UPDATE agent_sessions SET context_prompt = ? WHERE id = ?").bind(prompt).bind(&id).execute(pool.inner()).await.map_err(|e| e.to_string())?; }
    Ok(())
}

#[tauri::command]
pub async fn agent_delete_session(pool: State<'_, SqlitePool>, id: String) -> Result<(), String> {
    sqlx::query("DELETE FROM agent_sessions WHERE id = ?").bind(&id).execute(pool.inner()).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn agent_update_session_id(pool: State<'_, SqlitePool>, id: String, claude_session_id: String) -> Result<(), String> {
    let val = if claude_session_id.is_empty() { None } else { Some(claude_session_id) };
    sqlx::query("UPDATE agent_sessions SET claude_session_id = ? WHERE id = ?").bind(&val).bind(&id).execute(pool.inner()).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn agent_update_last_used(pool: State<'_, SqlitePool>, id: String) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query("UPDATE agent_sessions SET last_used_at = ? WHERE id = ?").bind(&now).bind(&id).execute(pool.inner()).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn agent_update_worktree(pool: State<'_, SqlitePool>, id: String, worktree_path: Option<String>, worktree_branch: Option<String>) -> Result<(), String> {
    sqlx::query("UPDATE agent_sessions SET worktree_path = ?, worktree_branch = ? WHERE id = ?").bind(&worktree_path).bind(&worktree_branch).bind(&id).execute(pool.inner()).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn agent_list_contexts(pool: State<'_, SqlitePool>) -> Result<Vec<AgentContext>, String> {
    sqlx::query_as::<_, AgentContext>("SELECT * FROM agent_contexts ORDER BY name").fetch_all(pool.inner()).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn agent_save_context(pool: State<'_, SqlitePool>, id: Option<String>, name: String, content: String) -> Result<AgentContext, String> {
    let now = chrono::Utc::now().to_rfc3339();
    let ctx_id = id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    sqlx::query("INSERT INTO agent_contexts (id, name, content, created_at, updated_at) VALUES (?, ?, ?, ?, ?) ON CONFLICT(id) DO UPDATE SET name = excluded.name, content = excluded.content, updated_at = excluded.updated_at")
        .bind(&ctx_id).bind(&name).bind(&content).bind(&now).bind(&now).execute(pool.inner()).await.map_err(|e| e.to_string())?;
    sqlx::query_as::<_, AgentContext>("SELECT * FROM agent_contexts WHERE id = ?").bind(&ctx_id).fetch_one(pool.inner()).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn agent_delete_context(pool: State<'_, SqlitePool>, id: String) -> Result<(), String> {
    sqlx::query("DELETE FROM agent_contexts WHERE id = ?").bind(&id).execute(pool.inner()).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn agent_get_session_contexts(pool: State<'_, SqlitePool>, session_id: String) -> Result<Vec<AgentContext>, String> {
    sqlx::query_as::<_, AgentContext>("SELECT c.* FROM agent_contexts c INNER JOIN agent_session_contexts sc ON c.id = sc.context_id WHERE sc.session_id = ? ORDER BY c.name")
        .bind(&session_id).fetch_all(pool.inner()).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn agent_attach_context(pool: State<'_, SqlitePool>, session_id: String, context_id: String) -> Result<(), String> {
    sqlx::query("INSERT OR IGNORE INTO agent_session_contexts (session_id, context_id) VALUES (?, ?)").bind(&session_id).bind(&context_id).execute(pool.inner()).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn agent_detach_context(pool: State<'_, SqlitePool>, session_id: String, context_id: String) -> Result<(), String> {
    sqlx::query("DELETE FROM agent_session_contexts WHERE session_id = ? AND context_id = ?").bind(&session_id).bind(&context_id).execute(pool.inner()).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn agent_inject_contexts(pool: State<'_, SqlitePool>, project_path: String, context_ids: Vec<String>) -> Result<(), String> {
    if context_ids.is_empty() { return Ok(()); }

    // Fetch context content from DB by ID
    let mut contexts: Vec<(String, String)> = Vec::new();
    for id in &context_ids {
        let row: Option<(String, String)> = sqlx::query_as("SELECT name, content FROM agent_contexts WHERE id = ?")
            .bind(id).fetch_optional(pool.inner()).await.map_err(|e| e.to_string())?;
        if let Some(ctx) = row {
            contexts.push(ctx);
        }
    }
    if contexts.is_empty() { return Ok(()); }

    let claude_md_path = PathBuf::from(&project_path).join("CLAUDE.md");
    let marker_start = "<!-- CLAUGE-CONTEXT-START -->";
    let marker_end = "<!-- CLAUGE-CONTEXT-END -->";

    let existing_content = if claude_md_path.exists() {
        let raw = std::fs::read_to_string(&claude_md_path).map_err(|e| e.to_string())?;
        if let (Some(start), Some(_end)) = (raw.find(marker_start), raw.find(marker_end)) {
            raw[..start].trim_end().to_string()
        } else {
            raw
        }
    } else {
        String::new()
    };

    // Filter out snippets whose content already exists in the file
    let mut filtered = String::new();
    for (name, content) in &contexts {
        if !existing_content.contains(content.trim()) {
            if !filtered.is_empty() { filtered.push_str("\n\n---\n\n"); }
            filtered.push_str(&format!("## {}\n\n{}", name, content));
        }
    }
    if filtered.is_empty() { return Ok(()); }

    let injected = format!("\n\n{}\n{}\n{}\n", marker_start, filtered, marker_end);

    if !existing_content.is_empty() {
        std::fs::write(&claude_md_path, format!("{}{}", existing_content.trim_end(), injected)).map_err(|e| e.to_string())?;
    } else {
        std::fs::write(&claude_md_path, filtered).map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub fn agent_remove_injected_contexts(project_path: String) -> Result<(), String> {
    let claude_md_path = PathBuf::from(&project_path).join("CLAUDE.md");
    if !claude_md_path.exists() { return Ok(()); }

    let content = std::fs::read_to_string(&claude_md_path).map_err(|e| e.to_string())?;
    let marker_start = "<!-- CLAUGE-CONTEXT-START -->";
    let marker_end = "<!-- CLAUGE-CONTEXT-END -->";

    if let (Some(start), Some(end)) = (content.find(marker_start), content.find(marker_end)) {
        let cleaned = format!("{}{}", &content[..start].trim_end(), &content[end + marker_end.len()..]);
        if cleaned.trim().is_empty() {
            let _ = std::fs::remove_file(&claude_md_path);
        } else {
            std::fs::write(&claude_md_path, cleaned.trim_end().to_string() + "\n").map_err(|e| e.to_string())?;
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
