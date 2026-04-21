# Clauge Codebase Refactoring — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Decompose `lib.rs` (2,074 lines) and `+page.svelte` (2,896 lines) into focused domain modules and components with zero functional regressions.

**Architecture:** Rust backend splits into domain modules (models, storage, profiles, git, worktree, terminal, plugins, usage, sessions, contexts, system). Svelte frontend splits into reactive stores (`.svelte.ts`) and extracted components. Both orchestrators (`lib.rs`, `+page.svelte`) become thin wiring layers.

**Tech Stack:** Rust/Tauri v2, Svelte 5 with runes (`$state`, `$derived`), xterm.js, SvelteKit

**Spec:** `docs/superpowers/specs/2026-04-22-codebase-refactoring-design.md`

---

## Important Notes for Implementer

1. **This is a pure move-code refactoring.** Function bodies do NOT change. You are cutting code from one file and pasting into another with updated imports.
2. **Build after every task.** `cargo build` for Rust tasks, `bun run build` for Svelte tasks. If it doesn't compile, fix before moving on.
3. **The 4 existing components in `src/lib/components/` are dead code.** They are NOT imported by `+page.svelte`. You can safely delete or overwrite them.
4. **Read the source file before each task.** Line numbers reference the state at the START of the refactoring. After each task, line numbers shift — always search for function names, not line numbers.

---

## Phase 1: Rust Backend Refactoring

### Task 1: Create `models.rs` — Extract all shared structs

**Files:**
- Create: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Create `models.rs` with all struct definitions**

Create `src-tauri/src/models.rs` with the following content. Move these verbatim from `lib.rs` lines 19-117:

```rust
use portable_pty::{MasterPty, Child};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::sync::Arc;
use parking_lot::Mutex;

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionProfile {
    pub id: String,
    pub title: String,
    pub purpose: String,
    pub project_path: String,
    pub project_name: String,
    pub claude_session_id: Option<String>,
    pub context_prompt: String,
    pub created_at: String,
    pub last_used_at: String,
    #[serde(default)]
    pub worktree_path: Option<String>,
    #[serde(default)]
    pub worktree_branch: Option<String>,
    #[serde(default)]
    pub skip_permissions: bool,
    #[serde(default)]
    pub git_name: Option<String>,
    #[serde(default)]
    pub git_email: Option<String>,
    #[serde(default)]
    pub contexts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStore {
    pub profiles: Vec<SessionProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredSession {
    pub session_id: String,
    pub modified_at: String,
    pub preview: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_creation_tokens: u64,
    pub total_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudePlugin {
    pub name: String,
    pub marketplace: String,
    pub enabled: bool,
    pub version: Option<String>,
    pub install_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketplacePlugin {
    pub name: String,
    pub description: String,
    pub marketplace: String,
    pub category: Option<String>,
    pub installed: bool,
    pub installs: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalOutputPayload {
    pub terminal_id: String,
    pub data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitFileChange {
    pub path: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageAnalytics {
    pub total_cost: f64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cache_read_tokens: u64,
    pub total_cache_write_tokens: u64,
    pub total_sessions: u32,
    pub total_api_calls: u32,
    pub cache_hit_percent: f64,
    pub daily: Vec<DailyUsage>,
    pub by_model: Vec<ModelUsage>,
    pub by_project: Vec<ProjectUsage>,
    pub top_sessions: Vec<SessionCost>,
    pub tools: Vec<ToolCount>,
    pub shell_commands: Vec<ToolCount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyUsage {
    pub date: String,
    pub cost: f64,
    pub api_calls: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelUsage {
    pub model: String,
    pub cost: f64,
    pub api_calls: u32,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_hit_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectUsage {
    pub project: String,
    pub cost: f64,
    pub sessions: u32,
    pub api_calls: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionCost {
    pub session_id: String,
    pub project: String,
    pub cost: f64,
    pub api_calls: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCount {
    pub name: String,
    pub count: u32,
}

// ---------------------------------------------------------------------------
// Terminal state (non-Clone — holds PTY handles)
// ---------------------------------------------------------------------------

pub(crate) struct TerminalEntry {
    pub master: Box<dyn MasterPty + Send>,
    pub writer: Box<dyn Write + Send>,
    #[allow(dead_code)]
    pub child: Box<dyn Child + Send>,
}

pub struct TerminalState {
    pub terminals: Arc<Mutex<HashMap<String, TerminalEntry>>>,
}

impl Default for TerminalState {
    fn default() -> Self {
        Self {
            terminals: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}
```

- [ ] **Step 2: Update `lib.rs` — add module declaration, remove moved structs**

At the top of `lib.rs`, add:
```rust
pub mod models;
pub use models::*;
```

Delete the struct definitions from `lib.rs` (lines 15-117 — everything from `// Data model` through the `TerminalState` `Default` impl). Keep all `use` statements and all functions.

- [ ] **Step 3: Verify build**

Run: `cd /Users/macbook/Personal/CtxMgr/Clauge && cargo build --manifest-path src-tauri/Cargo.toml 2>&1 | tail -5`

Expected: Build succeeds. Fix any import errors (some `use` statements in `lib.rs` may now be unused — remove them).

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/models.rs src-tauri/src/lib.rs
git commit -m "refactor(rust): extract models.rs with all shared structs"
```

---

### Task 2: Create `storage.rs` — Extract persistence helpers

**Files:**
- Create: `src-tauri/src/storage.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Create `storage.rs`**

Move these functions from `lib.rs` (search by function name, not line number):
- `get_storage_dir()` (originally line 198)
- `get_storage_path()` (originally line 213)
- `load_profiles()` (originally line 217)
- `save_profiles()` (originally line 233)
- `encode_project_path()` (originally line 247)
- `now_iso8601()` (originally line 254)
- `project_name_from_path()` (originally line 258)

```rust
use crate::models::{SessionProfile, SessionStore};
use std::path::PathBuf;

pub fn get_storage_dir() -> PathBuf {
    // ... exact body from lib.rs — includes migration from ~/.ctx-mgr
}

pub fn get_storage_path() -> PathBuf {
    get_storage_dir().join("sessions.json")
}

pub fn load_profiles() -> Vec<SessionProfile> {
    // ... exact body from lib.rs
}

pub fn save_profiles(profiles: &[SessionProfile]) -> Result<(), String> {
    // ... exact body from lib.rs
}

pub fn encode_project_path(project_path: &str) -> String {
    // ... exact body from lib.rs
}

pub fn now_iso8601() -> String {
    // ... exact body from lib.rs
}

pub fn project_name_from_path(project_path: &str) -> String {
    // ... exact body from lib.rs
}
```

Add required imports at top of file. These functions use: `std::fs`, `std::path::PathBuf`, `dirs`, `serde_json`, `chrono::Utc`. Copy exact function bodies from `lib.rs`.

- [ ] **Step 2: Update `lib.rs`**

Add module declaration:
```rust
pub mod storage;
```

Delete the 7 moved functions from `lib.rs`. In remaining `lib.rs` code, replace any direct calls to these functions with `storage::` prefix, OR add `use storage::*;` at the top. The simpler approach is `use storage::*;` since many functions call `load_profiles()` etc.

- [ ] **Step 3: Verify build**

Run: `cd /Users/macbook/Personal/CtxMgr/Clauge && cargo build --manifest-path src-tauri/Cargo.toml 2>&1 | tail -5`

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/storage.rs src-tauri/src/lib.rs
git commit -m "refactor(rust): extract storage.rs with persistence helpers"
```

---

### Task 3: Create `profiles.rs` — Extract profile CRUD commands

**Files:**
- Create: `src-tauri/src/profiles.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Create `profiles.rs`**

Move these from `lib.rs`:
- `get_context_prompt()` (originally line 123) — make it `pub` since `terminal.rs` needs it later
- `get_profiles()` (originally line 271)
- `create_profile()` (originally line 276)
- `delete_profile()` (originally line 306)
- `rename_profile()` (originally line 314)
- `update_last_used()` (originally line 326)

```rust
use crate::models::SessionProfile;
use crate::storage::{load_profiles, save_profiles, now_iso8601, project_name_from_path};
use uuid::Uuid;

pub fn get_context_prompt(purpose: &str) -> String {
    // ... exact body from lib.rs
}

#[tauri::command]
pub fn get_profiles() -> Result<Vec<SessionProfile>, String> {
    Ok(load_profiles())
}

#[tauri::command]
pub fn create_profile(
    title: String,
    purpose: String,
    project_path: String,
    skip_permissions: Option<bool>,
    custom_prompt: Option<String>,
    git_name: Option<String>,
    git_email: Option<String>,
    contexts: Option<Vec<String>>,
) -> Result<SessionProfile, String> {
    // ... exact body from lib.rs — uses load_profiles, save_profiles, now_iso8601,
    //     project_name_from_path, get_context_prompt, Uuid::new_v4
}

#[tauri::command]
pub fn delete_profile(id: String) -> Result<(), String> {
    // ... exact body
}

#[tauri::command]
pub fn rename_profile(id: String, new_title: String) -> Result<(), String> {
    // ... exact body
}

#[tauri::command]
pub fn update_last_used(id: String) -> Result<(), String> {
    // ... exact body
}
```

- [ ] **Step 2: Update `lib.rs`**

Add: `pub mod profiles;`

Delete the 6 moved functions. Update `generate_handler![]` to prefix these commands:
```rust
profiles::get_profiles,
profiles::create_profile,
profiles::delete_profile,
profiles::rename_profile,
profiles::update_last_used,
```

- [ ] **Step 3: Verify build**

Run: `cd /Users/macbook/Personal/CtxMgr/Clauge && cargo build --manifest-path src-tauri/Cargo.toml 2>&1 | tail -5`

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/profiles.rs src-tauri/src/lib.rs
git commit -m "refactor(rust): extract profiles.rs with profile CRUD commands"
```

---

### Task 4: Create `git.rs` — Extract git operation commands

**Files:**
- Create: `src-tauri/src/git.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Create `git.rs`**

Move these 14 functions from `lib.rs`:
- `get_git_status()` (line 365)
- `get_git_branch()` (line 386)
- `get_git_ahead_behind()` (line 396)
- `git_commit()` (line 717)
- `git_push()` (line 743)
- `git_pull()` (line 764)
- `git_diff_file()` (line 777)
- `git_stage_file()` (line 805)
- `git_unstage_file()` (line 818)
- `git_log()` (line 831)
- `git_stash()` (line 856)
- `git_stash_pop()` (line 866)
- `git_list_branches()` (line 879)
- `git_switch_branch()` (line 900)

```rust
use crate::models::GitFileChange;
use std::process::Command;

#[tauri::command]
pub fn get_git_status(project_path: String) -> Result<Vec<GitFileChange>, String> {
    // ... exact body — uses std::process::Command, parses `git status --porcelain -u`
}

// ... all 13 other functions with exact bodies
// All use std::process::Command to run git CLI
// git_log uses serde_json::Value
// git_diff_file uses std::fs::read_to_string for untracked files
```

Required imports: `std::process::Command`, `serde_json`, `std::fs`. Check each function body for any imports needed.

- [ ] **Step 2: Update `lib.rs`**

Add: `pub mod git;`

Delete the 14 moved functions. Update `generate_handler![]`:
```rust
git::get_git_status,
git::get_git_branch,
git::get_git_ahead_behind,
git::git_commit,
git::git_push,
git::git_pull,
git::git_diff_file,
git::git_stage_file,
git::git_unstage_file,
git::git_log,
git::git_stash,
git::git_stash_pop,
git::git_list_branches,
git::git_switch_branch,
```

- [ ] **Step 3: Verify build**

Run: `cd /Users/macbook/Personal/CtxMgr/Clauge && cargo build --manifest-path src-tauri/Cargo.toml 2>&1 | tail -5`

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/git.rs src-tauri/src/lib.rs
git commit -m "refactor(rust): extract git.rs with 14 git operation commands"
```

---

### Task 5: Create `worktree.rs` — Extract worktree commands

**Files:**
- Create: `src-tauri/src/worktree.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Create `worktree.rs`**

Move these from `lib.rs`:
- `sanitize_branch_name()` (line 924) — internal helper
- `is_git_repo()` (line 913)
- `create_worktree()` (line 957)
- `remove_worktree()` (line 1009)
- `update_profile_worktree()` (line 1023)

```rust
use crate::models::SessionProfile;
use crate::storage::{load_profiles, save_profiles};
use std::process::Command;

fn sanitize_branch_name(name: &str) -> String {
    // ... exact body
}

#[tauri::command]
pub fn is_git_repo(path: String) -> Result<bool, String> {
    // ... exact body
}

#[tauri::command]
pub fn create_worktree(project_path: String, branch_name: String) -> Result<String, String> {
    // ... exact body — uses sanitize_branch_name, std::fs, std::process::Command
}

#[tauri::command]
pub fn remove_worktree(project_path: String, worktree_path: String, _branch_name: Option<String>) -> Result<(), String> {
    // ... exact body
}

#[tauri::command]
pub fn update_profile_worktree(id: String, worktree_path: Option<String>, worktree_branch: Option<String>) -> Result<(), String> {
    // ... exact body — uses load_profiles, save_profiles
}
```

- [ ] **Step 2: Update `lib.rs`**

Add: `pub mod worktree;`

Delete the 5 moved functions. Update `generate_handler![]`:
```rust
worktree::is_git_repo,
worktree::create_worktree,
worktree::remove_worktree,
worktree::update_profile_worktree,
```

- [ ] **Step 3: Verify build**

Run: `cd /Users/macbook/Personal/CtxMgr/Clauge && cargo build --manifest-path src-tauri/Cargo.toml 2>&1 | tail -5`

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/worktree.rs src-tauri/src/lib.rs
git commit -m "refactor(rust): extract worktree.rs with git worktree commands"
```

---

### Task 6: Create `terminal.rs` — Extract PTY commands

**Files:**
- Create: `src-tauri/src/terminal.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Create `terminal.rs`**

Move these from `lib.rs`:
- `resolve_claude_path()` (line 1280) — internal helper
- `spawn_terminal()` (line 1299)
- `spawn_shell()` (line 1433)
- `write_to_terminal()` (line 1485)
- `resize_terminal()` (line 1512)
- `kill_terminal()` (line 1538)

```rust
use crate::models::{TerminalEntry, TerminalOutputPayload, TerminalState};
use base64::Engine;
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::{Read, Write};
use tauri::ipc::Channel;
use tauri::State;
use uuid::Uuid;

fn resolve_claude_path() -> String {
    // ... exact body — runs `$SHELL -l -i -c 'which claude'`
}

#[tauri::command]
pub fn spawn_terminal(
    state: State<'_, TerminalState>,
    session_id: Option<String>,
    project_path: String,
    context_prompt: Option<String>,
    skip_permissions: Option<bool>,
    git_name: Option<String>,
    git_email: Option<String>,
    on_output: Channel<TerminalOutputPayload>,
) -> Result<String, String> {
    // ... exact body from lib.rs lines 1299-1429
}

#[tauri::command]
pub fn spawn_shell(
    state: State<'_, TerminalState>,
    project_path: String,
    on_output: Channel<TerminalOutputPayload>,
) -> Result<String, String> {
    // ... exact body from lib.rs lines 1433-1482
}

#[tauri::command]
pub fn write_to_terminal(
    state: State<'_, TerminalState>,
    terminal_id: String,
    data: String,
) -> Result<(), String> {
    // ... exact body
}

#[tauri::command]
pub fn resize_terminal(
    state: State<'_, TerminalState>,
    terminal_id: String,
    cols: u32,
    rows: u32,
) -> Result<(), String> {
    // ... exact body
}

#[tauri::command]
pub fn kill_terminal(
    state: State<'_, TerminalState>,
    terminal_id: String,
) -> Result<(), String> {
    // ... exact body
}
```

Note: `spawn_terminal` builds the claude command string inline (no call to `get_context_prompt` — the prompt is passed as a parameter from the frontend). If it DOES call `get_context_prompt`, add `use crate::profiles::get_context_prompt;`.

- [ ] **Step 2: Update `lib.rs`**

Add: `pub mod terminal;`

Delete the 6 moved functions. Update `generate_handler![]`:
```rust
terminal::spawn_terminal,
terminal::spawn_shell,
terminal::write_to_terminal,
terminal::resize_terminal,
terminal::kill_terminal,
```

- [ ] **Step 3: Verify build**

Run: `cd /Users/macbook/Personal/CtxMgr/Clauge && cargo build --manifest-path src-tauri/Cargo.toml 2>&1 | tail -5`

Note: The `run()` function's `.run()` closure accesses `TerminalState` for cleanup on exit. This stays in `lib.rs` since it uses `models::TerminalState` via `pub use models::*`.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/terminal.rs src-tauri/src/lib.rs
git commit -m "refactor(rust): extract terminal.rs with PTY management commands"
```

---

### Task 7: Create `plugins.rs` — Extract plugin commands

**Files:**
- Create: `src-tauri/src/plugins.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Create `plugins.rs`**

Move these from `lib.rs`:
- `get_claude_plugins()` (line 1552)
- `toggle_claude_plugin()` (line 1608)
- `get_marketplace_plugins()` (line 1631)
- `install_plugin()` (line 1723)
- `uninstall_plugin()` (line 1739)

```rust
use crate::models::{ClaudePlugin, MarketplacePlugin};
use std::process::Command;

#[tauri::command]
pub fn get_claude_plugins() -> Result<Vec<ClaudePlugin>, String> {
    // ... exact body — reads ~/.claude/settings.json and ~/.claude/plugins/installed_plugins.json
}

#[tauri::command]
pub fn toggle_claude_plugin(plugin_key: String, enabled: bool) -> Result<(), String> {
    // ... exact body — writes enabledPlugins to ~/.claude/settings.json
}

#[tauri::command]
pub fn get_marketplace_plugins() -> Result<Vec<MarketplacePlugin>, String> {
    // ... exact body — scans ~/.claude/plugins/marketplaces/*/marketplace.json
}

#[tauri::command]
pub fn install_plugin(name: String, marketplace: String) -> Result<(), String> {
    // ... exact body — runs `claude plugins add`
}

#[tauri::command]
pub fn uninstall_plugin(name: String, marketplace: String) -> Result<(), String> {
    // ... exact body — runs `claude plugins remove`
}
```

Required imports: `std::fs`, `std::path::PathBuf`, `serde_json`, `dirs`. Check function bodies.

- [ ] **Step 2: Update `lib.rs`**

Add: `pub mod plugins;`

Delete the 5 moved functions. Update `generate_handler![]`:
```rust
plugins::get_claude_plugins,
plugins::toggle_claude_plugin,
plugins::get_marketplace_plugins,
plugins::install_plugin,
plugins::uninstall_plugin,
```

- [ ] **Step 3: Verify build**

Run: `cd /Users/macbook/Personal/CtxMgr/Clauge && cargo build --manifest-path src-tauri/Cargo.toml 2>&1 | tail -5`

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/plugins.rs src-tauri/src/lib.rs
git commit -m "refactor(rust): extract plugins.rs with plugin management commands"
```

---

### Task 8: Create `usage.rs` — Extract usage analytics commands

**Files:**
- Create: `src-tauri/src/usage.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Create `usage.rs`**

Move these from `lib.rs`:
- `get_usage_analytics()` (line 478) — the async wrapper
- `get_usage_analytics_sync()` (line 484) — the big .jsonl parser (~230 lines)
- `fetch_usage_limits()` (line 1182)

```rust
use crate::models::*;
use crate::storage::encode_project_path;
use std::path::PathBuf;

#[tauri::command]
pub async fn get_usage_analytics(days: Option<u32>) -> Result<UsageAnalytics, String> {
    tokio::task::spawn_blocking(move || get_usage_analytics_sync(days))
        .await
        .map_err(|e| format!("Task join error: {}", e))?
}

fn get_usage_analytics_sync(days: Option<u32>) -> Result<UsageAnalytics, String> {
    // ... exact body (~230 lines) — parses .jsonl files, computes costs
    // Uses: dirs, std::fs, chrono, serde_json, encode_project_path
}

#[tauri::command]
pub fn fetch_usage_limits(session_key: String) -> Result<serde_json::Value, String> {
    // ... exact body — uses reqwest with native-tls
}
```

Required imports: `chrono`, `reqwest`, `serde_json`, `std::fs`, `std::collections::HashMap`, `dirs`.

- [ ] **Step 2: Update `lib.rs`**

Add: `pub mod usage;`

Delete the 3 moved functions. Update `generate_handler![]`:
```rust
usage::get_usage_analytics,
usage::fetch_usage_limits,
```

- [ ] **Step 3: Verify build**

Run: `cd /Users/macbook/Personal/CtxMgr/Clauge && cargo build --manifest-path src-tauri/Cargo.toml 2>&1 | tail -5`

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/usage.rs src-tauri/src/lib.rs
git commit -m "refactor(rust): extract usage.rs with analytics and limits commands"
```

---

### Task 9: Create `sessions.rs` — Extract session discovery commands

**Files:**
- Create: `src-tauri/src/sessions.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Create `sessions.rs`**

Move these from `lib.rs`:
- `refresh_session_ids()` (line 340)
- `update_session_id()` (line 346)
- `count_project_sessions()` (line 1037)
- `discover_sessions()` (line 1046)
- `get_session_tokens()` (line 1104)
- `update_session_contexts()` (line 1881)

```rust
use crate::models::{DiscoveredSession, SessionProfile, TokenUsage};
use crate::storage::{encode_project_path, load_profiles, save_profiles};

#[tauri::command]
pub fn refresh_session_ids() -> Result<Vec<SessionProfile>, String> {
    Ok(load_profiles())
}

#[tauri::command]
pub fn update_session_id(id: String, claude_session_id: String) -> Result<(), String> {
    // ... exact body — uses load_profiles, save_profiles
}

#[tauri::command]
pub fn count_project_sessions(project_path: String) -> Result<u32, String> {
    // ... exact body
}

#[tauri::command]
pub fn discover_sessions(project_path: String) -> Result<Vec<DiscoveredSession>, String> {
    // ... exact body — uses encode_project_path, dirs, std::fs, serde_json
}

#[tauri::command]
pub fn get_session_tokens(project_path: String, session_id: Option<String>) -> Result<TokenUsage, String> {
    // ... exact body
}

#[tauri::command]
pub fn update_session_contexts(id: String, contexts: Vec<String>) -> Result<(), String> {
    // ... exact body — uses load_profiles, save_profiles
}
```

- [ ] **Step 2: Update `lib.rs`**

Add: `pub mod sessions;`

Delete the 6 moved functions. Update `generate_handler![]`:
```rust
sessions::refresh_session_ids,
sessions::update_session_id,
sessions::count_project_sessions,
sessions::discover_sessions,
sessions::get_session_tokens,
sessions::update_session_contexts,
```

- [ ] **Step 3: Verify build**

Run: `cd /Users/macbook/Personal/CtxMgr/Clauge && cargo build --manifest-path src-tauri/Cargo.toml 2>&1 | tail -5`

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/sessions.rs src-tauri/src/lib.rs
git commit -m "refactor(rust): extract sessions.rs with session discovery commands"
```

---

### Task 10: Create `contexts.rs` — Extract context snippet commands

**Files:**
- Create: `src-tauri/src/contexts.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Create `contexts.rs`**

Move these from `lib.rs`:
- `get_contexts_dir()` (line 1754) — internal helper
- `get_context_snippets()` (line 1762)
- `save_context_snippet()` (line 1786)
- `delete_context_snippet()` (line 1793)
- `inject_session_context()` (line 1803)
- `remove_injected_context()` (line 1858)

```rust
use crate::storage::get_storage_dir;
use std::path::PathBuf;

fn get_contexts_dir() -> PathBuf {
    // ... exact body — returns ~/.clauge/contexts/
}

#[tauri::command]
pub fn get_context_snippets() -> Result<Vec<serde_json::Value>, String> {
    // ... exact body
}

#[tauri::command]
pub fn save_context_snippet(name: String, content: String) -> Result<(), String> {
    // ... exact body
}

#[tauri::command]
pub fn delete_context_snippet(name: String) -> Result<(), String> {
    // ... exact body
}

#[tauri::command]
pub fn inject_session_context(project_path: String, context_names: Vec<String>) -> Result<(), String> {
    // ... exact body — uses get_contexts_dir, encode_project_path, dirs, std::fs
}

#[tauri::command]
pub fn remove_injected_context(project_path: String) -> Result<(), String> {
    // ... exact body
}
```

Required imports: `std::fs`, `serde_json`, `dirs`, `crate::storage::encode_project_path`.

- [ ] **Step 2: Update `lib.rs`**

Add: `pub mod contexts;`

Delete the 6 moved functions. Update `generate_handler![]`:
```rust
contexts::get_context_snippets,
contexts::save_context_snippet,
contexts::delete_context_snippet,
contexts::inject_session_context,
contexts::remove_injected_context,
```

- [ ] **Step 3: Verify build**

Run: `cd /Users/macbook/Personal/CtxMgr/Clauge && cargo build --manifest-path src-tauri/Cargo.toml 2>&1 | tail -5`

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/contexts.rs src-tauri/src/lib.rs
git commit -m "refactor(rust): extract contexts.rs with context snippet commands"
```

---

### Task 11: Create `system.rs` — Extract system utility commands

**Files:**
- Create: `src-tauri/src/system.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Create `system.rs`**

Move these from `lib.rs`:
- `save_session_key()` (line 1228)
- `load_session_key()` (line 1235)
- `get_app_version()` (line 1248)
- `get_claude_plan()` (line 1254)
- `update_tray_title()` (line 1270)

```rust
use crate::storage::get_storage_dir;
use tauri::Manager;

#[tauri::command]
pub fn save_session_key(key: String) -> Result<(), String> {
    // ... exact body — writes to ~/.clauge/session_key
}

#[tauri::command]
pub fn load_session_key() -> Result<Option<String>, String> {
    // ... exact body — reads from ~/.clauge/session_key
}

#[tauri::command]
pub fn get_app_version() -> String {
    // ... exact body — reads from Cargo.toml
}

#[tauri::command]
pub fn get_claude_plan() -> Result<String, String> {
    // ... exact body — reads macOS keychain
}

#[tauri::command]
pub fn update_tray_title(app_handle: tauri::AppHandle, title: String) -> Result<(), String> {
    // ... exact body — uses TrayIconId, Manager
}
```

Required imports: `std::fs`, `std::process::Command`, `tauri::tray::TrayIconId`.

- [ ] **Step 2: Update `lib.rs`**

Add: `pub mod system;`

Delete the 5 moved functions. Update `generate_handler![]`:
```rust
system::save_session_key,
system::load_session_key,
system::get_app_version,
system::get_claude_plan,
system::update_tray_title,
```

- [ ] **Step 3: Verify build**

Run: `cd /Users/macbook/Personal/CtxMgr/Clauge && cargo build --manifest-path src-tauri/Cargo.toml 2>&1 | tail -5`

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/system.rs src-tauri/src/lib.rs
git commit -m "refactor(rust): extract system.rs with utility commands"
```

---

### Task 12: Clean up `lib.rs` — Final thin orchestrator

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Verify `lib.rs` is now just module declarations + `run()`**

After all extractions, `lib.rs` should contain ONLY:

```rust
mod models;
mod storage;
mod profiles;
mod git;
mod worktree;
mod terminal;
mod plugins;
mod usage;
mod sessions;
mod contexts;
mod system;

use models::TerminalState;

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::TrayIconBuilder;
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(TerminalState::default())
        .invoke_handler(tauri::generate_handler![
            profiles::get_profiles,
            profiles::create_profile,
            profiles::delete_profile,
            profiles::rename_profile,
            profiles::update_last_used,
            sessions::refresh_session_ids,
            sessions::update_session_id,
            sessions::count_project_sessions,
            sessions::discover_sessions,
            sessions::get_session_tokens,
            sessions::update_session_contexts,
            git::get_git_status,
            git::get_git_branch,
            git::get_git_ahead_behind,
            git::git_commit,
            git::git_push,
            git::git_pull,
            git::git_diff_file,
            git::git_stage_file,
            git::git_unstage_file,
            git::git_log,
            git::git_stash,
            git::git_stash_pop,
            git::git_list_branches,
            git::git_switch_branch,
            worktree::is_git_repo,
            worktree::create_worktree,
            worktree::remove_worktree,
            worktree::update_profile_worktree,
            terminal::spawn_terminal,
            terminal::spawn_shell,
            terminal::write_to_terminal,
            terminal::resize_terminal,
            terminal::kill_terminal,
            plugins::get_claude_plugins,
            plugins::toggle_claude_plugin,
            plugins::get_marketplace_plugins,
            plugins::install_plugin,
            plugins::uninstall_plugin,
            contexts::get_context_snippets,
            contexts::save_context_snippet,
            contexts::delete_context_snippet,
            contexts::inject_session_context,
            contexts::remove_injected_context,
            usage::get_usage_analytics,
            usage::fetch_usage_limits,
            system::save_session_key,
            system::load_session_key,
            system::get_app_version,
            system::get_claude_plan,
            system::update_tray_title,
        ])
        .setup(|app| {
            // ... entire setup closure stays here (vibrancy, menu, tray, autostart)
            // This is ~80 lines — it's app-level wiring, belongs in lib.rs
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            match event {
                tauri::RunEvent::Reopen { .. } => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                tauri::RunEvent::ExitRequested { .. } => {
                    if let Some(state) = app.try_state::<TerminalState>() {
                        let mut terminals = state.terminals.lock();
                        for (id, mut entry) in terminals.drain() {
                            let _ = entry.child.kill();
                            eprintln!("[Clauge] Cleaned up terminal {} on exit", id);
                        }
                    }
                }
                _ => {}
            }
        });
}
```

- [ ] **Step 2: Remove any dead `use` statements from `lib.rs`**

The only imports `lib.rs` needs are for the `run()` function: `tauri` menu/tray types, `Manager`, and `TerminalState`. Remove everything else (`base64`, `portable_pty`, `parking_lot`, `uuid`, `serde`, `std::io`, etc.).

- [ ] **Step 3: Verify clean build**

Run: `cd /Users/macbook/Personal/CtxMgr/Clauge && cargo build --manifest-path src-tauri/Cargo.toml 2>&1 | tail -10`

Expected: Build succeeds with no warnings about unused imports.

- [ ] **Step 4: Verify line count**

Run: `wc -l src-tauri/src/*.rs`

Expected: `lib.rs` should be ~120-150 lines. Total lines across all files should be ~2074 (same as before).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "refactor(rust): finalize lib.rs as thin orchestrator — backend refactoring complete"
```

---

## Phase 2: Svelte Frontend — Extract Stores

### Task 13: Create `theme.svelte.ts` — Extract theme state and logic

**Files:**
- Create: `src/lib/stores/theme.svelte.ts`
- Modify: `src/routes/+page.svelte`

- [ ] **Step 1: Create the stores directory**

Run: `mkdir -p /Users/macbook/Personal/CtxMgr/Clauge/src/lib/stores`

- [ ] **Step 2: Create `theme.svelte.ts`**

```typescript
// src/lib/stores/theme.svelte.ts

let _currentTheme = $state(
  typeof localStorage !== 'undefined'
    ? (localStorage.getItem('clauge-theme') || 'dark')
    : 'dark'
);
let _accentColor = $state(
  typeof localStorage !== 'undefined'
    ? (localStorage.getItem('clauge-accent') || '#58a6ff')
    : '#58a6ff'
);

export function getCurrentTheme() { return _currentTheme; }
export function getAccentColor() { return _accentColor; }

export function applyTheme(themeName: string) {
  _currentTheme = themeName;
  localStorage.setItem('clauge-theme', themeName);
  // ... exact body of applyTheme from +page.svelte (line 182)
  // Sets CSS custom properties on document.documentElement
  // Updates all xterm terminal instances' theme
}

export function applyAccent(color: string) {
  _accentColor = color;
  localStorage.setItem('clauge-accent', color);
  // ... exact body of applyAccent from +page.svelte (line 203)
  // Sets --accent CSS variable + updates terminal cursors
}
```

**Note on Svelte 5 module-level `$state`:** In `.svelte.ts` files, `$state` works at module level. However, you cannot `export let x = $state(...)` directly — the rune creates a local binding. Use getter functions or export an object. Check if Svelte 5 allows direct export of `$state` in `.svelte.ts` files during the build step. If not, wrap state in a class or use getter/setter pattern:

```typescript
class ThemeStore {
  currentTheme = $state('dark');
  accentColor = $state('#58a6ff');

  constructor() {
    if (typeof localStorage !== 'undefined') {
      this.currentTheme = localStorage.getItem('clauge-theme') || 'dark';
      this.accentColor = localStorage.getItem('clauge-accent') || '#58a6ff';
    }
  }

  applyTheme(themeName: string) {
    this.currentTheme = themeName;
    // ... rest of body
  }

  applyAccent(color: string) {
    this.accentColor = color;
    // ... rest of body
  }
}

export const theme = new ThemeStore();
```

Use whichever pattern compiles. The class pattern is the safest for Svelte 5 `.svelte.ts` exports.

- [ ] **Step 3: Update `+page.svelte`**

Replace:
```javascript
let currentTheme = $state(typeof localStorage !== 'undefined' ? (localStorage.getItem('clauge-theme') || 'dark') : 'dark');
let accentColor = $state(typeof localStorage !== 'undefined' ? (localStorage.getItem('clauge-accent') || '#58a6ff') : '#58a6ff');
```

And the `applyTheme()` / `applyAccent()` function definitions with:

```javascript
import { theme } from "$lib/stores/theme.svelte";
```

Then replace all references in template/script:
- `currentTheme` → `theme.currentTheme`
- `accentColor` → `theme.accentColor`
- `applyTheme(...)` → `theme.applyTheme(...)`
- `applyAccent(...)` → `theme.applyAccent(...)`

- [ ] **Step 4: Verify build**

Run: `cd /Users/macbook/Personal/CtxMgr/Clauge && bun run build 2>&1 | tail -5`

- [ ] **Step 5: Commit**

```bash
git add src/lib/stores/theme.svelte.ts src/routes/+page.svelte
git commit -m "refactor(svelte): extract theme store"
```

---

### Task 14: Create `notifications.svelte.ts` — Extract notification logic

**Files:**
- Create: `src/lib/stores/notifications.svelte.ts`
- Modify: `src/routes/+page.svelte`

- [ ] **Step 1: Create `notifications.svelte.ts`**

Move from `+page.svelte`:
- `checkForActionPrompt()` (line 1187) — includes inner `checkBuffer()` function
- `playNotificationSound()` (line 1240)
- `sendActionNotification()` (line 1267)

```typescript
// src/lib/stores/notifications.svelte.ts
import { invoke } from "@tauri-apps/api/core";

// Internal state (not exported — notification module manages its own buffers)
const outputBuffers: Record<string, string> = {};
let lastNotifyTime = 0;
let soundInterval: ReturnType<typeof setInterval> | null = null;

export function checkForActionPrompt(base64Data: string, sessionTitle: string) {
  // ... exact body from +page.svelte line 1187
  // Includes inner checkBuffer() function
}

export function playNotificationSound() {
  // ... exact body — Web Audio chime (A5 + E6 sine tones)
}

export async function sendActionNotification(sessionTitle: string) {
  // ... exact body — uses tauri-plugin-notification
}
```

Note: `checkForActionPrompt` references `document.hasFocus()` — this is fine in a `.svelte.ts` file since it only runs in the browser. `sendActionNotification` imports from `@tauri-apps/plugin-notification` — add that import. Also needs `import("@tauri-apps/api/webviewWindow")` for `requestUserAttention`.

- [ ] **Step 2: Update `+page.svelte`**

Add import:
```javascript
import { checkForActionPrompt, playNotificationSound, sendActionNotification } from "$lib/stores/notifications.svelte";
```

Delete the 3 function definitions and any module-level notification state (buffers, intervals) from `+page.svelte`. The functions are called from the terminal output handler — update those call sites to use the imported functions (names are the same, no signature change needed).

- [ ] **Step 3: Verify build**

Run: `cd /Users/macbook/Personal/CtxMgr/Clauge && bun run build 2>&1 | tail -5`

- [ ] **Step 4: Commit**

```bash
git add src/lib/stores/notifications.svelte.ts src/routes/+page.svelte
git commit -m "refactor(svelte): extract notifications store"
```

---

### Task 15: Create `updater.svelte.ts` — Extract auto-update logic

**Files:**
- Create: `src/lib/stores/updater.svelte.ts`
- Modify: `src/routes/+page.svelte`

- [ ] **Step 1: Create `updater.svelte.ts`**

Move from `+page.svelte`:
- `updateReady` state (line 37)
- `updateDismissed` state (line 38)
- `showUpdateModal` state (line 39)
- `showWhatsNew` state (line 40)
- `whatsNewBody` state (line 41)
- `checkAndDownloadUpdate()` (line 1006)
- `restartToUpdate()` (line 1021)
- `checkWhatsNew()` (line 1043)

```typescript
// src/lib/stores/updater.svelte.ts

class UpdaterStore {
  updateReady = $state<any>(null);
  updateDismissed = $state(false);
  showUpdateModal = $state(false);
  showWhatsNew = $state(false);
  whatsNewBody = $state('');
  pendingUpdate: any = null;

  async checkAndDownloadUpdate() {
    // ... exact body — uses @tauri-apps/plugin-updater check()
  }

  async restartToUpdate() {
    // ... exact body — uses @tauri-apps/plugin-process relaunch()
  }

  checkWhatsNew(version: string) {
    // ... exact body — compares localStorage 'clauge-last-seen-version', fetches from GitHub API
  }
}

export const updater = new UpdaterStore();
```

Imports needed: `@tauri-apps/plugin-updater` (check), `@tauri-apps/plugin-process` (relaunch).

- [ ] **Step 2: Update `+page.svelte`**

Add import, replace state variables and functions with `updater.*` references. Search template for `updateReady`, `updateDismissed`, `showWhatsNew`, `whatsNewBody`, `showUpdateModal` and prefix with `updater.`.

- [ ] **Step 3: Verify build**

Run: `cd /Users/macbook/Personal/CtxMgr/Clauge && bun run build 2>&1 | tail -5`

- [ ] **Step 4: Commit**

```bash
git add src/lib/stores/updater.svelte.ts src/routes/+page.svelte
git commit -m "refactor(svelte): extract updater store"
```

---

### Task 16: Create `plugins.svelte.ts` — Extract plugin management state

**Files:**
- Create: `src/lib/stores/plugins.svelte.ts`
- Modify: `src/routes/+page.svelte`

- [ ] **Step 1: Create `plugins.svelte.ts`**

Move from `+page.svelte`:
- State: `claudePlugins` (line 13), `marketplacePlugins` (line 14), `pluginSearch` (line 15), `installingPlugin` (line 16), `pluginTab` (line 17)
- Functions: `loadClaudePlugins()` (line 1309), `togglePlugin()` (line 1313+), `installPlugin()`, `uninstallPlugin()`

```typescript
// src/lib/stores/plugins.svelte.ts
import { invoke } from "@tauri-apps/api/core";

class PluginsStore {
  claudePlugins = $state<any[]>([]);
  marketplacePlugins = $state<any[]>([]);
  pluginSearch = $state('');
  installingPlugin = $state('');
  pluginTab = $state('installed');

  async loadClaudePlugins() {
    // ... exact body
  }

  async togglePlugin(plugin: any) {
    // ... exact body
  }

  async installPlugin(plugin: any) {
    // ... exact body
  }

  async uninstallPlugin(plugin: any) {
    // ... exact body
  }
}

export const pluginsStore = new PluginsStore();
```

- [ ] **Step 2: Update `+page.svelte`**

Import and replace all references.

- [ ] **Step 3: Verify build**

Run: `cd /Users/macbook/Personal/CtxMgr/Clauge && bun run build 2>&1 | tail -5`

- [ ] **Step 4: Commit**

```bash
git add src/lib/stores/plugins.svelte.ts src/routes/+page.svelte
git commit -m "refactor(svelte): extract plugins store"
```

---

### Task 17: Create `contexts.svelte.ts` — Extract context snippet state

**Files:**
- Create: `src/lib/stores/contexts.svelte.ts`
- Modify: `src/routes/+page.svelte`

- [ ] **Step 1: Create `contexts.svelte.ts`**

Move from `+page.svelte`:
- State: `contextSnippets` (line 20), `contextEditing` (line 21), `contextNewName` (line 22), `contextNewContent` (line 23), `modalContexts` (line 24), `showContextPicker` (line 25), `modalContextEnabled` (line 26), `showContextDropdown` (line 27)
- Functions: `loadContextSnippets()` (line 1281), `saveContextSnippet()` (line 1284), `deleteContextSnippet()` (line 1293), `attachContextsToSession()` (line 1296), `detachContextsFromSession()` (line 1302)

```typescript
// src/lib/stores/contexts.svelte.ts
import { invoke } from "@tauri-apps/api/core";

class ContextsStore {
  contextSnippets = $state<any[]>([]);
  contextEditing = $state<any>(null);
  contextNewName = $state('');
  contextNewContent = $state('');
  modalContexts = $state<string[]>([]);
  showContextPicker = $state(false);
  modalContextEnabled = $state(false);
  showContextDropdown = $state(false);

  async loadContextSnippets() {
    // ... exact body
  }

  async saveContextSnippet() {
    // ... exact body
  }

  async deleteContextSnippet(name: string) {
    // ... exact body
  }

  async attachContextsToSession(profileId: string, projectPath: string, contextNames: string[]) {
    // ... exact body
  }

  async detachContextsFromSession(profileId: string, projectPath: string) {
    // ... exact body
  }
}

export const contextsStore = new ContextsStore();
```

- [ ] **Step 2: Update `+page.svelte`**

Import and replace.

- [ ] **Step 3: Verify build**

Run: `cd /Users/macbook/Personal/CtxMgr/Clauge && bun run build 2>&1 | tail -5`

- [ ] **Step 4: Commit**

```bash
git add src/lib/stores/contexts.svelte.ts src/routes/+page.svelte
git commit -m "refactor(svelte): extract contexts store"
```

---

### Task 18: Create `usage.svelte.ts` — Extract usage tracking state

**Files:**
- Create: `src/lib/stores/usage.svelte.ts`
- Modify: `src/routes/+page.svelte`

- [ ] **Step 1: Create `usage.svelte.ts`**

Move from `+page.svelte`:
- State: `usageLimits` (line 33), `sessionKeyInput` (line 34), `sessionKeyConfigured` (line 42), `usageError` (line 43), `showKeyEdit` (line 44), `usageRefreshMins` (line 102), `showDashboard` (line 98), `dashboardData` (line 99), `dashboardLoading` (line 100), `dashboardDays` (line 101)
- Functions: `loadUsageLimits()`, `loadDashboard()` (line 925), `formatCost()` (line 933), `formatTokens()` (line 934), `decodeProjectName()` (line 935)

```typescript
// src/lib/stores/usage.svelte.ts
import { invoke } from "@tauri-apps/api/core";

class UsageStore {
  usageLimits = $state<any>(null);
  sessionKeyInput = $state('');
  sessionKeyConfigured = $state(false);
  usageError = $state('');
  showKeyEdit = $state(false);
  usageRefreshMins = $state(
    typeof localStorage !== 'undefined'
      ? parseInt(localStorage.getItem('clauge-usage-refresh') || '5')
      : 5
  );
  showDashboard = $state(false);
  dashboardData = $state<any>(null);
  dashboardLoading = $state(false);
  dashboardDays = $state(30);

  async loadUsageLimits() {
    // ... exact body — invokes fetch_usage_limits, handles auth failure
  }

  async loadDashboard() {
    // ... exact body — invokes get_usage_analytics
  }

  formatCost(v: number): string {
    return v < 0.01 ? '<$0.01' : `$${v.toFixed(2)}`;
  }

  formatTokens(v: number): string {
    return v >= 1000000 ? `${(v/1000000).toFixed(1)}M` : v >= 1000 ? `${(v/1000).toFixed(1)}k` : String(v);
  }

  decodeProjectName(encoded: string): string {
    // ... exact body
  }
}

export const usageStore = new UsageStore();
```

- [ ] **Step 2: Update `+page.svelte`**

Import and replace. The `onMount` usage interval setup references `loadUsageLimits` — update to `usageStore.loadUsageLimits()`.

- [ ] **Step 3: Verify build**

Run: `cd /Users/macbook/Personal/CtxMgr/Clauge && bun run build 2>&1 | tail -5`

- [ ] **Step 4: Commit**

```bash
git add src/lib/stores/usage.svelte.ts src/routes/+page.svelte
git commit -m "refactor(svelte): extract usage store"
```

---

### Task 19: Create `git.svelte.ts` — Extract git state and operations

**Files:**
- Create: `src/lib/stores/git.svelte.ts`
- Modify: `src/routes/+page.svelte`

- [ ] **Step 1: Create `git.svelte.ts`**

Move from `+page.svelte`:
- State: `gitBranch` (line 87), `gitFiles` (line 88), `gitChanges` (line 86), `gitPanelOpen` (line 89), `gitTab` (line 90), `gitCommitMsg` (line 91), `gitLoading` (line 92), `gitAhead` (line 93), `gitBehind` (line 94), `gitMsg` (line 95), `gitDiff` (line 103), `gitDiffFile` (line 104), `gitCommits` (line 105), `gitBranches` (line 106), `stagedFiles` (line 107)
- Functions: `refreshGitStatus()` (line 746), `showGitMsg()` (line 764), `doGitCommit()` (line 769), `doGitCommitStaged()` (line 830), `doGitPush()` (line 782), `doGitPull()` (line 794), `viewDiff()` (line 806), `toggleStageFile()` (line 813), `loadGitHistory()` (line 850), `loadGitBranches()` (line 856), `switchBranch()` (line 862), `doGitStash()` (line 880), `doGitStashPop()` (line 890)

```typescript
// src/lib/stores/git.svelte.ts
import { invoke } from "@tauri-apps/api/core";

class GitStore {
  gitBranch = $state('');
  gitFiles = $state<any[]>([]);
  gitChanges = $state<Record<string, number>>({});
  gitPanelOpen = $state(false);
  gitTab = $state('changes');
  gitCommitMsg = $state('');
  gitLoading = $state('');
  gitAhead = $state(0);
  gitBehind = $state(0);
  gitMsg = $state('');
  gitDiff = $state('');
  gitDiffFile = $state('');
  gitCommits = $state<any[]>([]);
  gitBranches = $state<any[]>([]);
  stagedFiles = $state(new Set<string>());

  showGitMsg(msg: string, duration = 3000) {
    this.gitMsg = msg;
    setTimeout(() => { this.gitMsg = ''; }, duration);
  }

  async refreshGitStatus(projectPath: string) {
    // ... exact body — invokes get_git_status, get_git_branch, get_git_ahead_behind
    // Note: original references `activeProfile` — pass projectPath as parameter instead
  }

  async doGitCommit(projectPath: string) {
    // ... exact body
  }

  async doGitCommitStaged(projectPath: string) {
    // ... exact body
  }

  async doGitPush(projectPath: string) {
    // ... exact body
  }

  async doGitPull(projectPath: string) {
    // ... exact body
  }

  async viewDiff(projectPath: string, file: any) {
    // ... exact body
  }

  async toggleStageFile(projectPath: string, file: any) {
    // ... exact body
  }

  async loadGitHistory(projectPath: string) {
    // ... exact body
  }

  async loadGitBranches(projectPath: string) {
    // ... exact body
  }

  async switchBranch(projectPath: string, branchName: string) {
    // ... exact body
  }

  async doGitStash(projectPath: string) {
    // ... exact body
  }

  async doGitStashPop(projectPath: string) {
    // ... exact body
  }
}

export const gitStore = new GitStore();
```

**Important:** Many git functions reference `activeProfile?.worktreePath || activeProfile?.projectPath` to determine the working directory. In the store, accept `projectPath` as a parameter. The caller in `+page.svelte` passes the resolved path.

- [ ] **Step 2: Update `+page.svelte`**

Import `gitStore`. Replace all git state references with `gitStore.*`. Update function calls to pass the project path:
```javascript
// Before:
refreshGitStatus();
// After:
gitStore.refreshGitStatus(activeProfile?.worktreePath || activeProfile?.projectPath);
```

Update the 5-second polling interval in `onMount` similarly. Update all template bindings (e.g., `bind:value={gitCommitMsg}` → `bind:value={gitStore.gitCommitMsg}`).

- [ ] **Step 3: Verify build**

Run: `cd /Users/macbook/Personal/CtxMgr/Clauge && bun run build 2>&1 | tail -5`

- [ ] **Step 4: Commit**

```bash
git add src/lib/stores/git.svelte.ts src/routes/+page.svelte
git commit -m "refactor(svelte): extract git store with all git operations"
```

---

### Task 20: Create remaining stores — `terminal.svelte.ts` and `shell.svelte.ts`

**Files:**
- Create: `src/lib/stores/terminal.svelte.ts`
- Create: `src/lib/stores/shell.svelte.ts`
- Modify: `src/routes/+page.svelte`

These are the most complex stores because they manage DOM elements (xterm instances) and Tauri Channel streams. They're tightly coupled to each other and to `+page.svelte`'s DOM.

- [ ] **Step 1: Assess DOM coupling**

Read the terminal management functions in `+page.svelte`: `createTermEntry()`, `showTermEntry()`, `createShellEntry()`, `showShellEntry()`, `spawnShellForProfile()`, `toggleShell()`, `selectProfile()`. Note which ones:
- Create DOM elements (`document.createElement`)
- Reference bind:this elements (`terminalEl`, `shellEl`, `wrapperEl`)
- Call xterm methods (`term.open()`, `fitAddon.fit()`)
- Use `invoke` with Tauri Channel

**Decision point:** If terminal functions heavily manipulate DOM refs owned by `+page.svelte`, they may be better left in `+page.svelte` for now, or extracted as components (Phase 3) rather than stores. The key question: can the logic be separated from the DOM?

- [ ] **Step 2: Create `terminal.svelte.ts` with state and non-DOM logic**

Extract the state and functions that don't directly touch DOM:

```typescript
// src/lib/stores/terminal.svelte.ts
import { invoke, Channel } from "@tauri-apps/api/core";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";

class TerminalStore {
  terminalMap = $state(new Map());
  activeTermEntry = $state<any>(null);
  currentTerminalId = $state<string | null>(null);
  termFontSize = $state(
    typeof localStorage !== 'undefined'
      ? parseInt(localStorage.getItem('clauge-font-size') || '13')
      : 13
  );
  sessionActivity = $state<Record<string, string>>({});

  getTermConfig() {
    // ... exact body — returns xterm Terminal options object
  }

  async loadWebGLAddon(term: Terminal) {
    // ... exact body — dynamic import of @xterm/addon-webgl
  }

  // createTermEntry, showTermEntry stay in +page.svelte or move to a component
  // because they manipulate DOM refs (terminalEl.appendChild, etc.)
}

export const terminalStore = new TerminalStore();
```

- [ ] **Step 3: Create `shell.svelte.ts` with state**

```typescript
// src/lib/stores/shell.svelte.ts

class ShellStore {
  shellMap = $state(new Map());
  shellOpenMap = $state<Record<string, boolean>>({});
  activeShellEntry = $state<any>(null);
  shellWidthMap = $state<Record<string, number>>({});
  isDraggingDivider = $state(false);
  focusedPanel = $state('claude');

  getShellWidth(profileId: string): number {
    return this.shellWidthMap[profileId] ?? 50;
  }
}

export const shellStore = new ShellStore();
```

- [ ] **Step 4: Update `+page.svelte`**

Import both stores. Move state references to store properties. Keep DOM-manipulation functions (`createTermEntry`, `showTermEntry`, `createShellEntry`, `showShellEntry`, `spawnShellForProfile`, `toggleShell`, `selectProfile`, `startDividerDrag`) in `+page.svelte` for now — they'll be extracted into components in Phase 3.

- [ ] **Step 5: Verify build**

Run: `cd /Users/macbook/Personal/CtxMgr/Clauge && bun run build 2>&1 | tail -5`

- [ ] **Step 6: Commit**

```bash
git add src/lib/stores/terminal.svelte.ts src/lib/stores/shell.svelte.ts src/routes/+page.svelte
git commit -m "refactor(svelte): extract terminal and shell stores"
```

---

## Phase 3: Svelte Frontend — Extract Components

### Task 21: Create `BottomBar.svelte`

**Files:**
- Create: `src/lib/components/BottomBar.svelte`
- Modify: `src/routes/+page.svelte`

- [ ] **Step 1: Create `BottomBar.svelte`**

Extract the `<div class="bottom-bar">` section from `+page.svelte` (lines 1537-1729) into a new component.

```svelte
<script>
  import { invoke } from "@tauri-apps/api/core";
  import { gitStore } from "$lib/stores/git.svelte";
  import { usageStore } from "$lib/stores/usage.svelte";
  import { shellStore } from "$lib/stores/shell.svelte";

  let {
    activeProfile,
    appVersion,
    claudePlan,
    profileMenuOpen = $bindable(),
    showSettings = $bindable(),
    showDashboard = $bindable(),
    onToggleShell,
    onOpenExternal,
  } = $props();
</script>

<div class="bottom-bar">
  <div class="bottom-left">
    <!-- profile avatar + menu -->
    <!-- git status bar (branch, ahead/behind, changes) — onclick opens gitStore.gitPanelOpen -->
  </div>
  <div class="bottom-center">
    <!-- usage chips — onclick opens showDashboard -->
  </div>
  <div class="bottom-right">
    <!-- shell toggle button, version -->
  </div>
</div>

<style>
  /* Move all .bottom-bar, .bottom-left, .bottom-center, .bottom-right styles here */
</style>
```

Move the corresponding `<style>` rules from `+page.svelte` line 2445+ that match `.bottom-bar*` selectors.

- [ ] **Step 2: Use in `+page.svelte`**

```svelte
<script>
  import BottomBar from "$lib/components/BottomBar.svelte";
</script>

<!-- Replace the entire <div class="bottom-bar">...</div> with: -->
<BottomBar
  {activeProfile}
  {appVersion}
  {claudePlan}
  bind:profileMenuOpen
  bind:showSettings
  bind:showDashboard
  onToggleShell={toggleShell}
  onOpenExternal={openExternal}
/>
```

- [ ] **Step 3: Verify build**

Run: `cd /Users/macbook/Personal/CtxMgr/Clauge && bun run build 2>&1 | tail -5`

- [ ] **Step 4: Commit**

```bash
git add src/lib/components/BottomBar.svelte src/routes/+page.svelte
git commit -m "refactor(svelte): extract BottomBar component"
```

---

### Task 22: Create `GitPanel.svelte`

**Files:**
- Create: `src/lib/components/GitPanel.svelte`
- Modify: `src/routes/+page.svelte`

- [ ] **Step 1: Create `GitPanel.svelte`**

Extract the git popup markup from `+page.svelte`. Search for the git panel `{#if gitPanelOpen}` block (around lines 1580-1700 area — find the exact block by searching for `gitPanelOpen`).

```svelte
<script>
  import { gitStore } from "$lib/stores/git.svelte";

  let { projectPath } = $props();
</script>

{#if gitStore.gitPanelOpen}
<div class="git-popup">
  <!-- Tab bar: changes / history / branches -->
  <!-- Changes tab: file list, staging, diff viewer, commit -->
  <!-- History tab: commit list -->
  <!-- Branches tab: branch list -->
  <!-- Action row: stash, pop, pull, push -->
</div>
{/if}

<style>
  /* Move all .git-popup, .git-file-list, .git-commit-row, .git-diff-*, .git-history-* styles */
</style>
```

All git operations call `gitStore.doGitCommit(projectPath)` etc. — the `projectPath` prop is the resolved worktree/project path.

- [ ] **Step 2: Use in `+page.svelte`**

```svelte
<script>
  import GitPanel from "$lib/components/GitPanel.svelte";
</script>

<!-- Replace git popup block with: -->
<GitPanel projectPath={activeProfile?.worktreePath || activeProfile?.projectPath} />
```

- [ ] **Step 3: Verify build**

Run: `cd /Users/macbook/Personal/CtxMgr/Clauge && bun run build 2>&1 | tail -5`

- [ ] **Step 4: Commit**

```bash
git add src/lib/components/GitPanel.svelte src/routes/+page.svelte
git commit -m "refactor(svelte): extract GitPanel component"
```

---

### Task 23: Create `SettingsModal.svelte`

**Files:**
- Create: `src/lib/components/SettingsModal.svelte`
- Modify: `src/routes/+page.svelte`

- [ ] **Step 1: Create `SettingsModal.svelte`**

Extract the `{#if showSettings}` block from `+page.svelte` (line 1885).

```svelte
<script>
  import { theme } from "$lib/stores/theme.svelte";
  import { pluginsStore } from "$lib/stores/plugins.svelte";
  import { terminalStore } from "$lib/stores/terminal.svelte";

  let {
    show = $bindable(),
    settingsTab = $bindable(),
    appVersion,
    onOpenExternal,
  } = $props();
</script>

{#if show}
<div class="modal-overlay" onclick={() => show = false}>
  <div class="settings-modal" onclick|stopPropagation>
    <!-- Sidebar tabs: Appearance / Plugins / About -->
    <!-- Appearance: theme toggle, accent presets, font slider -->
    <!-- Plugins: installed/marketplace (uses pluginsStore) -->
    <!-- About: version, links -->
  </div>
</div>
{/if}

<style>
  /* Move .settings-modal, .settings-sidebar, .settings-content styles */
</style>
```

- [ ] **Step 2: Use in `+page.svelte`**

```svelte
<SettingsModal
  bind:show={showSettings}
  bind:settingsTab
  {appVersion}
  onOpenExternal={openExternal}
/>
```

- [ ] **Step 3: Verify build and commit**

```bash
git add src/lib/components/SettingsModal.svelte src/routes/+page.svelte
git commit -m "refactor(svelte): extract SettingsModal component"
```

---

### Task 24: Create `UsageDashboard.svelte`

**Files:**
- Create: `src/lib/components/UsageDashboard.svelte`
- Modify: `src/routes/+page.svelte`

- [ ] **Step 1: Create `UsageDashboard.svelte`**

Extract the `{#if showDashboard}` block (line 2171).

```svelte
<script>
  import { usageStore } from "$lib/stores/usage.svelte";

  let { show = $bindable() } = $props();
</script>

{#if show}
<div class="modal-overlay" onclick={() => show = false}>
  <div class="dashboard-modal" onclick|stopPropagation>
    <!-- Summary cards, daily chart, model/project breakdowns -->
    <!-- Top sessions, tools, shell commands -->
    <!-- Usage bars, session key config -->
  </div>
</div>
{/if}

<style>
  /* Move .dashboard-modal, .dashboard-grid, .dashboard-card styles */
</style>
```

- [ ] **Step 2: Use in `+page.svelte`**

```svelte
<UsageDashboard bind:show={usageStore.showDashboard} />
```

- [ ] **Step 3: Verify build and commit**

```bash
git add src/lib/components/UsageDashboard.svelte src/routes/+page.svelte
git commit -m "refactor(svelte): extract UsageDashboard component"
```

---

### Task 25: Create remaining modal components

**Files:**
- Create: `src/lib/components/DeleteConfirmModal.svelte`
- Create: `src/lib/components/UpdateToast.svelte`
- Modify: `src/routes/+page.svelte`

- [ ] **Step 1: Create `DeleteConfirmModal.svelte`**

Extract the `{#if deleteConfirm}` block (line 2409).

```svelte
<script>
  let {
    profile,
    onConfirm,
    onCancel,
  } = $props();
</script>

{#if profile}
<div class="modal-overlay" onclick={onCancel}>
  <div class="confirm-modal" onclick|stopPropagation>
    <!-- "Delete {profile.title}?" with cancel/delete buttons -->
  </div>
</div>
{/if}

<style>
  /* Move .confirm-modal styles */
</style>
```

- [ ] **Step 2: Create `UpdateToast.svelte`**

Extract the `{#if updateReady && !updateDismissed}` block (line 1731) and `{#if showWhatsNew}` block (line 2130).

```svelte
<script>
  import { updater } from "$lib/stores/updater.svelte";
</script>

{#if updater.updateReady && !updater.updateDismissed}
<div class="update-toast">
  <!-- "Update available" with restart/dismiss buttons -->
</div>
{/if}

{#if updater.showWhatsNew}
<div class="modal-overlay" onclick={() => updater.showWhatsNew = false}>
  <div class="whats-new-modal" onclick|stopPropagation>
    <!-- Release notes rendered as HTML -->
  </div>
</div>
{/if}

<style>
  /* Move .update-toast, .whats-new-modal styles */
</style>
```

- [ ] **Step 3: Use both in `+page.svelte`**

```svelte
<DeleteConfirmModal
  profile={deleteConfirm}
  onConfirm={confirmDelete}
  onCancel={() => deleteConfirm = null}
/>
<UpdateToast />
```

- [ ] **Step 4: Verify build**

Run: `cd /Users/macbook/Personal/CtxMgr/Clauge && bun run build 2>&1 | tail -5`

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/DeleteConfirmModal.svelte src/lib/components/UpdateToast.svelte src/routes/+page.svelte
git commit -m "refactor(svelte): extract DeleteConfirmModal and UpdateToast components"
```

---

### Task 26: Clean up dead component files

**Files:**
- Delete or overwrite: `src/lib/components/Sidebar.svelte` (dead code)
- Delete or overwrite: `src/lib/components/Terminal.svelte` (dead code)
- Delete or overwrite: `src/lib/components/NewSessionModal.svelte` (dead code)
- Delete or overwrite: `src/lib/components/ContextMenu.svelte` (dead code)

- [ ] **Step 1: Verify none of these are imported**

Run: `grep -r "import.*from.*components/Sidebar\|import.*from.*components/Terminal\|import.*from.*components/NewSessionModal\|import.*from.*components/ContextMenu" src/`

Expected: No matches (or only matches in the new components you created, not in `+page.svelte`).

- [ ] **Step 2: Delete dead files**

```bash
rm src/lib/components/Sidebar.svelte
rm src/lib/components/Terminal.svelte
rm src/lib/components/NewSessionModal.svelte
rm src/lib/components/ContextMenu.svelte
```

- [ ] **Step 3: Verify build**

Run: `cd /Users/macbook/Personal/CtxMgr/Clauge && bun run build 2>&1 | tail -5`

- [ ] **Step 4: Commit**

```bash
git add -A src/lib/components/
git commit -m "refactor(svelte): remove dead component files that were never imported"
```

---

## Phase 4: Final Verification

### Task 27: Full build + line count audit

- [ ] **Step 1: Full Rust build**

Run: `cd /Users/macbook/Personal/CtxMgr/Clauge && cargo build --manifest-path src-tauri/Cargo.toml 2>&1`

Expected: Success, no warnings.

- [ ] **Step 2: Full Svelte build**

Run: `cd /Users/macbook/Personal/CtxMgr/Clauge && bun run build 2>&1`

Expected: Success.

- [ ] **Step 3: Svelte type check**

Run: `cd /Users/macbook/Personal/CtxMgr/Clauge && bun run check 2>&1`

Expected: No errors.

- [ ] **Step 4: Line count audit**

Run:
```bash
echo "=== Rust ===" && wc -l src-tauri/src/*.rs && echo "=== Svelte Stores ===" && wc -l src/lib/stores/*.ts 2>/dev/null; wc -l src/lib/stores/*.svelte.ts 2>/dev/null && echo "=== Svelte Components ===" && wc -l src/lib/components/*.svelte && echo "=== Page ===" && wc -l src/routes/+page.svelte
```

Expected:
- `lib.rs`: ~120-150 lines
- Each Rust module: 50-300 lines
- `+page.svelte`: ~500-800 lines (still has terminal DOM management + sidebar markup + new session modal markup)
- Each store: 50-250 lines
- Each component: 40-350 lines

- [ ] **Step 5: File structure check**

Run:
```bash
find src-tauri/src -name '*.rs' | sort && echo "---" && find src/lib -name '*.svelte' -o -name '*.svelte.ts' | sort
```

Expected structure matches the spec.

- [ ] **Step 6: Commit final state**

```bash
git add -A
git commit -m "refactor: complete codebase decomposition — lib.rs and +page.svelte split into modules"
```

---

## Summary

| Phase | Tasks | What gets extracted |
|-------|-------|-------------------|
| Phase 1 (Rust) | Tasks 1-12 | `lib.rs` → 11 focused modules + thin orchestrator |
| Phase 2 (Svelte Stores) | Tasks 13-20 | State + logic → 10 `.svelte.ts` store files |
| Phase 3 (Svelte Components) | Tasks 21-26 | Markup + styles → 6 new components + cleanup of 4 dead files |
| Phase 4 (Verify) | Task 27 | Full build, type check, line count audit |

Total: 27 tasks, ~80 steps. Each step is atomic and independently verifiable.
