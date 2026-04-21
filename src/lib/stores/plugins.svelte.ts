import { invoke } from "@tauri-apps/api/core";

class PluginsStore {
  claudePlugins = $state<any[]>([]);
  marketplacePlugins = $state<any[]>([]);
  pluginSearch = $state('');
  installingPlugin = $state('');
  pluginTab = $state('installed');
  pluginMsg = $state('');
  pluginUninstallConfirm = $state<any>(null);

  async loadClaudePlugins() {
    try { this.claudePlugins = await invoke("get_claude_plugins"); } catch(_) { this.claudePlugins = []; }
    try { this.marketplacePlugins = await invoke("get_marketplace_plugins"); } catch(_) { this.marketplacePlugins = []; }
  }

  async togglePlugin(plugin: any) {
    const key = `${plugin.name}@${plugin.marketplace}`;
    try { await invoke("toggle_claude_plugin", { pluginKey: key, enabled: !plugin.enabled }); await this.loadClaudePlugins(); } catch(_) {}
  }

  async installPlugin(plugin: any) {
    this.installingPlugin = plugin.name;
    this.pluginMsg = '';
    try {
      await invoke("install_plugin", { name: plugin.name, marketplace: plugin.marketplace });
      await invoke("toggle_claude_plugin", { pluginKey: `${plugin.name}@${plugin.marketplace}`, enabled: true });
      await this.loadClaudePlugins();
      this.pluginMsg = `${plugin.name} installed`;
      setTimeout(() => { if (this.pluginMsg.includes('installed')) this.pluginMsg = ''; }, 3000);
    } catch(e) {
      this.pluginMsg = `Failed: ${String(e).slice(0, 60)}`;
      setTimeout(() => { this.pluginMsg = ''; }, 5000);
    }
    this.installingPlugin = '';
  }

  async uninstallPlugin(plugin: any) {
    this.pluginMsg = '';
    try {
      await invoke("uninstall_plugin", { name: plugin.name, marketplace: plugin.marketplace });
      await this.loadClaudePlugins();
      this.pluginMsg = `${plugin.name} uninstalled`;
      setTimeout(() => { if (this.pluginMsg.includes('uninstalled')) this.pluginMsg = ''; }, 3000);
    } catch(e) {
      this.pluginMsg = `Uninstall failed: ${String(e).slice(0, 60)}`;
      setTimeout(() => { this.pluginMsg = ''; }, 5000);
    }
    this.pluginUninstallConfirm = null;
  }
}

export const pluginsStore = new PluginsStore();
