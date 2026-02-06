<script lang="ts">
  import Icon from "$lib/components/common/Icon.svelte";
  import { t, isConnected, meterStore } from "$lib/stores";

  function formatNumber(value: number | undefined, decimals: number = 1): string {
    if (value === undefined || value === null) return "-";
    return value.toLocaleString("tr-TR", {
      minimumFractionDigits: decimals,
      maximumFractionDigits: decimals,
    });
  }

  // Calculate gauge percentage (0-100) for voltage (expecting 180-260V range)
  function voltageToPercent(voltage: number | undefined): number {
    if (!voltage) return 0;
    const min = 180;
    const max = 260;
    const normalized = ((voltage - min) / (max - min)) * 100;
    return Math.max(0, Math.min(100, normalized));
  }

  // Calculate gauge color based on voltage
  function voltageColor(voltage: number | undefined): string {
    if (!voltage) return "#94a3b8";
    if (voltage < 200 || voltage > 250) return "#ef4444"; // Red - danger
    if (voltage < 210 || voltage > 240) return "#f59e0b"; // Amber - warning
    return "#10b981"; // Green - normal
  }

  // Determine power flow direction based on power factor sign (simplified)
  function getDirection(_pf: number | undefined): "import" | "export" {
    // In real implementation this would come from actual direction indicator
    return "import";
  }
</script>

<div class="space-y-6">
  <!-- Header -->
  <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm">
    <div>
      <h3 class="text-xl font-bold text-slate-900 dark:text-white mb-2">{$t.liveMeasurements}</h3>
      <p class="text-sm text-slate-500 dark:text-slate-400">{$t.liveMeasurementsDescription}</p>
    </div>
  </div>

  {#if !$isConnected}
    <div class="bg-amber-500/10 border border-amber-500/20 rounded-xl p-6 text-center">
      <Icon name="warning" class="text-4xl text-amber-500 mb-3" />
      <p class="text-amber-600 dark:text-amber-400 font-medium">{$t.connectFirstWarning}</p>
    </div>
  {:else if $meterStore.shortReadData}
    {@const data = $meterStore.shortReadData}
    {@const isThreePhase = $meterStore.meterType === "three-phase"}

    <!-- Phase Gauges -->
    <div class="grid grid-cols-1 md:grid-cols-3 gap-6">
      <!-- Phase R / L1 -->
      <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm">
        <div class="text-center mb-4">
          <h4 class="font-bold text-slate-900 dark:text-white flex items-center justify-center gap-2">
            <div class="w-3 h-3 rounded-full bg-red-500"></div>
            {isThreePhase ? $t.phaseR : $t.voltageL1}
          </h4>
        </div>

        <!-- Circular Gauge -->
        <div class="relative w-48 h-48 mx-auto mb-4">
          <svg class="w-full h-full transform -rotate-90" viewBox="0 0 100 100">
            <!-- Background circle -->
            <circle
              cx="50" cy="50" r="42"
              fill="none"
              stroke="currentColor"
              class="text-slate-200 dark:text-[#334a5e]"
              stroke-width="8"
            />
            <!-- Value arc -->
            <circle
              cx="50" cy="50" r="42"
              fill="none"
              stroke={voltageColor(data.voltageL1)}
              stroke-width="8"
              stroke-linecap="round"
              stroke-dasharray={`${voltageToPercent(data.voltageL1) * 2.64} 264`}
              class="transition-all duration-500"
            />
          </svg>
          <div class="absolute inset-0 flex flex-col items-center justify-center">
            <span class="text-3xl font-mono font-bold text-slate-900 dark:text-white">
              {formatNumber(data.voltageL1)}
            </span>
            <span class="text-sm text-slate-500">V</span>
          </div>
        </div>

        <!-- Additional values -->
        <div class="grid grid-cols-2 gap-3">
          <div class="p-3 bg-slate-50 dark:bg-[#0f1821] rounded-lg text-center">
            <div class="text-xs text-slate-500 mb-1">{$t.current}</div>
            <div class="text-lg font-mono font-bold text-slate-900 dark:text-white">
              {formatNumber(data.currentL1, 2)} A
            </div>
          </div>
          <div class="p-3 bg-slate-50 dark:bg-[#0f1821] rounded-lg text-center">
            <div class="text-xs text-slate-500 mb-1">{$t.powerFactor}</div>
            <div class="text-lg font-mono font-bold text-slate-900 dark:text-white">
              {formatNumber(data.powerFactorL1, 3)}
            </div>
          </div>
        </div>

        <!-- Direction indicator -->
        <div class="mt-3 flex items-center justify-center gap-2 p-2 bg-emerald-500/10 rounded-lg">
          <Icon name={getDirection(data.powerFactorL1) === "import" ? "arrow_downward" : "arrow_upward"} class="text-emerald-500" size="sm" />
          <span class="text-sm font-medium text-emerald-600 dark:text-emerald-400">
            {getDirection(data.powerFactorL1) === "import" ? $t.energyImporting : $t.energyExporting}
          </span>
        </div>
      </div>

      <!-- Phase S / L2 (only for 3-phase) -->
      {#if isThreePhase}
        <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm">
          <div class="text-center mb-4">
            <h4 class="font-bold text-slate-900 dark:text-white flex items-center justify-center gap-2">
              <div class="w-3 h-3 rounded-full bg-yellow-500"></div>
              {$t.phaseS}
            </h4>
          </div>

          <div class="relative w-48 h-48 mx-auto mb-4">
            <svg class="w-full h-full transform -rotate-90" viewBox="0 0 100 100">
              <circle cx="50" cy="50" r="42" fill="none" stroke="currentColor" class="text-slate-200 dark:text-[#334a5e]" stroke-width="8" />
              <circle cx="50" cy="50" r="42" fill="none" stroke={voltageColor(data.voltageL2)} stroke-width="8" stroke-linecap="round" stroke-dasharray={`${voltageToPercent(data.voltageL2) * 2.64} 264`} class="transition-all duration-500" />
            </svg>
            <div class="absolute inset-0 flex flex-col items-center justify-center">
              <span class="text-3xl font-mono font-bold text-slate-900 dark:text-white">{formatNumber(data.voltageL2)}</span>
              <span class="text-sm text-slate-500">V</span>
            </div>
          </div>

          <div class="grid grid-cols-2 gap-3">
            <div class="p-3 bg-slate-50 dark:bg-[#0f1821] rounded-lg text-center">
              <div class="text-xs text-slate-500 mb-1">{$t.current}</div>
              <div class="text-lg font-mono font-bold text-slate-900 dark:text-white">{formatNumber(data.currentL2, 2)} A</div>
            </div>
            <div class="p-3 bg-slate-50 dark:bg-[#0f1821] rounded-lg text-center">
              <div class="text-xs text-slate-500 mb-1">{$t.powerFactor}</div>
              <div class="text-lg font-mono font-bold text-slate-900 dark:text-white">{formatNumber(data.powerFactorL2, 3)}</div>
            </div>
          </div>

          <div class="mt-3 flex items-center justify-center gap-2 p-2 bg-emerald-500/10 rounded-lg">
            <Icon name="arrow_downward" class="text-emerald-500" size="sm" />
            <span class="text-sm font-medium text-emerald-600 dark:text-emerald-400">{$t.energyImporting}</span>
          </div>
        </div>

        <!-- Phase T / L3 -->
        <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm">
          <div class="text-center mb-4">
            <h4 class="font-bold text-slate-900 dark:text-white flex items-center justify-center gap-2">
              <div class="w-3 h-3 rounded-full bg-blue-500"></div>
              {$t.phaseT}
            </h4>
          </div>

          <div class="relative w-48 h-48 mx-auto mb-4">
            <svg class="w-full h-full transform -rotate-90" viewBox="0 0 100 100">
              <circle cx="50" cy="50" r="42" fill="none" stroke="currentColor" class="text-slate-200 dark:text-[#334a5e]" stroke-width="8" />
              <circle cx="50" cy="50" r="42" fill="none" stroke={voltageColor(data.voltageL3)} stroke-width="8" stroke-linecap="round" stroke-dasharray={`${voltageToPercent(data.voltageL3) * 2.64} 264`} class="transition-all duration-500" />
            </svg>
            <div class="absolute inset-0 flex flex-col items-center justify-center">
              <span class="text-3xl font-mono font-bold text-slate-900 dark:text-white">{formatNumber(data.voltageL3)}</span>
              <span class="text-sm text-slate-500">V</span>
            </div>
          </div>

          <div class="grid grid-cols-2 gap-3">
            <div class="p-3 bg-slate-50 dark:bg-[#0f1821] rounded-lg text-center">
              <div class="text-xs text-slate-500 mb-1">{$t.current}</div>
              <div class="text-lg font-mono font-bold text-slate-900 dark:text-white">{formatNumber(data.currentL3, 2)} A</div>
            </div>
            <div class="p-3 bg-slate-50 dark:bg-[#0f1821] rounded-lg text-center">
              <div class="text-xs text-slate-500 mb-1">{$t.powerFactor}</div>
              <div class="text-lg font-mono font-bold text-slate-900 dark:text-white">{formatNumber(data.powerFactorL3, 3)}</div>
            </div>
          </div>

          <div class="mt-3 flex items-center justify-center gap-2 p-2 bg-emerald-500/10 rounded-lg">
            <Icon name="arrow_downward" class="text-emerald-500" size="sm" />
            <span class="text-sm font-medium text-emerald-600 dark:text-emerald-400">{$t.energyImporting}</span>
          </div>
        </div>
      {:else}
        <!-- Frequency display for single-phase (use remaining 2 columns) -->
        <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm md:col-span-2">
          <h4 class="font-bold text-slate-900 dark:text-white mb-4 flex items-center gap-2">
            <Icon name="speed" class="text-primary" />
            {$t.frequency}
          </h4>
          <div class="flex items-center justify-center py-8">
            <div class="text-6xl font-mono font-bold text-primary">
              {formatNumber(data.frequency, 2)}
              <span class="text-2xl text-slate-500 ml-2">Hz</span>
            </div>
          </div>
        </div>
      {/if}
    </div>

    <!-- Bottom Row: Frequency (for 3-phase) -->
    {#if isThreePhase}
      <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-4">
            <div class="p-4 bg-primary/10 rounded-xl">
              <Icon name="speed" class="text-primary text-3xl" />
            </div>
            <div>
              <h4 class="font-bold text-slate-900 dark:text-white">{$t.frequency}</h4>
              <p class="text-sm text-slate-500">Grid frequency</p>
            </div>
          </div>
          <div class="text-4xl font-mono font-bold text-primary">
            {formatNumber(data.frequency, 2)}
            <span class="text-xl text-slate-500 ml-1">Hz</span>
          </div>
        </div>
      </div>
    {/if}
  {:else}
    <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-12 shadow-sm text-center">
      <Icon name="speed" class="text-6xl text-slate-300 dark:text-slate-600 mb-4" />
      <p class="text-slate-500 dark:text-slate-400">Veri yok - Genel Bakis sayfasindan okuma yapin</p>
    </div>
  {/if}
</div>
