<script lang="ts">
  import Icon from "$lib/components/common/Icon.svelte";
  import { t, isConnected, connectionStore, progressStore, meterStore, isMeterReading, addLog, type LogType } from "$lib/stores";
  import { exportToExcel } from "$lib/utils/export";
  import { readShort, onReadProgress, onCommLog, connect as tauriConnect, getConnectionStatus, getSetting, listSerialPorts } from "$lib/utils/tauri";
  import { onMount, onDestroy } from "svelte";
  import { get } from "svelte/store";

  let isReading = $state(false);
  let readComplete = $state(false);

  // Debug: watch for meter data changes
  $effect(() => {
    console.log("[ShortRead $effect] Triggered, checking meterStore...");
    console.log("[ShortRead $effect] Full store state:", $meterStore);
    if ($meterStore.shortReadData) {
      console.log("[ShortRead $effect] ✓ Meter data available!", $meterStore.shortReadData);
    } else {
      console.log("[ShortRead $effect] ✗ No meter data yet, shortReadData is:", $meterStore.shortReadData);
    }
  });

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

  let unlistenProgress: (() => void) | null = null;
  let unlistenLog: (() => void) | null = null;

  onMount(async () => {
    // Listen to progress events from backend
    unlistenProgress = await onReadProgress((event) => {
      if (event.step === 1) {
        const steps = shortReadSteps.map((s) => ({
          id: s.id,
          label: $t[s.label as keyof typeof $t] as string,
        }));
        progressStore.start($t.shortRead, steps);
      }
      progressStore.setStep(event.step - 1);
      if (event.step === event.total) {
        progressStore.complete();
      }
    });

    // Listen to communication log events from backend
    unlistenLog = await onCommLog((event) => {
      const logType = event.logType.toLowerCase() as LogType;
      // For TX/RX, include the data field if available
      if ((logType === 'tx' || logType === 'rx') && event.data) {
        addLog(logType, event.data);
      } else {
        addLog(logType, event.message);
      }
    });
  });

  onDestroy(() => {
    if (unlistenProgress) unlistenProgress();
    if (unlistenLog) unlistenLog();
  });

  async function startShortRead() {
    // Block if already reading
    if ($isMeterReading) {
      addLog("warn", "Bir okuma işlemi devam ediyor, lütfen bekleyin...");
      return;
    }

    isReading = true;
    readComplete = false;
    meterStore.setReading(true);

    try {
      // Check if already connected
      const connected = await getConnectionStatus();

      if (!connected) {
        addLog("info", "Bağlantı yok, otomatik bağlanılıyor...");

        // Get last used connection settings or use defaults
        const lastPort = await getSetting("lastPort") || "COM2";
        const lastBaud = await getSetting("lastBaud") || "0"; // 0 = auto
        const lastConnectionType = await getSetting("lastConnectionType") || "optical";

        // Try to connect
        try {
          const identity = await tauriConnect({
            connectionType: lastConnectionType,
            port: lastPort,
            baudRate: parseInt(lastBaud),
            timeoutMs: 2000,
            meterAddress: null,
            password: null,
          });

          connectionStore.connect({
            port: lastPort,
            baudRate: parseInt(lastBaud) || 9600,
            connectionType: lastConnectionType as "optical" | "rs485",
          });

          connectionStore.setMeterIdentity({
            flag: identity.manufacturer,
            manufacturer: identity.manufacturer,
            baudChar: identity.baudRateChar,
            edasId: identity.edasId,
            model: identity.model,
            serialNumber: identity.serialNumber || "",
          });

          addLog("success", `Bağlantı başarılı: ${identity.manufacturer} — ${identity.model}`);
        } catch (e) {
          addLog("error", `Bağlantı başarısız: ${e}`);
          isReading = false;
          return;
        }
      }

      addLog("info", "Kısa okuma başlatılıyor...");

      // Call the real Tauri backend command
      const result = await readShort();

      // Detect meter type based on voltage readings
      let meterType = "single-phase";
      if (result.voltageL2 > 0 || result.voltageL3 > 0) {
        meterType = "three-phase";
      }

      // Check if bidirectional (would need reverse energy registers from full read)
      const isBidirectional = false;

      // Store the real data from meter
      meterStore.setShortReadData(result, meterType, isBidirectional);

      addLog("success", "Kısa okuma başarıyla tamamlandı!");
      readComplete = true;
    } catch (error) {
      addLog("error", `Kısa okuma hatası: ${error}`);
      progressStore.reset();
    } finally {
      isReading = false;
      meterStore.setReading(false);
    }
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
        disabled={isReading}
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
  </div>

  <!-- Progress Bar -->
  {#if isReading}
    <div
      class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm"
    >
      <div class="flex items-center justify-between mb-4">
        <div class="flex items-center gap-3">
          <Icon name="sync" class="text-primary animate-spin" />
          <h4 class="font-bold text-slate-900 dark:text-white">{$t.shortRead}</h4>
        </div>
        <div class="flex items-center gap-4">
          <span class="text-sm font-mono text-primary">{$progressStore.percentage}%</span>
          <span class="text-sm text-slate-500 dark:text-slate-400">
            {$progressStore.elapsedTime.toFixed(1)}s
          </span>
        </div>
      </div>

      <!-- Progress Bar -->
      <div class="h-2 bg-slate-200 dark:bg-[#334a5e] rounded-lg overflow-hidden">
        <div
          class="h-full bg-gradient-to-r from-primary to-emerald-400 transition-all duration-300"
          style="width: {$progressStore.percentage}%"
        ></div>
      </div>
    </div>
  {/if}

  <!-- Results -->
  {#if $meterStore.shortReadData}
    {@const data = $meterStore.shortReadData}
    {console.log("ShortRead page rendering data:", data)}
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
