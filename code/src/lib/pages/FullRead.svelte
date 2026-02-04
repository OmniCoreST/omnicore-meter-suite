<script lang="ts">
  import Icon from "$lib/components/common/Icon.svelte";
  import { t, meterStore } from "$lib/stores";

  let activeTab = $state("current");

  const tabs = [
    { id: "current", labelKey: "currentData", icon: "electric_meter" },
    { id: "historical", labelKey: "historicalData", icon: "history" },
    { id: "warnings", labelKey: "warnings", icon: "warning" },
    { id: "outages", labelKey: "outageRecords", icon: "power_off" },
  ];

  function formatNumber(value: number | undefined, decimals: number = 3): string {
    if (value === undefined) return "-";
    return value.toLocaleString("tr-TR", {
      minimumFractionDigits: decimals,
      maximumFractionDigits: decimals,
    });
  }

  // Parse monthly historical data from raw OBIS codes
  let monthlyData = $derived.by(() => {
    const data = $meterStore.shortReadData;
    if (!data || !data.rawData) return [];

    const months = [];
    const monthNames = ["Ocak", "Şubat", "Mart", "Nisan", "Mayıs", "Haziran",
                        "Temmuz", "Ağustos", "Eylül", "Ekim", "Kasım", "Aralık"];

    // Parse monthly registers (1.8.x*1 through 1.8.x*12)
    for (let month = 1; month <= 12; month++) {
      const t1Match = data.rawData.match(new RegExp(`1\\.8\\.1\\*${month}\\(([\\d.]+)\\*kWh\\)`));
      const t2Match = data.rawData.match(new RegExp(`1\\.8\\.2\\*${month}\\(([\\d.]+)\\*kWh\\)`));
      const t3Match = data.rawData.match(new RegExp(`1\\.8\\.3\\*${month}\\(([\\d.]+)\\*kWh\\)`));
      const t4Match = data.rawData.match(new RegExp(`1\\.8\\.4\\*${month}\\(([\\d.]+)\\*kWh\\)`));
      const totalMatch = data.rawData.match(new RegExp(`1\\.8\\.0\\*${month}\\(([\\d.]+)\\*kWh\\)`));

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
</script>

<div class="space-y-6">
  <!-- Header -->
  <div
    class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm"
  >
    <div class="flex items-start justify-between">
      <div>
        <h3 class="text-xl font-bold text-slate-900 dark:text-white mb-2">{$t.fullRead}</h3>
        <p class="text-sm text-slate-500 dark:text-slate-400">{$t.fullReadDescription}</p>
      </div>
    </div>
  </div>

  {#if $meterStore.shortReadData}
    {@const data = $meterStore.shortReadData}

    <!-- Tabs -->
    <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl shadow-sm overflow-hidden">
      <div class="flex border-b border-slate-200 dark:border-[#334a5e]">
        {#each tabs as tab}
          <button
            onclick={() => activeTab = tab.id}
            class="flex-1 flex items-center justify-center gap-2 px-6 py-4 font-medium transition-colors
              {activeTab === tab.id
                ? 'bg-primary/10 text-primary border-b-2 border-primary'
                : 'text-slate-600 dark:text-slate-400 hover:bg-slate-50 dark:hover:bg-[#1a2632]'}"
          >
            <Icon name={tab.icon} size="sm" />
            <span>{$t[tab.labelKey]}</span>
          </button>
        {/each}
      </div>

      <div class="p-6">
        {#if activeTab === "current"}
          <!-- Current Data Tab - Same as Short Read -->
          <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
            <!-- Show all current meter values -->
            <div class="space-y-4">
              <h4 class="font-bold text-slate-900 dark:text-white flex items-center gap-2">
                <Icon name="badge" class="text-primary" size="sm" />
                {$t.meterIdentity}
              </h4>
              <div class="space-y-2 text-sm">
                <div class="flex justify-between">
                  <span class="text-slate-500">{$t.serialNumber}</span>
                  <span class="font-mono font-bold">{data.serialNumber || "510095849"}</span>
                </div>
                <div class="flex justify-between">
                  <span class="text-slate-500">{$t.programVersion}</span>
                  <span class="font-mono">{data.programVersion}</span>
                </div>
                <div class="flex justify-between">
                  <span class="text-slate-500">{$t.meterDate}</span>
                  <span class="font-mono">{data.meterDate} {data.meterTime}</span>
                </div>
              </div>
            </div>

            <div class="space-y-4">
              <h4 class="font-bold text-slate-900 dark:text-white flex items-center gap-2">
                <Icon name="bolt" class="text-primary" size="sm" />
                {$t.activeEnergyImport}
              </h4>
              <div class="space-y-2 text-sm">
                <div class="flex justify-between">
                  <span class="text-slate-500">{$t.total}</span>
                  <span class="font-mono font-bold">{formatNumber(data.activeEnergyImportTotal)} kWh</span>
                </div>
                <div class="flex justify-between">
                  <span class="text-slate-500">{$t.t1Day}</span>
                  <span class="font-mono">{formatNumber(data.activeEnergyImportT1)} kWh</span>
                </div>
                <div class="flex justify-between">
                  <span class="text-slate-500">{$t.t2Peak}</span>
                  <span class="font-mono">{formatNumber(data.activeEnergyImportT2)} kWh</span>
                </div>
                <div class="flex justify-between">
                  <span class="text-slate-500">{$t.t3Night}</span>
                  <span class="font-mono">{formatNumber(data.activeEnergyImportT3)} kWh</span>
                </div>
              </div>
            </div>

            <div class="space-y-4">
              <h4 class="font-bold text-slate-900 dark:text-white flex items-center gap-2">
                <Icon name="electric_bolt" class="text-primary" size="sm" />
                {$t.instantaneousValues}
              </h4>
              <div class="space-y-2 text-sm">
                <div class="flex justify-between">
                  <span class="text-slate-500">{$t.voltageL1}</span>
                  <span class="font-mono font-bold">{formatNumber(data.voltageL1, 1)} V</span>
                </div>
                <div class="flex justify-between">
                  <span class="text-slate-500">{$t.frequency}</span>
                  <span class="font-mono">{formatNumber(data.frequency, 1)} Hz</span>
                </div>
              </div>
            </div>
          </div>
        {:else if activeTab === "historical"}
          <!-- Historical Data Tab -->
          <div class="space-y-6">
            <div class="flex items-center justify-between">
              <h4 class="font-bold text-slate-900 dark:text-white flex items-center gap-2">
                <Icon name="calendar_month" class="text-primary" />
                12 Aylık Enerji Geçmişi
              </h4>
              <span class="text-sm text-slate-500">Son 12 ay</span>
            </div>

            <!-- Monthly Data Table -->
            <div class="overflow-x-auto">
              <table class="w-full text-sm">
                <thead class="bg-slate-50 dark:bg-[#111c26]">
                  <tr class="border-b border-slate-200 dark:border-[#334a5e]">
                    <th class="px-4 py-3 text-left font-bold text-slate-700 dark:text-slate-300">Ay</th>
                    <th class="px-4 py-3 text-right font-bold text-slate-700 dark:text-slate-300">T1 Gündüz</th>
                    <th class="px-4 py-3 text-right font-bold text-slate-700 dark:text-slate-300">T2 Puant</th>
                    <th class="px-4 py-3 text-right font-bold text-slate-700 dark:text-slate-300">T3 Gece</th>
                    <th class="px-4 py-3 text-right font-bold text-slate-700 dark:text-slate-300">T4</th>
                    <th class="px-4 py-3 text-right font-bold text-primary">Toplam</th>
                  </tr>
                </thead>
                <tbody>
                  {#each monthlyData as monthData, i}
                    <tr class="border-b border-slate-100 dark:border-[#334a5e]/30 hover:bg-slate-50 dark:hover:bg-[#1a2632] transition-colors">
                      <td class="px-4 py-3">
                        <div class="flex items-center gap-2">
                          <div class="size-2 rounded-full bg-primary"></div>
                          <span class="font-medium text-slate-900 dark:text-white">{monthData.monthName}</span>
                        </div>
                      </td>
                      <td class="px-4 py-3 text-right font-mono text-slate-600 dark:text-slate-400">
                        {formatNumber(monthData.t1)} kWh
                      </td>
                      <td class="px-4 py-3 text-right font-mono text-slate-600 dark:text-slate-400">
                        {formatNumber(monthData.t2)} kWh
                      </td>
                      <td class="px-4 py-3 text-right font-mono text-slate-600 dark:text-slate-400">
                        {formatNumber(monthData.t3)} kWh
                      </td>
                      <td class="px-4 py-3 text-right font-mono text-slate-600 dark:text-slate-400">
                        {formatNumber(monthData.t4)} kWh
                      </td>
                      <td class="px-4 py-3 text-right font-mono font-bold text-primary">
                        {formatNumber(monthData.total)} kWh
                      </td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>

            <!-- Summary Cards -->
            <div class="grid grid-cols-1 md:grid-cols-4 gap-4 mt-6">
              {#each ["T1", "T2", "T3", "Total"] as tariff, i}
                {@const totalKey = i === 0 ? 't1' : i === 1 ? 't2' : i === 2 ? 't3' : 'total'}
                {@const sum = monthlyData.reduce((acc, m) => acc + m[totalKey], 0)}
                <div class="bg-gradient-to-br from-primary/10 to-emerald-500/10 dark:from-primary/20 dark:to-emerald-500/20 rounded-xl p-4 border border-primary/20">
                  <div class="text-xs font-bold text-slate-500 dark:text-slate-400 uppercase tracking-wider mb-1">
                    {tariff === "Total" ? "Yıllık Toplam" : `${tariff} Toplamı`}
                  </div>
                  <div class="text-2xl font-bold text-primary">
                    {formatNumber(sum, 0)}
                  </div>
                  <div class="text-xs text-slate-600 dark:text-slate-400 mt-1">kWh</div>
                </div>
              {/each}
            </div>
          </div>
        {:else if activeTab === "warnings"}
          <div class="text-center py-12 text-slate-500">
            <Icon name="warning" class="text-6xl text-slate-300 dark:text-slate-600 mb-4" />
            <p>{$t.warningsComingSoon || "Voltage, current, and magnetic field warnings"}</p>
          </div>
        {:else if activeTab === "outages"}
          <div class="text-center py-12 text-slate-500">
            <Icon name="power_off" class="text-6xl text-slate-300 dark:text-slate-600 mb-4" />
            <p>{$t.outagesComingSoon || "Outage records from 96.7.x codes"}</p>
          </div>
        {/if}
      </div>
    </div>
  {:else}
    <div
      class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-12 shadow-sm text-center"
    >
      <Icon name="assignment" class="text-6xl text-slate-300 dark:text-slate-600 mb-4" />
      <p class="text-slate-500 dark:text-slate-400">
        {$t.noDataYet || "Connect to a meter to see full reading data"}
      </p>
    </div>
  {/if}
</div>
