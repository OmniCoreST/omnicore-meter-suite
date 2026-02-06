<script lang="ts">
  import Icon from "$lib/components/common/Icon.svelte";
  import { t, isConnected, meterStore } from "$lib/stores";
  import { exportToExcel } from "$lib/utils/export";

  let activeTab = $state<"import" | "export" | "reactive">("import");

  function formatNumber(value: number | undefined, decimals: number = 3): string {
    if (value === undefined || value === null) return "-";
    return value.toLocaleString("tr-TR", {
      minimumFractionDigits: decimals,
      maximumFractionDigits: decimals,
    });
  }

  const monthNames = ["Ocak", "Subat", "Mart", "Nisan", "Mayis", "Haziran",
                      "Temmuz", "Agustos", "Eylul", "Ekim", "Kasim", "Aralik"];

  // Parse monthly historical data from raw OBIS codes (if available)
  let monthlyData = $derived.by(() => {
    const data = $meterStore.shortReadData;
    // @ts-ignore - rawData may exist on extended data
    if (!data || !data.rawData) return [];

    const months = [];

    for (let month = 1; month <= 12; month++) {
      // @ts-ignore
      const raw = data.rawData;
      const t1Match = raw.match(new RegExp(`1\\.8\\.1\\*${month}\\(([\\d.]+)\\*kWh\\)`));
      const t2Match = raw.match(new RegExp(`1\\.8\\.2\\*${month}\\(([\\d.]+)\\*kWh\\)`));
      const t3Match = raw.match(new RegExp(`1\\.8\\.3\\*${month}\\(([\\d.]+)\\*kWh\\)`));
      const t4Match = raw.match(new RegExp(`1\\.8\\.4\\*${month}\\(([\\d.]+)\\*kWh\\)`));
      const totalMatch = raw.match(new RegExp(`1\\.8\\.0\\*${month}\\(([\\d.]+)\\*kWh\\)`));

      months.push({
        month,
        monthName: monthNames[month - 1],
        t1: t1Match ? parseFloat(t1Match[1]) : 0,
        t2: t2Match ? parseFloat(t2Match[1]) : 0,
        t3: t3Match ? parseFloat(t3Match[1]) : 0,
        t4: t4Match ? parseFloat(t4Match[1]) : 0,
        total: totalMatch ? parseFloat(totalMatch[1]) : 0,
      });
    }

    return months;
  });

  async function handleExport() {
    try {
      const data = $meterStore.shortReadData;
      if (!data) { alert("Veri yok - önce okuma yapın"); return; }

      const exportData = [
        { tariff: $t.total, value: data.activeEnergyImportTotal, unit: "kWh" },
        { tariff: $t.t1Day, value: data.activeEnergyImportT1, unit: "kWh" },
        { tariff: $t.t2Peak, value: data.activeEnergyImportT2, unit: "kWh" },
        { tariff: $t.t3Night, value: data.activeEnergyImportT3, unit: "kWh" },
        { tariff: $t.t4, value: data.activeEnergyImportT4, unit: "kWh" },
      ];

      await exportToExcel(exportData, "energy_data", [
        { key: "tariff", label: "Tarife" },
        { key: "value", label: "Deger" },
        { key: "unit", label: "Birim" },
      ]);
    } catch (e) { alert("Export hatası: " + e); }
  }
</script>

<div class="space-y-6">
  <!-- Header -->
  <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm">
    <div class="flex items-start justify-between">
      <div>
        <h3 class="text-xl font-bold text-slate-900 dark:text-white mb-2">{$t.energy}</h3>
        <p class="text-sm text-slate-500 dark:text-slate-400">{$t.energyDescription}</p>
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
    {@const isKombi = $meterStore.meterType === "kombi"}

    <!-- Tab Selection (only if bidirectional or kombi) -->
    {#if isBidirectional || isKombi}
      <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-2 shadow-sm">
        <div class="flex gap-2">
          <button
            onclick={() => activeTab = "import"}
            class="flex-1 px-4 py-3 rounded-lg font-bold transition-colors
              {activeTab === 'import' ? 'bg-primary text-white' : 'text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-[#334a5e]'}"
          >
            <Icon name="arrow_downward" size="sm" class="inline mr-2" />
            {$t.activeEnergyImport}
          </button>
          {#if isBidirectional}
            <button
              onclick={() => activeTab = "export"}
              class="flex-1 px-4 py-3 rounded-lg font-bold transition-colors
                {activeTab === 'export' ? 'bg-primary text-white' : 'text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-[#334a5e]'}"
            >
              <Icon name="arrow_upward" size="sm" class="inline mr-2" />
              {$t.activeEnergyExport}
            </button>
          {/if}
          {#if isKombi}
            <button
              onclick={() => activeTab = "reactive"}
              class="flex-1 px-4 py-3 rounded-lg font-bold transition-colors
                {activeTab === 'reactive' ? 'bg-primary text-white' : 'text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-[#334a5e]'}"
            >
              <Icon name="sync_alt" size="sm" class="inline mr-2" />
              {$t.reactiveEnergy}
            </button>
          {/if}
        </div>
      </div>
    {/if}

    <!-- Current Energy Section -->
    <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm">
      <h4 class="font-bold text-slate-900 dark:text-white mb-6 flex items-center gap-2">
        <Icon name="bolt" class="text-primary" />
        {$t.currentEnergy}
      </h4>

      {#if activeTab === "import"}
        <!-- Active Energy Import -->
        <div class="grid grid-cols-2 md:grid-cols-5 gap-4">
          <!-- Total -->
          <div class="col-span-2 md:col-span-1 p-6 bg-gradient-to-br from-primary/10 to-emerald-500/10 rounded-xl border border-primary/20">
            <div class="text-xs text-slate-500 dark:text-slate-400 uppercase tracking-wider mb-2">{$t.total}</div>
            <div class="text-3xl font-mono font-bold text-primary">{formatNumber(data.activeEnergyImportTotal)}</div>
            <div class="text-sm text-slate-500 mt-1">kWh</div>
          </div>

          <!-- T1 -->
          <div class="p-4 bg-blue-500/10 rounded-xl border border-blue-500/20">
            <div class="text-xs text-slate-500 dark:text-slate-400 uppercase tracking-wider mb-2">{$t.t1Day}</div>
            <div class="text-xl font-mono font-bold text-blue-600 dark:text-blue-400">{formatNumber(data.activeEnergyImportT1)}</div>
            <div class="text-xs text-slate-500 mt-1">kWh</div>
          </div>

          <!-- T2 -->
          <div class="p-4 bg-red-500/10 rounded-xl border border-red-500/20">
            <div class="text-xs text-slate-500 dark:text-slate-400 uppercase tracking-wider mb-2">{$t.t2Peak}</div>
            <div class="text-xl font-mono font-bold text-red-600 dark:text-red-400">{formatNumber(data.activeEnergyImportT2)}</div>
            <div class="text-xs text-slate-500 mt-1">kWh</div>
          </div>

          <!-- T3 -->
          <div class="p-4 bg-emerald-500/10 rounded-xl border border-emerald-500/20">
            <div class="text-xs text-slate-500 dark:text-slate-400 uppercase tracking-wider mb-2">{$t.t3Night}</div>
            <div class="text-xl font-mono font-bold text-emerald-600 dark:text-emerald-400">{formatNumber(data.activeEnergyImportT3)}</div>
            <div class="text-xs text-slate-500 mt-1">kWh</div>
          </div>

          <!-- T4 -->
          <div class="p-4 bg-amber-500/10 rounded-xl border border-amber-500/20">
            <div class="text-xs text-slate-500 dark:text-slate-400 uppercase tracking-wider mb-2">{$t.t4}</div>
            <div class="text-xl font-mono font-bold text-amber-600 dark:text-amber-400">{formatNumber(data.activeEnergyImportT4)}</div>
            <div class="text-xs text-slate-500 mt-1">kWh</div>
          </div>
        </div>
      {:else if activeTab === "export"}
        <!-- Active Energy Export -->
        <div class="grid grid-cols-2 md:grid-cols-5 gap-4">
          <div class="col-span-2 md:col-span-1 p-6 bg-gradient-to-br from-violet-500/10 to-purple-500/10 rounded-xl border border-violet-500/20">
            <div class="text-xs text-slate-500 dark:text-slate-400 uppercase tracking-wider mb-2">{$t.total}</div>
            <div class="text-3xl font-mono font-bold text-violet-600">{formatNumber(data.activeEnergyExportTotal)}</div>
            <div class="text-sm text-slate-500 mt-1">kWh</div>
          </div>

          <div class="p-4 bg-blue-500/10 rounded-xl border border-blue-500/20">
            <div class="text-xs text-slate-500 dark:text-slate-400 uppercase tracking-wider mb-2">{$t.t1Day}</div>
            <div class="text-xl font-mono font-bold text-blue-600 dark:text-blue-400">{formatNumber(data.activeEnergyExportT1)}</div>
            <div class="text-xs text-slate-500 mt-1">kWh</div>
          </div>

          <div class="p-4 bg-red-500/10 rounded-xl border border-red-500/20">
            <div class="text-xs text-slate-500 dark:text-slate-400 uppercase tracking-wider mb-2">{$t.t2Peak}</div>
            <div class="text-xl font-mono font-bold text-red-600 dark:text-red-400">{formatNumber(data.activeEnergyExportT2)}</div>
            <div class="text-xs text-slate-500 mt-1">kWh</div>
          </div>

          <div class="p-4 bg-emerald-500/10 rounded-xl border border-emerald-500/20">
            <div class="text-xs text-slate-500 dark:text-slate-400 uppercase tracking-wider mb-2">{$t.t3Night}</div>
            <div class="text-xl font-mono font-bold text-emerald-600 dark:text-emerald-400">{formatNumber(data.activeEnergyExportT3)}</div>
            <div class="text-xs text-slate-500 mt-1">kWh</div>
          </div>

          <div class="p-4 bg-amber-500/10 rounded-xl border border-amber-500/20">
            <div class="text-xs text-slate-500 dark:text-slate-400 uppercase tracking-wider mb-2">{$t.t4}</div>
            <div class="text-xl font-mono font-bold text-amber-600 dark:text-amber-400">{formatNumber(data.activeEnergyExportT4)}</div>
            <div class="text-xs text-slate-500 mt-1">kWh</div>
          </div>
        </div>
      {:else}
        <!-- Reactive Energy -->
        <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
          <div class="p-4 bg-indigo-500/10 rounded-xl border border-indigo-500/20">
            <div class="text-xs text-slate-500 dark:text-slate-400 uppercase tracking-wider mb-2">{$t.inductiveImport}</div>
            <div class="text-xl font-mono font-bold text-indigo-600 dark:text-indigo-400">{formatNumber(data.reactiveEnergyInductiveImport)}</div>
            <div class="text-xs text-slate-500 mt-1">kVArh</div>
          </div>

          <div class="p-4 bg-pink-500/10 rounded-xl border border-pink-500/20">
            <div class="text-xs text-slate-500 dark:text-slate-400 uppercase tracking-wider mb-2">{$t.capacitiveImport}</div>
            <div class="text-xl font-mono font-bold text-pink-600 dark:text-pink-400">{formatNumber(data.reactiveEnergyCapacitiveImport)}</div>
            <div class="text-xs text-slate-500 mt-1">kVArh</div>
          </div>

          <div class="p-4 bg-cyan-500/10 rounded-xl border border-cyan-500/20">
            <div class="text-xs text-slate-500 dark:text-slate-400 uppercase tracking-wider mb-2">{$t.inductiveExport}</div>
            <div class="text-xl font-mono font-bold text-cyan-600 dark:text-cyan-400">{formatNumber(data.reactiveEnergyInductiveExport)}</div>
            <div class="text-xs text-slate-500 mt-1">kVArh</div>
          </div>

          <div class="p-4 bg-teal-500/10 rounded-xl border border-teal-500/20">
            <div class="text-xs text-slate-500 dark:text-slate-400 uppercase tracking-wider mb-2">{$t.capacitiveExport}</div>
            <div class="text-xl font-mono font-bold text-teal-600 dark:text-teal-400">{formatNumber(data.reactiveEnergyCapacitiveExport)}</div>
            <div class="text-xs text-slate-500 mt-1">kVArh</div>
          </div>
        </div>
      {/if}
    </div>

    <!-- 12-Month History Section -->
    <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm">
      <h4 class="font-bold text-slate-900 dark:text-white mb-6 flex items-center gap-2">
        <Icon name="calendar_month" class="text-primary" />
        {$t.monthlyHistory}
      </h4>

      {#if monthlyData.length > 0 && monthlyData.some(m => m.total > 0)}
        <div class="overflow-x-auto">
          <table class="w-full text-sm">
            <thead class="bg-slate-50 dark:bg-[#0f1821]">
              <tr class="border-b border-slate-200 dark:border-[#334a5e]">
                <th class="px-4 py-3 text-left font-bold text-slate-700 dark:text-slate-300">{$t.month}</th>
                <th class="px-4 py-3 text-right font-bold text-blue-600">{$t.t1Day}</th>
                <th class="px-4 py-3 text-right font-bold text-red-600">{$t.t2Peak}</th>
                <th class="px-4 py-3 text-right font-bold text-emerald-600">{$t.t3Night}</th>
                <th class="px-4 py-3 text-right font-bold text-amber-600">{$t.t4}</th>
                <th class="px-4 py-3 text-right font-bold text-primary">{$t.total}</th>
              </tr>
            </thead>
            <tbody>
              {#each monthlyData as monthData}
                <tr class="border-b border-slate-100 dark:border-[#334a5e]/30 hover:bg-slate-50 dark:hover:bg-[#1a2632] transition-colors">
                  <td class="px-4 py-3">
                    <div class="flex items-center gap-2">
                      <div class="size-2 rounded-full bg-primary"></div>
                      <span class="font-medium text-slate-900 dark:text-white">{monthData.monthName}</span>
                    </div>
                  </td>
                  <td class="px-4 py-3 text-right font-mono text-slate-600 dark:text-slate-400">{formatNumber(monthData.t1)}</td>
                  <td class="px-4 py-3 text-right font-mono text-slate-600 dark:text-slate-400">{formatNumber(monthData.t2)}</td>
                  <td class="px-4 py-3 text-right font-mono text-slate-600 dark:text-slate-400">{formatNumber(monthData.t3)}</td>
                  <td class="px-4 py-3 text-right font-mono text-slate-600 dark:text-slate-400">{formatNumber(monthData.t4)}</td>
                  <td class="px-4 py-3 text-right font-mono font-bold text-primary">{formatNumber(monthData.total)}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {:else}
        <div class="text-center py-8 text-slate-400">
          <Icon name="history" class="text-4xl mb-2" />
          <p class="text-sm">{$t.noHistoryData}</p>
        </div>
      {/if}
    </div>
  {:else}
    <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-12 shadow-sm text-center">
      <Icon name="bolt" class="text-6xl text-slate-300 dark:text-slate-600 mb-4" />
      <p class="text-slate-500 dark:text-slate-400">Veri yok - Genel Bakis sayfasindan okuma yapin</p>
    </div>
  {/if}
</div>
