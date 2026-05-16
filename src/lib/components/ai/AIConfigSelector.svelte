<script lang="ts">
  import { cloudPlan } from '$lib/stores/cloud';

  type AiConfig = {
    id: number;
    label: string;
    provider: string;
    baseUrl: string | null;
    defaultModel: string | null;
    isDefault: number;
    createdAt: string;
    lastUsedAt: string | null;
  };

  type SelectorValue = string | null;

  let {
    value = $bindable<SelectorValue>(null),
    configs = [],
    onUpgradeClick,
  }: {
    value?: SelectorValue;
    configs?: AiConfig[];
    onUpgradeClick: () => void;
  } = $props();

  const isPro = $derived($cloudPlan === 'pro');

  const effectiveValue = $derived.by(() => {
    if (value) return value;
    if (isPro) return 'clauge';
    const def = configs.find((c) => c.isDefault === 1);
    return def ? `config:${def.id}` : null;
  });

  function handleChange(e: Event) {
    const target = e.target as HTMLSelectElement;
    const next = target.value;
    if (next === 'clauge' && !isPro) {
      target.value = effectiveValue ?? '';
      onUpgradeClick();
      return;
    }
    value = next;
  }
</script>

<select class="ai-selector" value={effectiveValue ?? ''} onchange={handleChange}>
  <option value="clauge">
    Clauge AI{!isPro ? ' (PRO)' : ''}
  </option>
  {#each configs as cfg (cfg.id)}
    <option value={`config:${cfg.id}`}>
      {cfg.label}
    </option>
  {/each}
  {#if configs.length === 0 && !isPro}
    <option disabled value="">No providers configured</option>
  {/if}
</select>

<style>
  .ai-selector {
    background: var(--n2, #0e0e0e);
    color: var(--t1, #ddd);
    border: 1px solid var(--b1, #2a2a2a);
    border-radius: var(--radius-md, 6px);
    padding: 0.25rem 0.5rem;
    font-size: 0.85rem;
    font-family: var(--ui);
    cursor: pointer;
    max-width: 200px;
  }
  .ai-selector:hover { border-color: var(--b2, #3a3a3a); }
  .ai-selector:focus { outline: 2px solid var(--acc, #4a90e2); outline-offset: -1px; }
</style>
