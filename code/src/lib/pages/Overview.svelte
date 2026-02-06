<script lang="ts">
  import Icon from "$lib/components/common/Icon.svelte";
  import { t, isConnected, connectionStore, meterStore, progressStore, addLog, isMeterReading } from "$lib/stores";
  import { readShort, onReadProgress, onCommLog, connect as tauriConnect, getConnectionStatus, getSetting } from "$lib/utils/tauri";
  import { onMount, onDestroy } from "svelte";
  import type { LogType } from "$lib/stores";

  let isReading = $state(false);

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
    unlistenProgress = await onReadProgress((event) => {
      if (event.step === 1) {
        const steps = shortReadSteps.map((s) => ({
          id: s.id,
          label: $t[s.label as keyof typeof $t] as string,
        }));
        progressStore.start($t.quickRead, steps);
      }
      const stepId = shortReadSteps[event.step - 1]?.id;
      if (stepId) progressStore.setStep(stepId);
      if (event.step === event.total) {
        progressStore.complete();
      }
    });

    unlistenLog = await onCommLog((event) => {
      const logType = event.logType.toLowerCase() as LogType;
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

  async function startQuickRead() {
    if ($isMeterReading) {
      addLog("warn", "Bir okuma islemi devam ediyor...");
      return;
    }

    isReading = true;
    meterStore.setReading(true);

    try {
      const connected = await getConnectionStatus();

      if (!connected) {
        addLog("info", "Baglanti yok, otomatik baglaniliyor...");
        const lastPort = await getSetting("lastPort") || "COM2";
        const lastBaud = await getSetting("lastBaud") || "0";
        const lastConnectionType = await getSetting("lastConnectionType") || "optical";

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
            connectionType: lastConnectionType as "optical" | "serial" | "auto",
          });

          connectionStore.setMeterIdentity({
            flag: identity.manufacturer,
            manufacturer: identity.manufacturer,
            baudChar: identity.baudRateChar,
            edasId: identity.edasId,
            model: identity.model,
            serialNumber: identity.serialNumber || "",
          });

          addLog("success", `Baglanti basarili: ${identity.manufacturer} - ${identity.model}`);
        } catch (e) {
          addLog("error", `Baglanti basarisiz: ${e}`);
          isReading = false;
          meterStore.setReading(false);
          return;
        }
      }

      addLog("info", "Kisa okuma baslatiliyor...");
      const result = await readShort();

      let meterType: "single-phase" | "three-phase" | "kombi" = "single-phase";
      if (result.voltageL2 > 0 || result.voltageL3 > 0) {
        meterType = "three-phase";
      }

      meterStore.setShortReadData(result, meterType, false);
      addLog("success", "Kisa okuma basariyla tamamlandi!");
    } catch (error) {
      addLog("error", `Kisa okuma hatasi: ${error}`);
      progressStore.reset();
    } finally {
      isReading = false;
      meterStore.setReading(false);
    }
  }

  // Parse FF code to determine health status
  function parseFFHealth(ffCode: string | undefined) {
    if (!ffCode) return { systemBattery: true, clockBattery: true, topCover: false, terminalCover: false, magnetic: false, relay: true };

    try {
      const ffValue = BigInt("0x" + ffCode);
      return {
        systemBattery: (ffValue & (1n << 37n)) === 0n, // Bit 37: System Battery
        clockBattery: (ffValue & (1n << 38n)) === 0n,  // Bit 38: Clock Battery
        topCover: (ffValue & (1n << 6n)) !== 0n,       // Bit 6: Top Cover Open
        terminalCover: (ffValue & (1n << 5n)) !== 0n,  // Bit 5: Terminal Cover Open
        magnetic: ((ffValue & (1n << 11n)) !== 0n) || ((ffValue & (1n << 12n)) !== 0n) || ((ffValue & (1n << 13n)) !== 0n), // Bits 11-13: Magnetic
        relay: true, // Will be overridden from relayStatus
      };
    } catch {
      return { systemBattery: true, clockBattery: true, topCover: false, terminalCover: false, magnetic: false, relay: true };
    }
  }

  let health = $derived(parseFFHealth($meterStore.shortReadData?.ffCode));
  let relayStatus = $derived($meterStore.shortReadData?.relayStatus);
  let hasRelayData = $derived(relayStatus !== undefined && relayStatus !== null && relayStatus !== "");
  let relayActive = $derived(relayStatus === "active");
</script>

<div class="space-y-6">
  <!-- Header -->
  <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm">
    <div class="flex items-start justify-between">
      <div>
        <h3 class="text-xl font-bold text-slate-900 dark:text-white mb-2">{$t.overview}</h3>
        <p class="text-sm text-slate-500 dark:text-slate-400">{$t.overviewDescription}</p>
      </div>
    </div>
  </div>

  {#if !$isConnected}
    <div class="bg-amber-500/10 border border-amber-500/20 rounded-xl p-6 text-center">
      <Icon name="warning" class="text-4xl text-amber-500 mb-3" />
      <p class="text-amber-600 dark:text-amber-400 font-medium">{$t.connectFirstWarning}</p>
    </div>
  {:else}
    <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
      <!-- Left Column: Meter Identity -->
      <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm">
        <h4 class="font-bold text-slate-900 dark:text-white mb-4 flex items-center gap-2">
          <Icon name="badge" class="text-primary" />
          {$t.meterIdentity}
        </h4>

        {#if $meterStore.shortReadData}
          {@const data = $meterStore.shortReadData}
          <div class="space-y-4">
            <div class="p-4 bg-gradient-to-br from-primary/10 to-emerald-500/10 rounded-xl border border-primary/20">
              <div class="text-xs text-slate-500 mb-1">{$t.serialNumber}</div>
              <div class="text-2xl font-mono font-bold {data.serialNumber ? 'text-primary' : 'text-slate-400'}">
                {data.serialNumber || "-"}
              </div>
            </div>

            <div class="space-y-3">
              <div class="flex justify-between items-center py-2 border-b border-slate-100 dark:border-[#334a5e]">
                <span class="text-sm text-slate-500">{$t.programVersion}</span>
                <span class="text-sm font-mono font-bold text-slate-900 dark:text-white">{data.programVersion}</span>
              </div>
              <div class="flex justify-between items-center py-2 border-b border-slate-100 dark:border-[#334a5e]">
                <span class="text-sm text-slate-500">{$t.productionDate}</span>
                <span class="text-sm text-slate-900 dark:text-white">{data.productionDate}</span>
              </div>
              <div class="flex justify-between items-center py-2 border-b border-slate-100 dark:border-[#334a5e]">
                <span class="text-sm text-slate-500">{$t.calibrationDate}</span>
                <span class="text-sm text-slate-900 dark:text-white">{data.calibrationDate}</span>
              </div>
              {#if $connectionStore.meterIdentity}
                <div class="flex justify-between items-center py-2">
                  <span class="text-sm text-slate-500">Model</span>
                  <span class="text-sm font-bold text-slate-900 dark:text-white">{$connectionStore.meterIdentity.model}</span>
                </div>
              {/if}
            </div>
          </div>
        {:else}
          <div class="text-center py-8 text-slate-400">
            <Icon name="info" class="text-4xl mb-2" />
            <p class="text-sm">Veri yok - Okuma yapın</p>
          </div>
        {/if}
      </div>

      <!-- Center Column: Clock & Time -->
      <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm">
        <h4 class="font-bold text-slate-900 dark:text-white mb-4 flex items-center gap-2">
          <Icon name="schedule" class="text-primary" />
          {$t.dateTime}
        </h4>

        {#if $meterStore.shortReadData}
          {@const data = $meterStore.shortReadData}
          <div class="space-y-4">
            <!-- Large Clock Display -->
            <div class="p-6 bg-gradient-to-br from-slate-900 to-slate-800 dark:from-[#0f1821] dark:to-[#1a2632] rounded-xl text-center">
              <div class="text-4xl font-mono font-bold text-white tracking-wider mb-2">
                {data.meterTime}
              </div>
              <div class="text-lg text-slate-300">{data.meterDate}</div>
            </div>

            <!-- DST Status -->
            <div class="flex items-center justify-between p-3 bg-slate-50 dark:bg-[#0f1821] rounded-lg">
              <div class="flex items-center gap-2">
                <Icon name="wb_sunny" class="text-amber-500" size="sm" />
                <span class="text-sm text-slate-600 dark:text-slate-400">{$t.dstStatus}</span>
              </div>
              <span class="text-sm font-bold text-slate-900 dark:text-white">
                {$t.dstInactive}
              </span>
            </div>

            <!-- Time Drift (placeholder) -->
            <div class="flex items-center justify-between p-3 bg-slate-50 dark:bg-[#0f1821] rounded-lg">
              <div class="flex items-center gap-2">
                <Icon name="sync_problem" class="text-slate-400" size="sm" />
                <span class="text-sm text-slate-600 dark:text-slate-400">{$t.timeDriftStatus}</span>
              </div>
              <span class="text-sm font-mono text-emerald-500">+0s</span>
            </div>
          </div>
        {:else}
          <div class="text-center py-8 text-slate-400">
            <Icon name="schedule" class="text-4xl mb-2" />
            <p class="text-sm">Saat bilgisi yok</p>
          </div>
        {/if}
      </div>

      <!-- Right Column: Health Status -->
      <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm">
        <h4 class="font-bold text-slate-900 dark:text-white mb-4 flex items-center gap-2">
          <Icon name="health_and_safety" class="text-primary" />
          {$t.meterHealth}
        </h4>

        <div class="space-y-3">
          <!-- System Battery -->
          <div class="flex items-center justify-between p-3 rounded-lg {health.systemBattery ? 'bg-emerald-500/10 border border-emerald-500/20' : 'bg-red-500/10 border border-red-500/20'}">
            <div class="flex items-center gap-2">
              <Icon name="battery_full" class="{health.systemBattery ? 'text-emerald-500' : 'text-red-500'}" size="sm" />
              <span class="text-sm text-slate-700 dark:text-slate-300">{$t.systemBattery}</span>
            </div>
            <span class="text-sm font-bold {health.systemBattery ? 'text-emerald-500' : 'text-red-500'}">
              {health.systemBattery ? $t.full : $t.low}
            </span>
          </div>

          <!-- Clock Battery -->
          <div class="flex items-center justify-between p-3 rounded-lg {health.clockBattery ? 'bg-emerald-500/10 border border-emerald-500/20' : 'bg-red-500/10 border border-red-500/20'}">
            <div class="flex items-center gap-2">
              <Icon name="schedule" class="{health.clockBattery ? 'text-emerald-500' : 'text-red-500'}" size="sm" />
              <span class="text-sm text-slate-700 dark:text-slate-300">{$t.clockBattery}</span>
            </div>
            <span class="text-sm font-bold {health.clockBattery ? 'text-emerald-500' : 'text-red-500'}">
              {health.clockBattery ? $t.full : $t.low}
            </span>
          </div>

          <!-- Top Cover -->
          <div class="flex items-center justify-between p-3 rounded-lg {!health.topCover ? 'bg-emerald-500/10 border border-emerald-500/20' : 'bg-amber-500/10 border border-amber-500/20'}">
            <div class="flex items-center gap-2">
              <Icon name="door_open" class="{!health.topCover ? 'text-emerald-500' : 'text-amber-500'}" size="sm" />
              <span class="text-sm text-slate-700 dark:text-slate-300">{$t.topCover}</span>
            </div>
            <span class="text-sm font-bold {!health.topCover ? 'text-emerald-500' : 'text-amber-500'}">
              {health.topCover ? $t.open : $t.closed}
            </span>
          </div>

          <!-- Terminal Cover -->
          <div class="flex items-center justify-between p-3 rounded-lg {!health.terminalCover ? 'bg-emerald-500/10 border border-emerald-500/20' : 'bg-amber-500/10 border border-amber-500/20'}">
            <div class="flex items-center gap-2">
              <Icon name="sensor_door" class="{!health.terminalCover ? 'text-emerald-500' : 'text-amber-500'}" size="sm" />
              <span class="text-sm text-slate-700 dark:text-slate-300">{$t.terminalCoverStatus}</span>
            </div>
            <span class="text-sm font-bold {!health.terminalCover ? 'text-emerald-500' : 'text-amber-500'}">
              {health.terminalCover ? $t.open : $t.closed}
            </span>
          </div>

          <!-- Magnetic Tampering -->
          <div class="flex items-center justify-between p-3 rounded-lg {!health.magnetic ? 'bg-emerald-500/10 border border-emerald-500/20' : 'bg-red-500/10 border border-red-500/20'}">
            <div class="flex items-center gap-2">
              <Icon name="sensors" class="{!health.magnetic ? 'text-emerald-500' : 'text-red-500'}" size="sm" />
              <span class="text-sm text-slate-700 dark:text-slate-300">{$t.magneticTampering}</span>
            </div>
            <span class="text-sm font-bold {!health.magnetic ? 'text-emerald-500' : 'text-red-500'}">
              {health.magnetic ? $t.detected : $t.notDetected}
            </span>
          </div>

          <!-- Relay Status -->
          <div class="flex items-center justify-between p-3 rounded-lg {hasRelayData ? (relayActive ? 'bg-emerald-500/10 border border-emerald-500/20' : 'bg-red-500/10 border border-red-500/20') : 'bg-slate-100 dark:bg-[#1a2632] border border-slate-200 dark:border-[#334a5e]'}">
            <div class="flex items-center gap-2">
              <Icon name="power" class="{hasRelayData ? (relayActive ? 'text-emerald-500' : 'text-red-500') : 'text-slate-400'}" size="sm" />
              <span class="text-sm {hasRelayData ? 'text-slate-700 dark:text-slate-300' : 'text-slate-400'}">{$t.relayState}</span>
            </div>
            <span class="text-sm font-bold {hasRelayData ? (relayActive ? 'text-emerald-500' : 'text-red-500') : 'text-slate-400'}">
              {hasRelayData ? (relayActive ? $t.energyOn : $t.energyCut) : '-'}
            </span>
          </div>
        </div>
      </div>
    </div>

    <!-- Read Buttons -->
    <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm">
      <div class="flex flex-wrap items-center justify-center gap-4">
        <button
          onclick={startQuickRead}
          disabled={isReading}
          class="flex items-center gap-3 px-8 py-4 bg-primary hover:bg-primary/90 text-white font-bold rounded-xl shadow-lg shadow-primary/20 transition-all disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {#if isReading}
            <Icon name="sync" class="animate-spin" />
            {$t.reading}
          {:else}
            <Icon name="bolt" />
            {$t.quickRead}
          {/if}
        </button>

        <button
          disabled={isReading}
          class="flex items-center gap-3 px-8 py-4 bg-slate-100 dark:bg-[#334a5e] hover:bg-slate-200 dark:hover:bg-[#455a6e] text-slate-700 dark:text-white font-bold rounded-xl transition-all disabled:opacity-50 disabled:cursor-not-allowed"
        >
          <Icon name="assignment" />
          {$t.fullReadBtn}
        </button>
      </div>

      <!-- Progress Bar -->
      {#if $progressStore.active}
        <div class="mt-6">
          <div class="flex items-center justify-between mb-2">
            <span class="text-sm font-medium text-slate-600 dark:text-slate-400">{$progressStore.title}</span>
            <span class="text-sm font-mono text-primary">{$progressStore.percentage}%</span>
          </div>
          <div class="h-2 bg-slate-200 dark:bg-[#334a5e] rounded-full overflow-hidden">
            <div
              class="h-full bg-gradient-to-r from-primary to-emerald-400 transition-all duration-300"
              style="width: {$progressStore.percentage}%"
            ></div>
          </div>
        </div>
      {/if}
    </div>
  {/if}
</div>
