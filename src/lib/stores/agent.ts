import { writable, get } from 'svelte/store';
import type { AgentSession, AgentContext, ContextUsage, GitFileChange } from '$lib/types/agent';
import { agentListSessions, agentListContexts, agentGitStatus, agentGitBranch, agentGitAheadBehind, agentGetSessionContextUsage, agentFetchUsageLimits, agentUpdateTrayTitle, agentGetClaudePlan } from '$lib/commands/agent';

// Sessions
export const agentSessions = writable<AgentSession[]>([]);
export const activeAgentSession = writable<AgentSession | null>(null);

// Terminal tracking (frontend-only state)
export const agentTerminalMap = writable<Map<string, any>>(new Map());
export const agentShellMap = writable<Map<string, any>>(new Map());
export const agentTerminalIds = writable<Map<string, string>>(new Map());
export const agentShellIds = writable<Map<string, string>>(new Map());
export const agentShellOpen = writable<boolean>(false);

// Context usage per session
export const agentContextUsage = writable<Map<string, ContextUsage>>(new Map());

// Git state for active session
export const agentGitBranchName = writable<string>('');
export const agentGitFiles = writable<GitFileChange[]>([]);
export const agentGitAhead = writable<number>(0);
export const agentGitBehind = writable<number>(0);

// Contexts
export const agentContexts = writable<AgentContext[]>([]);

// Session activity tracking
export const agentSessionActivity = writable<Map<string, 'running' | 'done'>>(new Map());

// Notification preferences (loaded from settings)
export const agentNotifyEnabled = writable<boolean>(false);
export const agentSoundEnabled = writable<boolean>(false);
export const agentDockBounceEnabled = writable<boolean>(false);

// Usage limits (fetched from Claude AI API)
export const agentUsageLimits = writable<any>(null);
export const agentSessionKey = writable<string>('');

// Claude subscription plan
export const agentClaudePlan = writable<string>('');

export async function loadAgentClaudePlan() {
  try {
    const plan = await agentGetClaudePlan();
    agentClaudePlan.set(plan);
  } catch { /* ignore */ }
}

export async function loadAgentUsageLimits() {
  const key = get(agentSessionKey);
  if (!key) return;
  try {
    const limits = await agentFetchUsageLimits(key);
    agentUsageLimits.set(limits);
    // Update tray title with usage stats
    try {
      const standard = limits?.standard;
      const extended = limits?.extended;
      const parts: string[] = [];
      if (standard && typeof standard.percentUsed === 'number') {
        parts.push(`S:${Math.round(standard.percentUsed)}%`);
      }
      if (extended && typeof extended.percentUsed === 'number') {
        parts.push(`W:${Math.round(extended.percentUsed)}%`);
      }
      if (parts.length > 0) {
        await agentUpdateTrayTitle(parts.join(' '));
      }
    } catch { /* tray update best-effort */ }
  } catch { /* ignore */ }
}

export async function loadAgentSessions() {
  try {
    const sessions = await agentListSessions();
    agentSessions.set(sessions);
  } catch (e) {
    console.error('Failed to load agent sessions:', e);
  }
}

export async function loadAgentContexts() {
  try {
    const contexts = await agentListContexts();
    agentContexts.set(contexts);
  } catch (e) {
    console.error('Failed to load agent contexts:', e);
  }
}

export async function refreshAgentGitStatus() {
  const session = get(activeAgentSession);
  if (!session) return;
  const projectPath = session.worktreePath || session.projectPath;
  try {
    const [branch, files, [ahead, behind]] = await Promise.all([
      agentGitBranch(projectPath),
      agentGitStatus(projectPath),
      agentGitAheadBehind(projectPath),
    ]);
    agentGitBranchName.set(branch);
    agentGitFiles.set(files);
    agentGitAhead.set(ahead);
    agentGitBehind.set(behind);
  } catch { /* ignore — not a git repo */ }
}

export async function refreshAgentContextUsage(sessionId: string, projectPath: string, claudeSessionId: string) {
  try {
    const usage = await agentGetSessionContextUsage(projectPath, claudeSessionId);
    agentContextUsage.update(m => { m.set(sessionId, usage); return new Map(m); });
  } catch { /* ignore */ }
}
