<script lang="ts">
  import Icon from "$lib/components/common/Icon.svelte";
  import { t, isConnected } from "$lib/stores";
  import { exportToExcel } from "$lib/utils/export";

  let dstEnabled = $state(true);

  // Mock DST periods
  let dstPeriods = $state([
    { id: 1, offset: "+01:00", forward: "2024-03-31 03:00", backward: "2024-10-27 04:00" },
    { id: 2, offset: "+01:00", forward: "2025-03-30 03:00", backward: "2025-10-26 04:00" },
    { id: 3, offset: "", forward: "", backward: "" },
    { id: 4, offset: "", forward: "", backward: "" },
    { id: 5, offset: "", forward: "", backward: "" },
    { id: 6, offset: "", forward: "", backward: "" },
  ]);

  function handleExport() {
    const exportData = dstPeriods.map(period => ({
      period: period.id,
      offset: period.offset || "-",
      forward: period.forward || "-",
      backward: period.backward || "-",
    }));

    exportToExcel(exportData, "dst_periods", [
      { key: "period", label: $t.periodNumber },
      { key: "offset", label: $t.timeOffset },
      { key: "forward", label: $t.forwardDate },
      { key: "backward", label: $t.backwardDate },
    ]);
  }
</script>

<div class="space-y-6">
  <div
    class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm"
  >
    <h3 class="text-xl font-bold text-slate-900 dark:text-white mb-2">{$t.dstSettings}</h3>
    <p class="text-sm text-slate-500 dark:text-slate-400">
      Configure daylight saving time periods for the meter.
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

  <!-- DST Status -->
  <div
    class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm"
  >
    <div class="flex items-center justify-between">
      <div>
        <h4 class="font-bold text-slate-900 dark:text-white mb-1">{$t.dstStatus}</h4>
        <p class="text-sm text-slate-500 dark:text-slate-400">
          Enable or disable daylight saving time adjustments.
        </p>
      </div>
      <button
        onclick={() => (dstEnabled = !dstEnabled)}
        disabled={!$isConnected}
        class="relative w-14 h-8 rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed
          {dstEnabled ? 'bg-primary' : 'bg-slate-300 dark:bg-slate-600'}"
      >
        <span
          class="absolute top-1 left-1 w-6 h-6 bg-white rounded-lg transition-transform shadow-md
            {dstEnabled ? 'translate-x-6' : ''}"
        ></span>
      </button>
    </div>
    <div class="mt-4 text-sm">
      Status:
      <span class="font-bold {dstEnabled ? 'text-emerald-500' : 'text-slate-500'}">
        {dstEnabled ? $t.enabled : $t.disabled}
      </span>
    </div>
  </div>

  <!-- DST Periods -->
  <div
    class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm"
  >
    <div class="flex items-center justify-between mb-4">
      <h4 class="font-bold text-slate-900 dark:text-white">{$t.dstPeriods}</h4>
      <button
        onclick={handleExport}
        class="flex items-center gap-2 px-4 py-2 bg-emerald-600 hover:bg-emerald-700 text-white text-sm font-bold rounded-lg transition-colors"
      >
        <Icon name="download" size="sm" />
        {$t.exportToExcel}
      </button>
    </div>

    <div class="overflow-x-auto">
      <table class="w-full">
        <thead>
          <tr class="border-b border-slate-200 dark:border-[#334a5e]">
            <th class="px-4 py-3 text-left text-xs font-bold text-slate-500 uppercase">
              {$t.periodNumber}
            </th>
            <th class="px-4 py-3 text-left text-xs font-bold text-slate-500 uppercase">
              {$t.timeOffset}
            </th>
            <th class="px-4 py-3 text-left text-xs font-bold text-slate-500 uppercase">
              {$t.forwardDate}
            </th>
            <th class="px-4 py-3 text-left text-xs font-bold text-slate-500 uppercase">
              {$t.backwardDate}
            </th>
          </tr>
        </thead>
        <tbody class="divide-y divide-slate-200 dark:divide-[#334a5e]">
          {#each dstPeriods as period}
            <tr>
              <td class="px-4 py-3 text-sm font-bold text-slate-900 dark:text-white">
                {period.id}
              </td>
              <td class="px-4 py-3">
                <input
                  type="text"
                  bind:value={period.offset}
                  placeholder="+01:00"
                  disabled={!$isConnected}
                  class="w-24 bg-white dark:bg-[#1a2632] text-slate-700 dark:text-white border border-slate-300 dark:border-[#334a5e] rounded-lg px-3 py-2 text-sm font-mono focus:border-primary focus:ring-1 focus:ring-primary outline-none disabled:opacity-50 disabled:cursor-not-allowed"
                />
              </td>
              <td class="px-4 py-3">
                <input
                  type="text"
                  bind:value={period.forward}
                  placeholder="YYYY-MM-DD HH:MM"
                  disabled={!$isConnected}
                  class="w-40 bg-white dark:bg-[#1a2632] text-slate-700 dark:text-white border border-slate-300 dark:border-[#334a5e] rounded-lg px-3 py-2 text-sm font-mono focus:border-primary focus:ring-1 focus:ring-primary outline-none disabled:opacity-50 disabled:cursor-not-allowed"
                />
              </td>
              <td class="px-4 py-3">
                <input
                  type="text"
                  bind:value={period.backward}
                  placeholder="YYYY-MM-DD HH:MM"
                  disabled={!$isConnected}
                  class="w-40 bg-white dark:bg-[#1a2632] text-slate-700 dark:text-white border border-slate-300 dark:border-[#334a5e] rounded-lg px-3 py-2 text-sm font-mono focus:border-primary focus:ring-1 focus:ring-primary outline-none disabled:opacity-50 disabled:cursor-not-allowed"
                />
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>

    <div class="mt-6 flex justify-end">
      <button
        disabled={!$isConnected}
        class="flex items-center gap-2 px-6 py-3 bg-primary hover:bg-primary/90 text-white font-bold rounded-lg transition-all disabled:opacity-50 disabled:cursor-not-allowed"
      >
        <Icon name="save" />
        {$t.saveAll}
      </button>
    </div>
  </div>
</div>
