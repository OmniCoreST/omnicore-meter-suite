<script lang="ts">
  import Icon from "$lib/components/common/Icon.svelte";
  import { t, isConnected, meterStore } from "$lib/stores";
  import { exportToExcel } from "$lib/utils/export";

  function formatNumber(value: number | undefined, decimals: number = 3): string {
    if (value === undefined || value === null) return "-";
    return value.toLocaleString("tr-TR", {
      minimumFractionDigits: decimals,
      maximumFractionDigits: decimals,
    });
  }

  const monthNames = ["Ocak", "Subat", "Mart", "Nisan", "Mayis", "Haziran",
                      "Temmuz", "Agustos", "Eylul", "Ekim", "Kasim", "Aralik"];

  // Parse demand reset records from raw OBIS codes
  let demandResets = $derived.by(() => {
    const data = $meterStore.shortReadData;
    // @ts-ignore - rawData may exist on extended data
    if (!data || !data.rawData) return [];

    const resets = [];
    // @ts-ignore
    const raw = data.rawData;

    for (let i = 1; i <= 12; i++) {
      // 0.1.2*n - Demand reset timestamp
      const resetMatch = raw.match(new RegExp(`0\\.1\\.2\\*${i}\\(([^)]+)\\)`));
      // 1.6.0*n - Max demand value at reset
      const demandMatch = raw.match(new RegExp(`1\\.6\\.0\\*${i}\\(([\\d.]+)\\*kW\\)`));

      if (resetMatch) {
        resets.push({
          month: i,
          monthName: monthNames[i - 1],
          resetDate: resetMatch[1],
          maxDemand: demandMatch ? parseFloat(demandMatch[1]) : 0,
        });
      }
    }

    return resets;
  });

  // Parse monthly max demand history
  let monthlyDemand = $derived.by(() => {
    const data = $meterStore.shortReadData;
    // @ts-ignore
    if (!data || !data.rawData) return [];

    const months = [];
    // @ts-ignore
    const raw = data.rawData;

    for (let month = 1; month <= 12; month++) {
      const demandMatch = raw.match(new RegExp(`1\\.6\\.0\\*${month}\\(([\\d.]+)\\*kW\\)`));
      const timestampMatch = raw.match(new RegExp(`1\\.6\\.0\\*${month}\\([\\d.]+\\*kW\\)\\(([^)]+)\\)`));

      months.push({
        month,
        monthName: monthNames[month - 1],
        maxDemand: demandMatch ? parseFloat(demandMatch[1]) : 0,
        timestamp: timestampMatch ? timestampMatch[1] : "-",
      });
    }

    return months;
  });

  function handleExport() {
    const exportData = monthlyDemand.map(m => ({
      month: m.monthName,
      maxDemand: m.maxDemand,
      timestamp: m.timestamp,
    }));

    exportToExcel(exportData, "demand_history", [
      { key: "month", label: $t.month },
      { key: "maxDemand", label: `${$t.maxDemand} (kW)` },
      { key: "timestamp", label: $t.dateTime },
    ]);
  }
</script>

<div class="space-y-6">
  <!-- Header -->
  <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm">
    <div class="flex items-start justify-between">
      <div>
        <h3 class="text-xl font-bold text-slate-900 dark:text-white mb-2">{$t.demand}</h3>
        <p class="text-sm text-slate-500 dark:text-slate-400">{$t.demandDescription}</p>
      </div>
      {#if $meterStore.shortReadData}
        <button
          onclick={handleExport}
          class="flex items-center gap-2 px-4 py-2 bg-emerald-600 hover:bg-emerald-700 text-white text-sm font-bold rounded-lg transition-colors"
        >
          <Icon name="download" size="sm" />
          {$t.exportToExcel}
        </button>
      {/if}
    </div>
  </div>

  {#if !$isConnected}
    <div class="bg-amber-500/10 border border-amber-500/20 rounded-xl p-6 text-center">
      <Icon name="warning" class="text-4xl text-amber-500 mb-3" />
      <p class="text-amber-600 dark:text-amber-400 font-medium">{$t.connectFirstWarning}</p>
    </div>
  {:else if $meterStore.shortReadData}
    {@const data = $meterStore.shortReadData}
    {@const isBidirectional = $meterStore.isBidirectional}

    <!-- Current Max Demand Cards -->
    <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
      <!-- Import Demand -->
      <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm">
        <h4 class="font-bold text-slate-900 dark:text-white mb-4 flex items-center gap-2">
          <Icon name="trending_up" class="text-primary" />
          {$t.maxActivePowerImport}
        </h4>

        <div class="p-6 bg-gradient-to-br from-primary/10 to-emerald-500/10 rounded-xl border border-primary/20 mb-4">
          <div class="text-4xl font-mono font-bold text-primary">
            {formatNumber(data.maxDemandImport, 2)}
            <span class="text-xl text-slate-500 ml-1">kW</span>
          </div>
        </div>

        <div class="flex items-center gap-2 text-sm text-slate-500">
          <Icon name="schedule" size="sm" />
          <span>{data.maxDemandImportTimestamp || "-"}</span>
        </div>
      </div>

      <!-- Export Demand (only if bidirectional) -->
      {#if isBidirectional}
        <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm">
          <h4 class="font-bold text-slate-900 dark:text-white mb-4 flex items-center gap-2">
            <Icon name="trending_down" class="text-violet-500" />
            {$t.maxActivePowerExport}
          </h4>

          <div class="p-6 bg-gradient-to-br from-violet-500/10 to-purple-500/10 rounded-xl border border-violet-500/20 mb-4">
            <div class="text-4xl font-mono font-bold text-violet-600">
              {formatNumber(data.maxDemandExport, 2)}
              <span class="text-xl text-slate-500 ml-1">kW</span>
            </div>
          </div>

          <div class="flex items-center gap-2 text-sm text-slate-500">
            <Icon name="schedule" size="sm" />
            <span>{data.maxDemandExportTimestamp || "-"}</span>
          </div>
        </div>
      {:else}
        <!-- Placeholder for single column layout -->
        <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm">
          <h4 class="font-bold text-slate-900 dark:text-white mb-4 flex items-center gap-2">
            <Icon name="info" class="text-primary" />
            {$t.demandPeriod}
          </h4>

          <div class="flex items-center gap-4 p-4 bg-slate-50 dark:bg-[#0f1821] rounded-lg">
            <Icon name="timer" class="text-primary text-2xl" />
            <div>
              <div class="text-2xl font-mono font-bold text-slate-900 dark:text-white">15</div>
              <div class="text-sm text-slate-500">{$t.minutes}</div>
            </div>
          </div>
        </div>
      {/if}
    </div>

    <!-- 12-Month Demand History -->
    <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm">
      <h4 class="font-bold text-slate-900 dark:text-white mb-6 flex items-center gap-2">
        <Icon name="bar_chart" class="text-primary" />
        {$t.demandHistory}
      </h4>

      {#if monthlyDemand.length > 0 && monthlyDemand.some(m => m.maxDemand > 0)}
        <!-- Bar Chart Visualization -->
        <div class="mb-6 p-4 bg-slate-50 dark:bg-[#0f1821] rounded-xl">
          <div class="flex items-end justify-between gap-2 h-40">
            {#each monthlyDemand as monthData}
              {@const maxVal = Math.max(...monthlyDemand.map(m => m.maxDemand)) || 1}
              {@const height = (monthData.maxDemand / maxVal) * 100}
              <div class="flex-1 flex flex-col items-center">
                <div
                  class="w-full bg-gradient-to-t from-primary to-emerald-400 rounded-t-lg transition-all duration-500"
                  style="height: {height}%"
                  title="{monthData.monthName}: {formatNumber(monthData.maxDemand, 2)} kW"
                ></div>
                <div class="text-[10px] text-slate-500 mt-2 truncate w-full text-center">
                  {monthData.monthName.substring(0, 3)}
                </div>
              </div>
            {/each}
          </div>
        </div>

        <!-- Data Table -->
        <div class="overflow-x-auto">
          <table class="w-full text-sm">
            <thead class="bg-slate-50 dark:bg-[#0f1821]">
              <tr class="border-b border-slate-200 dark:border-[#334a5e]">
                <th class="px-4 py-3 text-left font-bold text-slate-700 dark:text-slate-300">{$t.month}</th>
                <th class="px-4 py-3 text-right font-bold text-primary">{$t.maxDemand}</th>
                <th class="px-4 py-3 text-right font-bold text-slate-700 dark:text-slate-300">{$t.dateTime}</th>
              </tr>
            </thead>
            <tbody>
              {#each monthlyDemand as monthData}
                <tr class="border-b border-slate-100 dark:border-[#334a5e]/30 hover:bg-slate-50 dark:hover:bg-[#1a2632] transition-colors">
                  <td class="px-4 py-3">
                    <div class="flex items-center gap-2">
                      <div class="size-2 rounded-full bg-primary"></div>
                      <span class="font-medium text-slate-900 dark:text-white">{monthData.monthName}</span>
                    </div>
                  </td>
                  <td class="px-4 py-3 text-right font-mono font-bold text-primary">
                    {formatNumber(monthData.maxDemand, 2)} kW
                  </td>
                  <td class="px-4 py-3 text-right font-mono text-slate-600 dark:text-slate-400">
                    {monthData.timestamp}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {:else}
        <div class="text-center py-8 text-slate-400">
          <Icon name="trending_up" class="text-4xl mb-2" />
          <p class="text-sm">{$t.noHistoryData}</p>
        </div>
      {/if}
    </div>

    <!-- Demand Reset Records -->
    <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm">
      <h4 class="font-bold text-slate-900 dark:text-white mb-6 flex items-center gap-2">
        <Icon name="restart_alt" class="text-amber-500" />
        {$t.demandResets}
      </h4>

      {#if demandResets.length > 0}
        <div class="overflow-x-auto">
          <table class="w-full text-sm">
            <thead class="bg-slate-50 dark:bg-[#0f1821]">
              <tr class="border-b border-slate-200 dark:border-[#334a5e]">
                <th class="px-4 py-3 text-left font-bold text-slate-700 dark:text-slate-300">#</th>
                <th class="px-4 py-3 text-left font-bold text-slate-700 dark:text-slate-300">{$t.resetDate}</th>
                <th class="px-4 py-3 text-right font-bold text-slate-700 dark:text-slate-300">{$t.valueBeforeReset}</th>
              </tr>
            </thead>
            <tbody>
              {#each demandResets as reset, i}
                <tr class="border-b border-slate-100 dark:border-[#334a5e]/30 hover:bg-slate-50 dark:hover:bg-[#1a2632] transition-colors">
                  <td class="px-4 py-3 font-mono text-slate-600 dark:text-slate-400">{i + 1}</td>
                  <td class="px-4 py-3 font-mono text-slate-900 dark:text-white">{reset.resetDate}</td>
                  <td class="px-4 py-3 text-right font-mono font-bold text-amber-600">{formatNumber(reset.maxDemand, 2)} kW</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {:else}
        <div class="text-center py-8 text-slate-400">
          <Icon name="restart_alt" class="text-4xl mb-2" />
          <p class="text-sm">Demant sifirlama kaydi yok</p>
        </div>
      {/if}
    </div>
  {:else}
    <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-12 shadow-sm text-center">
      <Icon name="trending_up" class="text-6xl text-slate-300 dark:text-slate-600 mb-4" />
      <p class="text-slate-500 dark:text-slate-400">Veri yok - Genel Bakis sayfasindan okuma yapin</p>
    </div>
  {/if}
</div>
