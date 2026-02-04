<script lang="ts">
  import Icon from "$lib/components/common/Icon.svelte";
  import { t, isConnected } from "$lib/stores";

  let meterTime = $state("14:30:35");
  let meterDate = $state("2024-12-15");
  let computerTime = $state(new Date().toLocaleTimeString("en-GB"));
  let computerDate = $state(new Date().toISOString().split("T")[0]);

  // Update computer time every second
  $effect(() => {
    const interval = setInterval(() => {
      const now = new Date();
      computerTime = now.toLocaleTimeString("en-GB");
      computerDate = now.toISOString().split("T")[0];
    }, 1000);

    return () => clearInterval(interval);
  });
</script>

<div class="space-y-6">
  <div
    class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm"
  >
    <h3 class="text-xl font-bold text-slate-900 dark:text-white mb-2">{$t.timeSync}</h3>
    <p class="text-sm text-slate-500 dark:text-slate-400">
      Synchronize meter time with computer or set manually.
    </p>

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

  <!-- Time Display -->
  <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
    <!-- Meter Time -->
    <div
      class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm"
    >
      <h4 class="font-bold text-slate-900 dark:text-white mb-4 flex items-center gap-2">
        <Icon name="electric_meter" class="text-primary" />
        {$t.currentMeterTime}
      </h4>
      <div class="text-center py-8">
        <div class="text-5xl font-mono font-bold text-slate-900 dark:text-white mb-2">
          {meterTime}
        </div>
        <div class="text-xl font-mono text-slate-500">{meterDate}</div>
      </div>
    </div>

    <!-- Computer Time -->
    <div
      class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm"
    >
      <h4 class="font-bold text-slate-900 dark:text-white mb-4 flex items-center gap-2">
        <Icon name="computer" class="text-primary" />
        {$t.computerTime}
      </h4>
      <div class="text-center py-8">
        <div class="text-5xl font-mono font-bold text-primary mb-2">{computerTime}</div>
        <div class="text-xl font-mono text-slate-500">{computerDate}</div>
      </div>
    </div>
  </div>

  <!-- Sync Options -->
  <div
    class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm"
  >
    <h4 class="font-bold text-slate-900 dark:text-white mb-4">Synchronization Options</h4>
    <div class="flex flex-col md:flex-row gap-4">
      <button
        disabled={!$isConnected}
        class="flex-1 flex items-center justify-center gap-2 px-6 py-4 bg-primary hover:bg-primary/90 text-white font-bold rounded-xl transition-all disabled:opacity-50 disabled:cursor-not-allowed"
      >
        <Icon name="sync" />
        {$t.syncToComputerTime}
      </button>
      <button
        disabled={!$isConnected}
        class="flex-1 flex items-center justify-center gap-2 px-6 py-4 bg-slate-100 dark:bg-[#334a5e] hover:bg-slate-200 dark:hover:bg-[#455a6e] text-slate-700 dark:text-white font-bold rounded-xl transition-all disabled:opacity-50 disabled:cursor-not-allowed"
      >
        <Icon name="edit" />
        {$t.manualTimeEntry}
      </button>
    </div>
  </div>
</div>
