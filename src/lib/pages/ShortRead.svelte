<script lang="ts">
  import Icon from "$lib/components/common/Icon.svelte";
  import { t, isConnected, progressStore, meterStore, addLog } from "$lib/stores";
  import { exportToExcel } from "$lib/utils/export";

  let isReading = $state(false);
  let readComplete = $state(false);

  const shortReadSteps = [
    { id: "open-port", label: "openingSerialPort" },
    { id: "handshake", label: "sendingHandshake" },
    { id: "identify", label: "identifyingDevice" },
    { id: "baud-switch", label: "switchingBaudRate" },
    { id: "request-packet", label: "requestingShortPacket" },
    { id: "receive-data", label: "receivingData" },
    { id: "parse-data", label: "parsingData" },
    { id: "complete", label: "completed" },
  ];

  async function startShortRead() {
    if (!$isConnected) return;

    isReading = true;
    readComplete = false;

    const steps = shortReadSteps.map((s) => ({
      id: s.id,
      label: $t[s.label as keyof typeof $t] as string,
    }));

    progressStore.start($t.shortRead, steps);

    // Simulate reading process
    for (let i = 0; i < steps.length; i++) {
      progressStore.nextStep();
      addLog("info", steps[i].label);
      await new Promise((r) => setTimeout(r, 500 + Math.random() * 500));
    }

    progressStore.complete();

    // Set mock data
    meterStore.setShortReadData(
      {
        serialNumber: "123456789",
        programVersion: "V01.00",
        productionDate: "2024-06-30",
        calibrationDate: "2024-06-30",
        meterDate: "2024-12-15",
        meterTime: "14:30:35",
        dayOfWeek: 4,
        activeEnergyImportTotal: 123456.789,
        activeEnergyImportT1: 45678.123,
        activeEnergyImportT2: 34567.234,
        activeEnergyImportT3: 43211.432,
        activeEnergyImportT4: 0,
        maxDemandImport: 123.456,
        maxDemandImportTimestamp: "2024-02-01 13:30",
        voltageL1: 220.5,
        voltageL2: 221.3,
        voltageL3: 219.8,
        currentL1: 16.5,
        currentL2: 15.8,
        currentL3: 17.2,
        frequency: 49.9,
        powerFactorL1: 0.97,
        powerFactorL2: 0.96,
        powerFactorL3: 0.98,
        ffCode: "0000000000000000",
        gfCode: "0000000000000000",
        batteryStatus: "full",
        relayStatus: "active",
      },
      "three-phase",
      false
    );

    isReading = false;
    readComplete = true;
  }

  function formatNumber(value: number | undefined, decimals: number = 3): string {
    if (value === undefined) return "-";
    return value.toLocaleString("tr-TR", {
      minimumFractionDigits: decimals,
      maximumFractionDigits: decimals,
    });
  }

  function handleExport() {
    const data = $meterStore.shortReadData;
    if (!data) return;

    const exportData = [
      { parameter: $t.serialNumber, value: data.serialNumber },
      { parameter: $t.programVersion, value: data.programVersion },
      { parameter: $t.productionDate, value: data.productionDate },
      { parameter: $t.calibrationDate, value: data.calibrationDate },
      { parameter: $t.meterDate, value: data.meterDate },
      { parameter: $t.meterTime, value: data.meterTime },
      { parameter: $t.dayOfWeek, value: String(data.dayOfWeek) },
      { parameter: `${$t.activeEnergyImport} - ${$t.total}`, value: `${formatNumber(data.activeEnergyImportTotal)} kWh` },
      { parameter: `${$t.activeEnergyImport} - ${$t.t1Day}`, value: `${formatNumber(data.activeEnergyImportT1)} kWh` },
      { parameter: `${$t.activeEnergyImport} - ${$t.t2Peak}`, value: `${formatNumber(data.activeEnergyImportT2)} kWh` },
      { parameter: `${$t.activeEnergyImport} - ${$t.t3Night}`, value: `${formatNumber(data.activeEnergyImportT3)} kWh` },
      { parameter: $t.maxDemand, value: `${formatNumber(data.maxDemandImport)} kW @ ${data.maxDemandImportTimestamp}` },
      { parameter: $t.voltageL1, value: `${formatNumber(data.voltageL1, 1)} V` },
      { parameter: $t.voltageL2, value: `${formatNumber(data.voltageL2, 1)} V` },
      { parameter: $t.voltageL3, value: `${formatNumber(data.voltageL3, 1)} V` },
      { parameter: $t.currentL1, value: `${formatNumber(data.currentL1, 1)} A` },
      { parameter: $t.currentL2, value: `${formatNumber(data.currentL2, 1)} A` },
      { parameter: $t.currentL3, value: `${formatNumber(data.currentL3, 1)} A` },
      { parameter: $t.frequency, value: `${formatNumber(data.frequency, 1)} Hz` },
      { parameter: $t.powerFactorL1, value: formatNumber(data.powerFactorL1, 2) },
      { parameter: $t.powerFactorL2, value: formatNumber(data.powerFactorL2, 2) },
      { parameter: $t.powerFactorL3, value: formatNumber(data.powerFactorL3, 2) },
      { parameter: $t.ffStatusCode, value: data.ffCode },
      { parameter: $t.gfGeographicCode, value: data.gfCode },
      { parameter: $t.batteryStatus, value: data.batteryStatus === "full" ? $t.full : $t.low },
      { parameter: $t.relayStatus, value: data.relayStatus === "active" ? $t.active : $t.passive },
    ];

    exportToExcel(exportData, "short_read", [
      { key: "parameter", label: "Parameter" },
      { key: "value", label: "Value" },
    ]);
  }
</script>

<div class="space-y-6">
  <!-- Header Card -->
  <div
    class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm"
  >
    <div class="flex items-start justify-between">
      <div>
        <h3 class="text-xl font-bold text-slate-900 dark:text-white mb-2">{$t.shortRead}</h3>
        <p class="text-sm text-slate-500 dark:text-slate-400">{$t.shortReadDescription}</p>
      </div>
      <button
        onclick={startShortRead}
        disabled={!$isConnected || isReading}
        class="flex items-center gap-2 px-6 py-3 bg-primary hover:bg-primary/90 text-white font-bold rounded-lg shadow-lg shadow-primary/20 transition-all disabled:opacity-50 disabled:cursor-not-allowed"
      >
        {#if isReading}
          <Icon name="sync" class="animate-spin" />
          {$t.reading}
        {:else}
          <Icon name="play_arrow" />
          {$t.startShortRead}
        {/if}
      </button>
    </div>

    {#if !$isConnected}
      <div
        class="mt-4 p-4 bg-amber-500/10 border border-amber-500/20 rounded-xl text-amber-600 dark:text-amber-500 text-sm"
      >
        <div class="flex items-center gap-2">
          <Icon name="warning" />
          <span>{$t.connectFirstWarning}</span>
        </div>
      </div>
    {/if}
  </div>

  <!-- Progress Bar -->
  {#if $progressStore.active}
    <div
      class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm"
    >
      <div class="flex items-center justify-between mb-4">
        <h4 class="font-bold text-slate-900 dark:text-white">{$progressStore.title}</h4>
        <div class="flex items-center gap-4">
          <span class="text-sm font-mono text-primary">{$progressStore.percentage}%</span>
          <span class="text-sm text-slate-500 dark:text-slate-400">
            {$progressStore.elapsedTime.toFixed(1)}s
          </span>
        </div>
      </div>

      <!-- Progress Bar -->
      <div class="h-2 bg-slate-200 dark:bg-[#334a5e] rounded-lg overflow-hidden mb-6">
        <div
          class="h-full bg-gradient-to-r from-primary to-emerald-400 transition-all duration-300"
          style="width: {$progressStore.percentage}%"
        ></div>
      </div>

      <!-- Steps -->
      <div class="space-y-2">
        {#each $progressStore.steps as step}
          <div class="flex items-center gap-3">
            {#if step.status === "completed"}
              <Icon name="check_circle" class="text-emerald-500" size="sm" />
            {:else if step.status === "in_progress"}
              <Icon name="sync" class="text-primary animate-spin" size="sm" />
            {:else if step.status === "error"}
              <Icon name="error" class="text-red-500" size="sm" />
            {:else}
              <Icon name="circle" class="text-slate-300 dark:text-slate-600" size="sm" />
            {/if}
            <span
              class="text-sm {step.status === 'completed'
                ? 'text-emerald-600 dark:text-emerald-500'
                : step.status === 'in_progress'
                  ? 'text-slate-900 dark:text-white font-medium'
                  : step.status === 'error'
                    ? 'text-red-600 dark:text-red-500'
                    : 'text-slate-400 dark:text-slate-500'}"
            >
              {step.label}
            </span>
          </div>
        {/each}
      </div>
    </div>
  {/if}

  <!-- Results -->
  {#if readComplete && $meterStore.shortReadData}
    {@const data = $meterStore.shortReadData}
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
      <!-- Meter Identity -->
      <div
        class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm"
      >
        <h4 class="font-bold text-slate-900 dark:text-white mb-4 flex items-center gap-2">
          <Icon name="badge" class="text-primary" />
          {$t.meterIdentity}
        </h4>
        <div class="space-y-3">
          <div class="flex justify-between">
            <span class="text-sm text-slate-500">{$t.serialNumber}</span>
            <span class="text-sm font-mono font-bold text-slate-900 dark:text-white">
              {data.serialNumber}
            </span>
          </div>
          <div class="flex justify-between">
            <span class="text-sm text-slate-500">{$t.programVersion}</span>
            <span class="text-sm font-mono text-slate-900 dark:text-white">{data.programVersion}</span>
          </div>
          <div class="flex justify-between">
            <span class="text-sm text-slate-500">{$t.productionDate}</span>
            <span class="text-sm text-slate-900 dark:text-white">{data.productionDate}</span>
          </div>
          <div class="flex justify-between">
            <span class="text-sm text-slate-500">{$t.calibrationDate}</span>
            <span class="text-sm text-slate-900 dark:text-white">{data.calibrationDate}</span>
          </div>
        </div>
      </div>

      <!-- Date/Time -->
      <div
        class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm"
      >
        <h4 class="font-bold text-slate-900 dark:text-white mb-4 flex items-center gap-2">
          <Icon name="schedule" class="text-primary" />
          {$t.dateTime}
        </h4>
        <div class="space-y-3">
          <div class="flex justify-between">
            <span class="text-sm text-slate-500">{$t.meterDate}</span>
            <span class="text-sm font-mono text-slate-900 dark:text-white">{data.meterDate}</span>
          </div>
          <div class="flex justify-between">
            <span class="text-sm text-slate-500">{$t.meterTime}</span>
            <span class="text-sm font-mono font-bold text-slate-900 dark:text-white">
              {data.meterTime}
            </span>
          </div>
          <div class="flex justify-between">
            <span class="text-sm text-slate-500">{$t.dayOfWeek}</span>
            <span class="text-sm text-slate-900 dark:text-white">{data.dayOfWeek} ({$t.thursday})</span>
          </div>
        </div>
      </div>

      <!-- Active Energy Import -->
      <div
        class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm"
      >
        <h4 class="font-bold text-slate-900 dark:text-white mb-4 flex items-center gap-2">
          <Icon name="bolt" class="text-primary" />
          {$t.activeEnergyImport}
        </h4>
        <div class="space-y-3">
          <div class="flex justify-between">
            <span class="text-sm text-slate-500">{$t.total}</span>
            <span class="text-sm font-mono font-bold text-slate-900 dark:text-white">
              {formatNumber(data.activeEnergyImportTotal)} kWh
            </span>
          </div>
          <div class="flex justify-between">
            <span class="text-sm text-slate-500">{$t.t1Day}</span>
            <span class="text-sm font-mono text-slate-900 dark:text-white">
              {formatNumber(data.activeEnergyImportT1)} kWh
            </span>
          </div>
          <div class="flex justify-between">
            <span class="text-sm text-slate-500">{$t.t2Peak}</span>
            <span class="text-sm font-mono text-slate-900 dark:text-white">
              {formatNumber(data.activeEnergyImportT2)} kWh
            </span>
          </div>
          <div class="flex justify-between">
            <span class="text-sm text-slate-500">{$t.t3Night}</span>
            <span class="text-sm font-mono text-slate-900 dark:text-white">
              {formatNumber(data.activeEnergyImportT3)} kWh
            </span>
          </div>
        </div>
      </div>

      <!-- Instantaneous Values -->
      <div
        class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm md:col-span-2 lg:col-span-3"
      >
        <h4 class="font-bold text-slate-900 dark:text-white mb-4 flex items-center gap-2">
          <Icon name="speed" class="text-primary" />
          {$t.instantValues}
        </h4>
        <div class="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-5 gap-4">
          <div class="p-4 bg-slate-50 dark:bg-[#0f1821] rounded-xl text-center">
            <span class="text-xs text-slate-500 block mb-1">{$t.voltageL1}</span>
            <span class="text-lg font-mono font-bold text-slate-900 dark:text-white">
              {formatNumber(data.voltageL1, 1)} V
            </span>
          </div>
          <div class="p-4 bg-slate-50 dark:bg-[#0f1821] rounded-xl text-center">
            <span class="text-xs text-slate-500 block mb-1">{$t.voltageL2}</span>
            <span class="text-lg font-mono font-bold text-slate-900 dark:text-white">
              {formatNumber(data.voltageL2, 1)} V
            </span>
          </div>
          <div class="p-4 bg-slate-50 dark:bg-[#0f1821] rounded-xl text-center">
            <span class="text-xs text-slate-500 block mb-1">{$t.voltageL3}</span>
            <span class="text-lg font-mono font-bold text-slate-900 dark:text-white">
              {formatNumber(data.voltageL3, 1)} V
            </span>
          </div>
          <div class="p-4 bg-slate-50 dark:bg-[#0f1821] rounded-xl text-center">
            <span class="text-xs text-slate-500 block mb-1">{$t.frequency}</span>
            <span class="text-lg font-mono font-bold text-slate-900 dark:text-white">
              {formatNumber(data.frequency, 1)} Hz
            </span>
          </div>
          <div class="p-4 bg-slate-50 dark:bg-[#0f1821] rounded-xl text-center">
            <span class="text-xs text-slate-500 block mb-1">{$t.powerFactorL1}</span>
            <span class="text-lg font-mono font-bold text-slate-900 dark:text-white">
              {formatNumber(data.powerFactorL1, 2)}
            </span>
          </div>
        </div>
      </div>

      <!-- Status Codes -->
      <div
        class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm md:col-span-2 lg:col-span-3"
      >
        <h4 class="font-bold text-slate-900 dark:text-white mb-4 flex items-center gap-2">
          <Icon name="info" class="text-primary" />
          {$t.statusCodes}
        </h4>
        <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
          <div class="flex items-center gap-3">
            <span class="text-sm text-slate-500">{$t.ffStatusCode}:</span>
            <span class="text-sm font-mono text-slate-900 dark:text-white">{data.ffCode}</span>
          </div>
          <div class="flex items-center gap-3">
            <span class="text-sm text-slate-500">{$t.batteryStatus}:</span>
            <span
              class="text-sm font-bold {data.batteryStatus === 'full'
                ? 'text-emerald-500'
                : 'text-amber-500'}"
            >
              {data.batteryStatus === "full" ? $t.full : $t.low}
            </span>
          </div>
          <div class="flex items-center gap-3">
            <span class="text-sm text-slate-500">{$t.relayStatus}:</span>
            <span
              class="text-sm font-bold {data.relayStatus === 'active'
                ? 'text-emerald-500'
                : 'text-red-500'}"
            >
              {data.relayStatus === "active" ? $t.active : $t.passive}
            </span>
          </div>
        </div>
      </div>
    </div>

    <!-- Action Buttons -->
    <div class="flex justify-end gap-4">
      <button
        onclick={handleExport}
        class="flex items-center gap-2 px-6 py-3 bg-emerald-600 hover:bg-emerald-700 text-white font-bold rounded-lg transition-all"
      >
        <Icon name="download" />
        {$t.exportToExcel}
      </button>
      <button
        onclick={startShortRead}
        disabled={isReading}
        class="flex items-center gap-2 px-6 py-3 bg-primary hover:bg-primary/90 text-white font-bold rounded-lg shadow-lg shadow-primary/20 transition-all disabled:opacity-50"
      >
        <Icon name="refresh" />
        {$t.reRead}
      </button>
    </div>
  {/if}
</div>
