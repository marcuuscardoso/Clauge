# Clauge Codebase Refactoring — Design Spec

> Date: 2026-04-22

## Goal

Decompose the two monolithic files (`lib.rs` at 2,074 lines and `+page.svelte` at 2,896 lines) into focused, single-responsibility modules and components — without changing any user-facing behavior.

## Approach

**Hybrid decomposition:** Domain modules in Rust, reactive stores + extracted components in Svelte 5.

## Constraints

- Zero functional regressions — app must work identically before and after
- No new dependencies added
- Existing extracted components (`Sidebar`, `Terminal`, `NewSessionModal`, `ContextMenu`) remain untouched
- All Tauri command signatures stay identical (frontend invoke calls don't change)
- No changes to `tauri.conf.json`, `Cargo.toml`, or `package.json`

---

## Rust Backend

### Current State

Single `lib.rs` (2,074 lines) containing all structs, all Tauri commands, all business logic, and app setup.

### Target Structure

```
src-tauri/src/
├── lib.rs              # Module declarations, app builder setup (~200 lines)
├── main.rs             # Unchanged
├── models.rs           # All shared structs and types
├── storage.rs          # Profile persistence helpers
├── profiles.rs         # Profile CRUD Tauri commands
├── git.rs              # Git operation Tauri commands
├── worktree.rs         # Git worktree Tauri commands
├── terminal.rs         # PTY spawn/write/resize/kill commands
├── plugins.rs          # Plugin management Tauri commands
├── usage.rs            # Usage analytics + limits Tauri commands
├── sessions.rs         # Session discovery Tauri commands
└── system.rs           # App version, session key, tray, claude plan commands
```

### Module Breakdown

#### `models.rs` (~150 lines)

All shared data types. Every other module imports from here.

```rust
// Structs to move here:
pub struct SessionProfile { .. }
pub struct SessionStore { .. }
pub struct DiscoveredSession { .. }
pub struct TokenUsage { .. }
pub struct ClaudePlugin { .. }
pub struct MarketplacePlugin { .. }
pub struct GitFileChange { .. }
pub struct UsageAnalytics { .. }
pub struct DailyUsage { .. }
pub struct ModelUsage { .. }
pub struct ProjectUsage { .. }
pub struct SessionCost { .. }
pub struct ToolCount { .. }
pub struct TerminalOutputPayload { .. }
pub struct TerminalState { .. }   // + Default impl
```

All derive macros (`Serialize`, `Deserialize`, `Clone`, `Debug`) and serde attributes stay as-is.

#### `storage.rs` (~80 lines)

Internal helpers for profile persistence. Not Tauri commands — called by `profiles.rs` and `sessions.rs`.

```rust
pub fn get_storage_dir() -> PathBuf
pub fn get_storage_path() -> PathBuf
pub fn load_profiles() -> Vec<SessionProfile>
pub fn save_profiles(profiles: &[SessionProfile]) -> Result<(), String>
pub fn encode_project_path(project_path: &str) -> String
pub fn now_iso8601() -> String
pub fn project_name_from_path(project_path: &str) -> String
```

#### `profiles.rs` (~200 lines)

Profile management Tauri commands + the purpose prompt generator.

```rust
pub fn get_context_prompt(purpose: &str) -> String  // also used by terminal.rs

#[tauri::command] pub fn get_profiles() -> Result<Vec<SessionProfile>, String>
#[tauri::command] pub fn create_profile(..) -> Result<SessionProfile, String>
#[tauri::command] pub fn delete_profile(id: String) -> Result<(), String>
#[tauri::command] pub fn rename_profile(..) -> Result<(), String>
#[tauri::command] pub fn update_last_used(id: String) -> Result<(), String>
#[tauri::command] pub fn refresh_session_ids() -> Result<Vec<SessionProfile>, String>
#[tauri::command] pub fn update_session_id(..) -> Result<(), String>
```

Depends on: `models`, `storage`

#### `git.rs` (~300 lines)

All git operation Tauri commands.

```rust
#[tauri::command] pub fn get_git_status(..) -> Result<Vec<GitFileChange>, String>
#[tauri::command] pub fn get_git_branch(..) -> Result<String, String>
#[tauri::command] pub fn get_git_ahead_behind(..) -> Result<(u32, u32), String>
#[tauri::command] pub fn git_commit(..) -> Result<String, String>
#[tauri::command] pub fn git_push(..) -> Result<String, String>
#[tauri::command] pub fn git_pull(..) -> Result<String, String>
#[tauri::command] pub fn git_diff_file(..) -> Result<String, String>
#[tauri::command] pub fn git_stage_file(..) -> Result<(), String>
#[tauri::command] pub fn git_unstage_file(..) -> Result<(), String>
#[tauri::command] pub fn git_log(..) -> Result<Vec<serde_json::Value>, String>
#[tauri::command] pub fn git_stash(..) -> Result<String, String>
#[tauri::command] pub fn git_stash_pop(..) -> Result<String, String>
#[tauri::command] pub fn git_list_branches(..) -> Result<Vec<serde_json::Value>, String>
#[tauri::command] pub fn git_switch_branch(..) -> Result<(), String>
```

Depends on: `models`

#### `worktree.rs` (~120 lines)

Git worktree management Tauri commands.

```rust
#[tauri::command] pub fn is_git_repo(path: String) -> Result<bool, String>
#[tauri::command] pub fn create_worktree(..) -> Result<String, String>
#[tauri::command] pub fn remove_worktree(..) -> Result<(), String>
#[tauri::command] pub fn update_profile_worktree(..) -> Result<(), String>
```

Depends on: `models`, `storage`

#### `terminal.rs` (~300 lines)

PTY management Tauri commands + claude binary resolution.

```rust
fn resolve_claude_path() -> String  // internal helper

#[tauri::command] pub fn spawn_terminal(..) -> Result<String, String>
#[tauri::command] pub fn spawn_shell(..) -> Result<String, String>
#[tauri::command] pub fn write_to_terminal(..) -> Result<(), String>
#[tauri::command] pub fn resize_terminal(..) -> Result<(), String>
#[tauri::command] pub fn kill_terminal(..) -> Result<(), String>
```

Depends on: `models` (for `TerminalState`, `TerminalOutputPayload`), `profiles` (for `get_context_prompt`)

#### `plugins.rs` (~200 lines)

Plugin management Tauri commands.

```rust
#[tauri::command] pub fn get_claude_plugins() -> Result<Vec<ClaudePlugin>, String>
#[tauri::command] pub fn toggle_claude_plugin(..) -> Result<(), String>
#[tauri::command] pub fn get_marketplace_plugins() -> Result<Vec<MarketplacePlugin>, String>
#[tauri::command] pub fn install_plugin(..) -> Result<(), String>
#[tauri::command] pub fn uninstall_plugin(..) -> Result<(), String>
```

Depends on: `models`

#### `usage.rs` (~300 lines)

Usage analytics (the big .jsonl parser) and live usage limits.

```rust
#[tauri::command] pub async fn get_usage_analytics(days: Option<u32>) -> Result<UsageAnalytics, String>
fn get_usage_analytics_sync(days: Option<u32>) -> Result<UsageAnalytics, String>  // spawn_blocking target
#[tauri::command] pub fn fetch_usage_limits(session_key: String) -> Result<serde_json::Value, String>
```

Depends on: `models`, `storage` (for `encode_project_path`)

#### `sessions.rs` (~150 lines)

Session discovery Tauri commands.

```rust
#[tauri::command] pub fn count_project_sessions(..) -> Result<u32, String>
#[tauri::command] pub fn discover_sessions(..) -> Result<Vec<DiscoveredSession>, String>
#[tauri::command] pub fn get_session_tokens(..) -> Result<TokenUsage, String>
```

Depends on: `models`, `storage` (for `encode_project_path`)

#### `system.rs` (~80 lines)

Miscellaneous system Tauri commands.

```rust
#[tauri::command] pub fn save_session_key(key: String) -> Result<(), String>
#[tauri::command] pub fn load_session_key() -> Result<Option<String>, String>
#[tauri::command] pub fn get_app_version() -> String
#[tauri::command] pub fn get_claude_plan() -> Result<String, String>
#[tauri::command] pub fn update_tray_title(..) -> Result<(), String>
```

Depends on: `storage` (for `get_storage_dir`)

#### `lib.rs` (~200 lines)

Thin orchestrator: module declarations, re-exports, app builder.

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
mod system;

pub use models::*;  // TerminalState needed by managed state

pub fn run() {
    tauri::Builder::default()
        .plugin(...)       // all plugins unchanged
        .manage(...)       // TerminalState
        .setup(...)        // menu, tray, vibrancy — stays here
        .invoke_handler(tauri::generate_handler![
            // all commands referenced by module path
            profiles::get_profiles,
            profiles::create_profile,
            // ... etc
        ])
        .run(...)
}
```

### Dependency Graph

```
models.rs  <--  storage.rs  <--  profiles.rs
    ^               ^               |
    |               |               v
    +--- git.rs     +--- worktree.rs
    |               |
    +--- terminal.rs (also depends on profiles for get_context_prompt)
    |
    +--- plugins.rs
    |
    +--- usage.rs  (also depends on storage)
    |
    +--- sessions.rs (also depends on storage)
    |
    +--- system.rs (also depends on storage)
```

No circular dependencies. `models` is the root. `storage` is the shared persistence layer.

---

## Svelte Frontend

### Current State

Single `+page.svelte` (2,896 lines) with ~80 functions, ~50 state variables, all markup and styles.

### Target Structure

```
src/
├── routes/
│   └── +page.svelte              # Thin layout orchestrator (~200 lines)
└── lib/
    ├── stores/
    │   ├── profiles.svelte.ts     # Profile state + selection logic
    │   ├── terminal.svelte.ts     # Terminal map, creation, WebGL, session capture
    │   ├── shell.svelte.ts        # Shell panel state + management
    │   ├── git.svelte.ts          # Git state + all git operations
    │   ├── usage.svelte.ts        # Usage limits + dashboard data
    │   ├── plugins.svelte.ts      # Plugin state + CRUD
    │   ├── theme.svelte.ts        # Theme + accent color
    │   ├── notifications.svelte.ts # Action prompt detection + sound
    │   ├── updater.svelte.ts      # Auto-update state + logic
    │   └── contexts.svelte.ts     # Context snippets CRUD
    └── components/
        ├── Sidebar.svelte         # EXISTS — unchanged
        ├── Terminal.svelte        # EXISTS — unchanged
        ├── NewSessionModal.svelte # EXISTS — unchanged
        ├── ContextMenu.svelte     # EXISTS — unchanged
        ├── GitPanel.svelte        # NEW — git popup (changes/history/branches tabs)
        ├── BottomBar.svelte       # NEW — bottom status bar
        ├── UsageDashboard.svelte  # NEW — usage dashboard modal
        ├── SettingsModal.svelte   # NEW — settings (appearance/plugins/about)
        ├── ShellPanel.svelte      # NEW — shell terminal + divider
        ├── UpdateToast.svelte     # NEW — update notification + what's new
        └── DeleteConfirmModal.svelte # NEW — delete confirmation dialog
```

### Store Design Pattern

Each store is a Svelte 5 module using `$state` runes, exporting reactive variables and functions:

```typescript
// Example: git.svelte.ts
import { invoke } from "@tauri-apps/api/core";

// Reactive state
export let gitBranch = $state("");
export let gitFiles = $state([]);
// ...

// Operations
export async function refreshGitStatus(projectPath) { ... }
export async function doGitCommit(projectPath, message) { ... }
// ...
```

### Store Breakdown

#### `profiles.svelte.ts` (~120 lines)

```typescript
// State
export let profiles = $state([]);
export let activeProfile = $state(null);
export let statusMsg = $state("");

// Functions
export async function loadProfiles()
export async function createSession(opts)  // delegates to terminal/shell stores
export function deleteProfile(e, profile)
export async function confirmDelete()      // delegates cleanup to terminal/shell/worktree
export function groupByProject(list)
export function relativeTime(iso)
```

#### `terminal.svelte.ts` (~250 lines)

```typescript
// State
export let terminalMap = $state(new Map());
export let activeTermEntry = $state(null);
export let currentTerminalId = $state(null);
export let termFontSize = $state(13);
export let sessionActivity = $state({});

// Functions
export function getTermConfig()
export function createTermEntry(profileId)
export function showTermEntry(entry)
export async function loadWebGLAddon(term)
export function handleFileDrop(e)
// Session ID capture logic (internal)
// Exit detection logic (internal)
```

#### `shell.svelte.ts` (~180 lines)

```typescript
// State
export let shellMap = $state(new Map());
export let shellOpenMap = $state({});
export let activeShellEntry = $state(null);
export let shellWidthMap = $state({});
export let isDraggingDivider = $state(false);
export let focusedPanel = $state("claude");

// Functions
export function createShellEntry(profileId)
export function showShellEntry(sEntry)
export async function spawnShellForProfile(profile)
export async function toggleShell()
export function startDividerDrag(e)
export function getShellWidth(profileId)
```

#### `git.svelte.ts` (~250 lines)

```typescript
// State
export let gitBranch = $state("");
export let gitFiles = $state([]);
export let gitChanges = $state({});
export let gitPanelOpen = $state(false);
export let gitTab = $state("changes");
export let gitCommitMsg = $state("");
export let gitLoading = $state("");
export let gitAhead = $state(0);
export let gitBehind = $state(0);
export let gitMsg = $state("");
export let gitDiff = $state("");
export let gitDiffFile = $state("");
export let gitCommits = $state([]);
export let gitBranches = $state([]);
export let stagedFiles = $state(new Set());

// Functions
export async function refreshGitStatus()
export function showGitMsg(msg, duration)
export async function doGitCommit()
export async function doGitCommitStaged()
export async function doGitPush()
export async function doGitPull()
export async function viewDiff(file)
export async function toggleStageFile(file)
export async function loadGitHistory()
export async function loadGitBranches()
export async function switchBranch(branchName)
export async function doGitStash()
export async function doGitStashPop()
```

#### `usage.svelte.ts` (~150 lines)

```typescript
// State
export let usageLimits = $state(null);
export let sessionKeyConfigured = $state(false);
export let sessionKeyInput = $state("");
export let showKeyEdit = $state(false);
export let usageError = $state("");
export let usageRefreshMins = $state(5);
export let dashboardData = $state(null);
export let dashboardDays = $state(14);
export let dashboardLoading = $state(false);

// Functions
export async function loadUsageLimits()
export async function loadDashboard()
export function formatCost(v)
export function formatTokens(v)
export function decodeProjectName(encoded)
```

#### `plugins.svelte.ts` (~100 lines)

```typescript
// State
export let claudePlugins = $state([]);
export let marketplacePlugins = $state([]);
export let pluginSearch = $state("");
export let pluginTab = $state("installed");
export let installingPlugin = $state("");

// Functions
export async function loadClaudePlugins()
export async function togglePlugin(plugin)
export async function installPlugin(plugin)
export async function uninstallPlugin(plugin)
```

#### `theme.svelte.ts` (~50 lines)

```typescript
// State
export let currentTheme = $state("dark");
export let accentColor = $state("#58a6ff");

// Functions
export function applyTheme(themeName)
export function applyAccent(color)
```

#### `notifications.svelte.ts` (~100 lines)

```typescript
// Functions (stateless — uses internal buffers)
export function checkForActionPrompt(base64Data, sessionTitle)
export function playNotificationSound()
export async function sendActionNotification(sessionTitle)
```

#### `updater.svelte.ts` (~80 lines)

```typescript
// State
export let updateReady = $state(false);
export let pendingUpdate = $state(null);
export let updateDismissed = $state(false);
export let showWhatsNew = $state(false);
export let whatsNewBody = $state("");

// Functions
export async function checkAndDownloadUpdate()
export async function restartToUpdate()
export function checkWhatsNew(version)
```

#### `contexts.svelte.ts` (~50 lines)

```typescript
// State
export let contextSnippets = $state([]);
export let newSnippetName = $state("");
export let newSnippetContent = $state("");

// Functions
export async function loadContextSnippets()
export async function saveContextSnippet()
export async function deleteContextSnippet(name)
export async function attachContextsToSession(profileId, projectPath, contextNames)
export async function detachContextsFromSession(profileId, projectPath)
```

### Component Breakdown

#### `GitPanel.svelte` (~350 lines)

Props: receives git store state and functions.

Renders:
- Tab bar (changes / history / branches)
- Changes tab: file list with status badges, selective staging checkboxes, inline diff viewer, commit input + button
- History tab: commit list (hash, message, author, date)
- Branches tab: branch list with current indicator, click to switch
- Action row: Stash, Pop Stash, Pull, Push buttons with loading states

#### `BottomBar.svelte` (~150 lines)

Props: profile avatar state, git summary, usage limits, shell toggle, version.

Renders:
- Left: profile avatar (opens menu with Settings/Plugins/Dashboard/What's New/Report Issue/Coffee links), git status bar
- Center: usage limit chips (clickable to open dashboard)
- Right: shell toggle button (Cmd+L), version number

#### `UsageDashboard.svelte` (~300 lines)

Props: dashboard data, loading state, days selection.

Renders:
- Summary cards (total cost, API calls, sessions, cache hit rate)
- Daily chart (cost + calls per day)
- By Model breakdown table
- By Project breakdown table
- Top 5 sessions table
- Tool usage + shell commands lists
- Live usage bars (session/weekly/sonnet)
- Session key config input

#### `SettingsModal.svelte` (~200 lines)

Props: theme/accent state, plugin state.

Renders:
- Sidebar tabs (Appearance / Plugins / About)
- Appearance: theme toggle, accent color presets, font size slider
- Plugins: installed/marketplace sub-tabs with toggle/install/uninstall
- About: version, links

#### `ShellPanel.svelte` (~80 lines)

Props: shell entry, width, drag state.

Renders:
- Shell terminal container
- Resize divider (drag handle between claude terminal and shell)

#### `UpdateToast.svelte` (~80 lines)

Props: update state, what's new state.

Renders:
- Bottom-right toast when update downloaded ("Restart to Update" / dismiss)
- What's New modal with release notes

#### `DeleteConfirmModal.svelte` (~40 lines)

Props: profile to delete, confirm callback.

Renders:
- Confirmation dialog with profile name, cancel/delete buttons

### Resulting `+page.svelte` (~200 lines)

```svelte
<script>
  import { onMount } from "svelte";

  // Stores
  import * as profiles from "$lib/stores/profiles.svelte";
  import * as terminal from "$lib/stores/terminal.svelte";
  import * as shell from "$lib/stores/shell.svelte";
  import * as git from "$lib/stores/git.svelte";
  import * as usage from "$lib/stores/usage.svelte";
  import * as plugins from "$lib/stores/plugins.svelte";
  import * as theme from "$lib/stores/theme.svelte";
  import * as notifications from "$lib/stores/notifications.svelte";
  import * as updater from "$lib/stores/updater.svelte";
  import * as contexts from "$lib/stores/contexts.svelte";

  // Components
  import Sidebar from "$lib/components/Sidebar.svelte";
  import Terminal from "$lib/components/Terminal.svelte";
  import NewSessionModal from "$lib/components/NewSessionModal.svelte";
  import ContextMenu from "$lib/components/ContextMenu.svelte";
  import GitPanel from "$lib/components/GitPanel.svelte";
  import BottomBar from "$lib/components/BottomBar.svelte";
  import UsageDashboard from "$lib/components/UsageDashboard.svelte";
  import SettingsModal from "$lib/components/SettingsModal.svelte";
  import ShellPanel from "$lib/components/ShellPanel.svelte";
  import UpdateToast from "$lib/components/UpdateToast.svelte";
  import DeleteConfirmModal from "$lib/components/DeleteConfirmModal.svelte";

  // App-level state (modals, layout)
  let showModal = $state(false);
  let showSettings = $state(false);
  let showDashboard = $state(false);
  let sidebarCollapsed = $state(false);
  let expandedGroups = $state({});
  let deleteConfirm = $state(null);

  // App-level DOM refs
  let terminalEl, shellEl, wrapperEl;

  // App-level functions (keyboard, resize, drag)
  function handleGlobalKeydown(e) { ... }
  function handleWindowResize() { ... }
  async function handleDragStart(e) { ... }
  function openExternal(url) { ... }

  onMount(() => {
    profiles.loadProfiles();
    usage.loadUsageLimits();
    updater.checkAndDownloadUpdate();
    theme.applyTheme(theme.currentTheme);
    theme.applyAccent(theme.accentColor);
    // intervals, event listeners...
  });
</script>

<!-- Layout: sidebar + main + shell -->
<!-- Each section delegates to its component -->
```

---

## Migration Strategy

Each step must leave the app fully functional. Build and test after every step.

### Phase 1: Rust Backend (do first — frontend is unaffected)

1. Create `models.rs` — move all structs, add `pub` visibility
2. Create `storage.rs` — move persistence helpers
3. Create each domain module one at a time (`profiles` -> `git` -> `worktree` -> `terminal` -> `plugins` -> `usage` -> `sessions` -> `system`)
4. After each module: update `lib.rs` imports, verify `cargo build` passes
5. Final `lib.rs`: only module declarations + app builder + `generate_handler![]`

### Phase 2: Svelte Stores (extract logic, don't touch markup yet)

1. Create store files one at a time, starting with leaf stores (`theme` -> `notifications` -> `updater` -> `contexts`)
2. Move state variables and functions to each store
3. Update `+page.svelte` to import from stores instead of local declarations
4. After each store: verify `bun run build` passes and app works

### Phase 3: Svelte Components (extract markup + styles)

1. Extract components one at a time, starting with modals (`DeleteConfirmModal` -> `SettingsModal` -> `UsageDashboard`)
2. Then layout components (`BottomBar` -> `GitPanel` -> `ShellPanel` -> `UpdateToast`)
3. After each component: verify build passes and UI renders correctly
4. Final `+page.svelte`: thin orchestrator with layout grid

### Verification

After each phase:
- `cargo build` (Rust) — must compile with no errors or warnings
- `bun run build` (Frontend) — must build with no errors
- `bun run check` (Svelte) — must pass type checking
- Manual test: create session, run terminal, use git panel, check usage, toggle shell, switch themes
