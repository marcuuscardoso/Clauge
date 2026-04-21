import { invoke } from "@tauri-apps/api/core";

class UsageStore {
  usageLimits = $state<any>(null);
  sessionKeyInput = $state('');
  sessionKeyConfigured = $state(false);
  usageError = $state('');
  showKeyEdit = $state(false);
  usageRefreshMins = $state(5);
  showDashboard = $state(false);
  dashboardData = $state<any>(null);
  dashboardLoading = $state(false);
  dashboardDays = $state(30);

  private usageRefreshInterval: ReturnType<typeof setInterval> | null = null;

  constructor() {
    if (typeof localStorage !== 'undefined') {
      this.usageRefreshMins = parseInt(localStorage.getItem('clauge-usage-refresh') || '5');
    }
  }

  async loadUsageLimits() {
    this.usageError = '';
    try {
      const key = await invoke("load_session_key");
      if (!key) { this.sessionKeyConfigured = false; return; }

      const usage = await invoke<any>("fetch_usage_limits", { sessionKey: key });

      this.usageLimits = {
        sessionPercent: usage.five_hour?.utilization || 0,
        sessionResets: usage.five_hour?.resets_at || "",
        weeklyAllPercent: usage.seven_day?.utilization || 0,
        weeklyAllResets: usage.seven_day?.resets_at || "",
        weeklySonnetPercent: usage.seven_day_sonnet?.utilization ?? null,
        weeklySonnetResets: usage.seven_day_sonnet?.resets_at ?? null,
      };
      this.usageError = '';

      const s = Math.round(this.usageLimits.sessionPercent);
      const w = Math.round(this.usageLimits.weeklyAllPercent);
      await invoke("update_tray_title", { title: `S:${s}% W:${w}%` }).catch(() => {});
    } catch(e) {
      console.error("Usage limits failed:", e);
      const err = String(e).toLowerCase();
      this.usageLimits = null;
      await invoke("update_tray_title", { title: "" }).catch(() => {});

      // Detect auth failures — session key expired or invalid
      if (err.includes('permission') || err.includes('unauthorized') || err.includes('invalid') || err.includes('403') || err.includes('401')) {
        this.sessionKeyConfigured = false;
        this.usageError = 'Session key expired or invalid. Please reconnect.';
        if (this.usageRefreshInterval) { clearInterval(this.usageRefreshInterval); this.usageRefreshInterval = null; }
      } else {
        this.usageError = 'Failed to fetch usage data. Try again.';
      }
    }
  }

  startRefreshInterval() {
    if (this.usageRefreshInterval) clearInterval(this.usageRefreshInterval);
    this.usageRefreshInterval = setInterval(() => this.loadUsageLimits(), this.usageRefreshMins * 60 * 1000);
  }

  stopRefreshInterval() {
    if (this.usageRefreshInterval) { clearInterval(this.usageRefreshInterval); this.usageRefreshInterval = null; }
  }

  async loadDashboard() {
    this.dashboardLoading = true;
    try {
      this.dashboardData = await invoke("get_usage_analytics", { days: this.dashboardDays });
    } catch(e) { console.error('Dashboard failed:', e); this.dashboardData = null; }
    this.dashboardLoading = false;
  }

  formatCost(v: number): string { return v < 0.01 ? '<$0.01' : `$${v.toFixed(2)}`; }

  formatTokens(v: number): string { return v >= 1000000 ? `${(v/1000000).toFixed(1)}M` : v >= 1000 ? `${(v/1000).toFixed(1)}k` : String(v); }

  decodeProjectName(encoded: string): string {
    // Encoded paths use - for / and - for . — extract the last meaningful part
    const parts = encoded.split('-').filter(Boolean);
    // Find the last non-trivial segment (skip Users, macbook, etc.)
    for (let i = parts.length - 1; i >= 0; i--) {
      if (parts[i].length > 2 && !['Users','home','macbook','Personal','Projects','Documents','Desktop','Work'].includes(parts[i])) {
        return parts[i];
      }
    }
    return parts[parts.length - 1] || encoded;
  }
}

export const usageStore = new UsageStore();
