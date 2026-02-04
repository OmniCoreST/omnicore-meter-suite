<script lang="ts">
  import Icon from "$lib/components/common/Icon.svelte";
  import { t, isConnected } from "$lib/stores";
</script>

<div class="space-y-6">
  <div
    class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm"
  >
    <div class="flex items-start justify-between">
      <div>
        <h3 class="text-xl font-bold text-slate-900 dark:text-white mb-2">{$t.fullRead}</h3>
        <p class="text-sm text-slate-500 dark:text-slate-400">{$t.fullReadDescription}</p>
      </div>
      <button
        disabled={!$isConnected}
        class="flex items-center gap-2 px-6 py-3 bg-primary hover:bg-primary/90 text-white font-bold rounded-lg shadow-lg shadow-primary/20 transition-all disabled:opacity-50 disabled:cursor-not-allowed"
      >
        <Icon name="play_arrow" />
        {$t.startFullRead}
      </button>
    </div>

    {#if !$isConnected}
      <div
        class="mt-4 p-4 bg-amber-500/10 border border-amber-500/20 rounded-xl text-amber-600 dark:text-amber-500 text-sm"
      >
        <div class="flex items-center gap-2">
          <Icon name="warning" />
          <span>Please connect to a meter first from the Dashboard.</span>
        </div>
      </div>
    {/if}
  </div>

  <div
    class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-12 shadow-sm text-center"
  >
    <Icon name="assignment" class="text-6xl text-slate-300 dark:text-slate-600 mb-4" />
    <p class="text-slate-500 dark:text-slate-400">
      Full reading functionality will display all meter data with historical records.
    </p>
  </div>
</div>
