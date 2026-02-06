<script lang="ts">
  import Icon from "$lib/components/common/Icon.svelte";
  import { t, connectionStore, isConnected, isConnecting, addLog, meterStore, isMeterReading, errorToast, successToast, sessionsStore, navigationStore, type SessionInfo } from "$lib/stores";
  import { onMount } from "svelte";
  import { listSerialPorts, connect as tauriConnect, disconnect as tauriDisconnect, readFull, setSetting, loadSessionFile, type PortInfo } from "$lib/utils/tauri";

  // Connection parameters
  let connectionType = $state("auto");
  let selectedPort = $state("");
  let baudRate = $state("auto");
  let meterAddress = $state("");
  let timeoutMs = $state(2000);

  // Serial ports from backend
  let serialPorts = $state<{ name: string; description: string; active: boolean }[]>([]);
  let loadingPorts = $state(false);

  // Computed state for ports display
  let portsLoading = $derived(loadingPorts);

  // Fetch serial ports and previous sessions on mount
  onMount(async () => {
    // Delay port refresh slightly to let UI render first
    setTimeout(() => refreshPorts(), 500);

    // Load previous sessions from saved files
    sessionsStore.refresh();
  });

  async function refreshPorts() {
    loadingPorts = true;
    try {
      const ports = await listSerialPorts();
      serialPorts = ports.map((p: PortInfo) => ({
        name: p.name,
        description: p.description || p.portType,
        active: false,
      }));
      // Select first port if none selected
      if (serialPorts.length > 0 && !selectedPort) {
        selectedPort = serialPorts[0].name;
      }
      addLog("info", `${serialPorts.length} seri port bulundu`);
    } catch (e) {
      console.error("Failed to list ports:", e);
      addLog("error", `Port listesi alınamadı: ${e}`);
      // Add manual port entry as fallback
      serialPorts = [
        { name: "COM1", description: "Manual", active: false },
        { name: "COM2", description: "Manual", active: false },
        { name: "COM3", description: "Manual", active: false },
        { name: "COM4", description: "Manual", active: false },
      ];
      selectedPort = "COM2";
    } finally {
      loadingPorts = false;
    }
  }

  // Previous sessions (from store, loaded from saved session files)
  let previousSessions = $derived($sessionsStore);

  // Session load confirmation dialog
  let showLoadConfirmDialog = $state(false);
  let pendingSessionToLoad = $state<SessionInfo | null>(null);
  let isLoadingSession = $state(false);

  // Session delete confirmation dialog
  let showDeleteConfirmDialog = $state(false);
  let pendingSessionToDelete = $state<SessionInfo | null>(null);
  let isDeletingSession = $state(false);

  const connectionTypes = [
    { value: "auto", labelKey: "autoDetect" as const },
    { value: "optical", labelKey: "opticalProbe" as const },
    { value: "serial", labelKey: "serialDirect" as const },
  ];

  // When optical is selected, force baudRate to auto (IEC 62056-21 requires 300 bps start)
  $effect(() => {
    if (connectionType === "optical") {
      baudRate = "auto";
    }
  });

  const baudRates = [
    { value: "auto", label: null },
    { value: "300", label: "300" },
    { value: "600", label: "600" },
    { value: "1200", label: "1200" },
    { value: "2400", label: "2400" },
    { value: "4800", label: "4800" },
    { value: "9600", label: "9600" },
    { value: "19200", label: "19200" },
  ];

  async function connect() {
    if ($isConnected) {
      try {
        await tauriDisconnect();
        connectionStore.disconnect();
        addLog("info", "Bağlantı kesildi");
      } catch (e) {
        addLog("error", `Bağlantı kesilemedi: ${e}`);
      }
    } else {
      connectionStore.setConnecting(true);
      const addressInfo = meterAddress ? ` (Adres: ${meterAddress})` : "";
      addLog("info", `${selectedPort} portuna bağlanılıyor...${addressInfo}`);

      try {
        // Parse baud rate - "auto" means 0 (let protocol negotiate)
        const baud = baudRate === "auto" ? 0 : parseInt(baudRate);

        const identity = await tauriConnect({
          connectionType: connectionType,
          port: selectedPort,
          baudRate: baud,
          timeoutMs: timeoutMs,
          meterAddress: meterAddress || null,
          password: null,
        });

        connectionStore.connect({
          port: selectedPort,
          baudRate: baud || 9600,
          connectionType: connectionType as "optical" | "serial" | "auto",
          meterAddress: meterAddress || undefined,
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

        // Save connection settings for later auto-connect
        await setSetting("lastPort", selectedPort);
        await setSetting("lastBaud", String(baud));
        await setSetting("lastConnectionType", connectionType);

        // Auto-trigger full read in background after connection to populate all sections
        setTimeout(async () => {
          try {
            meterStore.setReading(true);
            addLog("info", "Sayaç verileri tam okuma ile alınıyor (Mod 0)...");
            console.log("[Home] About to call readFull()...");
            const result = await readFull();
            console.log("[Home] readFull() returned successfully:", result);

            // Detect meter type by OBIS code presence (not voltage value — unloaded phases read 0V)
            let meterType: "single-phase" | "three-phase" | "kombi" = "single-phase";
            const raw = result.rawData || "";
            if (raw.includes("52.7.0") || raw.includes("72.7.0")) {
              meterType = "three-phase";
            }

            // Store data - this populates Short Read, Full Read, and Alarms sections
            // The result from Mode 0 (Readout) contains all available OBIS codes
            console.log("[Home] Storing meter data:", result);
            console.log("[Home] About to call meterStore.setShortReadData...");
            meterStore.setShortReadData(result, meterType, false);
            console.log("[Home] meterStore.setShortReadData completed");

            // The same data is available in all sections until disconnect
            // Events and Alarms pages will read from shortReadData.ffCode and .gfCode

            addLog("success", `Tüm sayaç verileri başarıyla okundu - ${meterType} sayaç tespit edildi`);
            console.log("[Home] Success! Meter store updated, data should be visible now");
          } catch (e) {
            console.error("[Home] ERROR in auto-read:", e);
            addLog("warn", `Otomatik tam okuma başarısız: ${e}`);
          } finally {
            meterStore.setReading(false);
          }
        }, 500);
      } catch (e) {
        addLog("error", `Bağlantı hatası: ${e}`);
        errorToast(`${$t.connectionFailed}: ${e}`, 5000);
        connectionStore.setConnecting(false);
      }
    }
  }

  function loadSession(session: SessionInfo) {
    // Check if there's an active session with data
    if ($meterStore.shortReadData) {
      // Show confirmation dialog
      pendingSessionToLoad = session;
      showLoadConfirmDialog = true;
    } else {
      // No active session, load directly
      performSessionLoad(session);
    }
  }

  function cancelLoadSession() {
    showLoadConfirmDialog = false;
    pendingSessionToLoad = null;
  }

  function confirmLoadSession() {
    if (pendingSessionToLoad) {
      performSessionLoad(pendingSessionToLoad);
    }
    showLoadConfirmDialog = false;
    pendingSessionToLoad = null;
  }

  async function performSessionLoad(session: SessionInfo) {
    if (!session.fileName) {
      errorToast($t.sessionLoadError || "Session file not found");
      return;
    }

    isLoadingSession = true;
    addLog("info", `Oturum yükleniyor: ${session.flag} — ${session.serialNumber}`);

    try {
      const sessionData = await loadSessionFile(session.fileName);

      // Extract meter data from session
      const meterData = sessionData.meterData as {
        shortReadData?: Record<string, unknown>;
        fullReadData?: Record<string, unknown>;
        loadProfileData?: Record<string, unknown>;
        meterType?: "single-phase" | "three-phase" | "kombi";
        isBidirectional?: boolean;
      };

      if (meterData?.shortReadData) {
        // Load the data into meterStore
        meterStore.setShortReadData(
          meterData.shortReadData as unknown as Parameters<typeof meterStore.setShortReadData>[0],
          meterData.meterType || "single-phase",
          meterData.isBidirectional || false
        );

        // Load full read data if available
        if (meterData.fullReadData) {
          meterStore.setFullReadData(
            meterData.fullReadData as unknown as Parameters<typeof meterStore.setFullReadData>[0]
          );
        }

        // Load profile data if available
        if (meterData.loadProfileData) {
          meterStore.setLoadProfileData(
            meterData.loadProfileData as unknown as Parameters<typeof meterStore.setLoadProfileData>[0]
          );
        }

        // Set connection status to "connected" so pages display the data
        connectionStore.setConnected({
          flag: sessionData.flag || session.flag,
          manufacturer: sessionData.flag || session.flag,
          edasId: "",
          model: sessionData.model || "",
          baudChar: "",
          serialNumber: sessionData.serialNumber || session.serialNumber,
        });

        successToast($t.sessionLoaded || `Session loaded: ${session.flag} — ${session.serialNumber}`);
        addLog("success", `Oturum yüklendi: ${session.flag} — ${session.serialNumber}`);
      } else {
        errorToast($t.sessionLoadError || "Invalid session data");
        addLog("error", "Oturum verisi geçersiz");
      }
    } catch (e) {
      console.error("Failed to load session:", e);
      errorToast(`${$t.sessionLoadError || "Failed to load session"}: ${e}`);
      addLog("error", `Oturum yüklenemedi: ${e}`);
    } finally {
      isLoadingSession = false;
    }
  }

  function promptDeleteSession(session: SessionInfo, event: MouseEvent) {
    event.stopPropagation();
    pendingSessionToDelete = session;
    showDeleteConfirmDialog = true;
  }

  function cancelDeleteSession() {
    showDeleteConfirmDialog = false;
    pendingSessionToDelete = null;
  }

  async function confirmDeleteSession() {
    if (!pendingSessionToDelete) return;

    isDeletingSession = true;
    try {
      await sessionsStore.delete(pendingSessionToDelete.fileName);
      successToast($t.sessionDeleted || "Session deleted");
      addLog("info", `Oturum silindi: ${pendingSessionToDelete.flag} — ${pendingSessionToDelete.serialNumber}`);
    } catch (e) {
      console.error("Failed to delete session:", e);
      errorToast(`${$t.sessionDeleteError || "Failed to delete session"}: ${e}`);
    } finally {
      isDeletingSession = false;
      showDeleteConfirmDialog = false;
      pendingSessionToDelete = null;
    }
  }

  let showPortsAsDropdown = $derived(serialPorts.length > 5);
  let hasNoPorts = $derived(serialPorts.length === 0);
</script>

<div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
  <!-- New Connection Card -->
  <div
    class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl shadow-sm overflow-hidden"
  >
    <!-- Card Header -->
    <div
      class="bg-gradient-to-r from-primary/10 to-emerald-500/10 dark:from-primary/20 dark:to-emerald-500/20 px-6 py-4 border-b border-slate-200 dark:border-[#334a5e]"
    >
      <div class="flex items-center gap-3">
        <div class="p-2 bg-primary/10 rounded-lg">
          <Icon name="cable" class="text-primary" />
        </div>
        <div>
          <h3 class="text-lg font-bold text-slate-900 dark:text-white">{$t.newConnection}</h3>
          <p class="text-xs text-slate-500 dark:text-slate-400">{$t.systemReadyMessage}</p>
        </div>
      </div>
    </div>

    <!-- Card Body -->
    <div class="p-6">
      <div class="flex flex-col lg:flex-row gap-6">
        <!-- Parameters Section -->
        <div class="flex-1 space-y-4">
          <!-- Connection Type -->
          <div class="flex flex-col gap-1.5">
            <label class="text-xs font-bold text-slate-500 dark:text-slate-400 uppercase tracking-wider">
              {$t.connectionType}
            </label>
            <select
              bind:value={connectionType}
              class="w-full px-3 py-2.5 bg-white dark:bg-[#1a2632] border border-slate-300 dark:border-[#334a5e] rounded-lg text-sm text-slate-900 dark:text-white focus:border-primary focus:ring-1 focus:ring-primary transition-colors"
            >
              {#each connectionTypes as type}
                <option value={type.value}>{$t[type.labelKey]}</option>
              {/each}
            </select>
          </div>

          <!-- Baud Rate -->
          <div class="flex flex-col gap-1.5">
            <label class="text-xs font-bold text-slate-500 dark:text-slate-400 uppercase tracking-wider">
              {$t.baudRate}
            </label>
            <select
              bind:value={baudRate}
              disabled={connectionType === "optical"}
              class="w-full px-3 py-2.5 bg-white dark:bg-[#1a2632] border border-slate-300 dark:border-[#334a5e] rounded-lg text-sm text-slate-900 dark:text-white focus:border-primary focus:ring-1 focus:ring-primary transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {#each baudRates as rate}
                <option value={rate.value}>{rate.label || $t.autoDetect}</option>
              {/each}
            </select>
          </div>

          <!-- Meter Address (Optional) -->
          <div class="flex flex-col gap-1.5">
            <label class="text-xs font-bold text-slate-500 dark:text-slate-400 uppercase tracking-wider">
              {$t.meterAddress}
              <span class="font-normal normal-case text-slate-400 dark:text-slate-500 ml-1">(opsiyonel)</span>
            </label>
            <input
              type="text"
              bind:value={meterAddress}
              placeholder={$t.meterAddressPlaceholder}
              class="w-full px-3 py-2.5 bg-white dark:bg-[#1a2632] border border-slate-300 dark:border-[#334a5e] rounded-lg text-sm font-mono text-slate-900 dark:text-white placeholder-slate-400 dark:placeholder-slate-500 focus:border-primary focus:ring-1 focus:ring-primary transition-colors"
            />
          </div>

          <!-- Connect Button -->
          <button
            onclick={connect}
            disabled={hasNoPorts || $isConnecting || $isMeterReading}
            class="relative w-full mt-2 flex items-center justify-center gap-2 px-6 py-3 bg-primary hover:bg-primary/90 disabled:bg-slate-300 dark:disabled:bg-slate-700 text-white font-bold rounded-lg shadow-lg transition-all disabled:cursor-not-allowed overflow-hidden
              {$isMeterReading ? 'shadow-primary/50 shadow-2xl' : 'shadow-primary/20'}"
          >
            {#if $isMeterReading}
              <!-- Glow animation when reading -->
              <div class="absolute inset-0 bg-gradient-to-r from-primary via-emerald-400 to-primary animate-pulse opacity-30"></div>
              <Icon name="sync" class="animate-spin relative z-10" />
              <span class="relative z-10">{$t.reading || "Okunuyor"}...</span>
            {:else if $isConnecting}
              <Icon name="sync" class="animate-spin" />
              <span>{$t.connecting || "Bağlanıyor"}...</span>
            {:else if $isConnected}
              <Icon name="power_off" />
              {$t.disconnect}
            {:else}
              <Icon name="power_settings_new" />
              {$t.connectNow}
            {/if}
          </button>
        </div>

        <!-- Serial Ports Section -->
        <div class="flex-1">
          <label class="text-xs font-bold text-slate-500 dark:text-slate-400 uppercase tracking-wider block mb-2">
            {$t.serialPorts}
            <span class="text-primary ml-1">({serialPorts.length} {$t.available})</span>
          </label>

          {#if portsLoading}
            <!-- Loading Ports -->
            <div class="p-6 bg-slate-50 dark:bg-[#0f1821] border border-slate-200 dark:border-[#334a5e] rounded-lg text-center">
              <Icon name="sync" class="text-primary text-3xl mb-2 animate-spin" />
              <p class="text-sm font-bold text-slate-600 dark:text-slate-400">{$t.loadingPorts}</p>
            </div>
          {:else if hasNoPorts}
            <!-- No Ports Detected -->
            <div class="p-6 bg-amber-500/10 border border-amber-500/20 rounded-lg text-center">
              <Icon name="usb_off" class="text-amber-500 text-3xl mb-2" />
              <p class="text-sm font-bold text-amber-600 dark:text-amber-500">{$t.noPortDetected}</p>
              <p class="text-xs text-amber-500/80 mt-1">{$t.checkConnection}</p>
            </div>
          {:else if showPortsAsDropdown}
            <!-- Dropdown for many ports -->
            <select
              bind:value={selectedPort}
              class="w-full px-3 py-2.5 bg-white dark:bg-[#1a2632] border border-slate-300 dark:border-[#334a5e] rounded-lg text-sm text-slate-900 dark:text-white focus:border-primary focus:ring-1 focus:ring-primary transition-colors"
            >
              {#each serialPorts as port}
                <option value={port.name}>{port.name} — {port.description}</option>
              {/each}
            </select>
          {:else}
            <!-- Visual Port Cards -->
            <div class="space-y-2">
              {#each serialPorts as port}
                <button
                  onclick={() => (selectedPort = port.name)}
                  class="w-full flex items-center gap-3 p-3 rounded-lg border transition-all text-left
                    {selectedPort === port.name
                    ? 'bg-primary/10 border-primary/30 ring-1 ring-primary/20'
                    : 'bg-slate-50 dark:bg-[#0f1821] border-slate-200 dark:border-[#334a5e] hover:border-primary/30'}"
                >
                  <div
                    class="p-2 rounded-lg {selectedPort === port.name
                      ? 'bg-primary/20'
                      : 'bg-slate-100 dark:bg-[#1a2632]'}"
                  >
                    <Icon
                      name="usb"
                      size="sm"
                      class={selectedPort === port.name ? "text-primary" : "text-slate-400"}
                    />
                  </div>
                  <div class="flex-1">
                    <div class="flex items-center gap-2">
                      <span
                        class="font-mono font-bold text-sm {selectedPort === port.name
                          ? 'text-primary'
                          : 'text-slate-900 dark:text-white'}"
                      >
                        {port.name}
                      </span>
                      {#if selectedPort === port.name}
                        <span class="size-2 bg-primary rounded-lg animate-pulse"></span>
                      {/if}
                    </div>
                    <span class="text-xs text-slate-500">{port.description}</span>
                  </div>
                </button>
              {/each}
            </div>
          {/if}
        </div>
      </div>
    </div>
  </div>

  <!-- Previous Sessions Card -->
  <div
    class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl shadow-sm overflow-hidden"
  >
    <!-- Card Header -->
    <div
      class="bg-gradient-to-r from-blue-500/10 to-violet-500/10 dark:from-blue-500/20 dark:to-violet-500/20 px-6 py-4 border-b border-slate-200 dark:border-[#334a5e]"
    >
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-3">
          <div class="p-2 bg-blue-500/10 rounded-lg">
            <Icon name="history" class="text-blue-500" />
          </div>
          <div>
            <h3 class="text-lg font-bold text-slate-900 dark:text-white">{$t.previousSessions}</h3>
            <p class="text-xs text-slate-500 dark:text-slate-400">
              {previousSessions.length} {$t.available}
            </p>
          </div>
        </div>
        <button
          onclick={() => navigationStore.navigate("sessions")}
          class="text-primary text-sm font-bold hover:underline"
        >{$t.viewAll}</button>
      </div>
    </div>

    <!-- Card Body -->
    <div class="p-4">
      {#if previousSessions.length === 0}
        <div class="p-8 text-center">
          <Icon name="folder_off" class="text-slate-300 dark:text-slate-600 text-4xl mb-2" />
          <p class="text-sm text-slate-500">{$t.noPreviousSessions}</p>
        </div>
      {:else}
        <div class="space-y-2">
          {#each previousSessions.slice(0, 4) as session}
            <div class="flex items-center gap-2 group">
              <button
                onclick={() => loadSession(session)}
                class="flex-1 flex items-center gap-4 p-3 rounded-lg border border-slate-200 dark:border-[#334a5e] hover:bg-slate-50 dark:hover:bg-[#1a2632] transition-all text-left"
              >
                <!-- Meter Flag Badge -->
                <div
                  class="w-10 h-10 flex items-center justify-center rounded-lg font-mono font-bold text-xs
                    {session.success
                    ? 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400'
                    : 'bg-red-500/10 text-red-600 dark:text-red-400'}"
                >
                  {session.flag}
                </div>

                <!-- Session Info -->
                <div class="flex-1 min-w-0">
                  <div class="flex items-center justify-between gap-2">
                    <span class="font-mono font-bold text-sm text-slate-900 dark:text-white truncate">
                      {session.serialNumber}
                    </span>
                    <span class="text-xs text-slate-500 whitespace-nowrap">{session.dateTime}</span>
                  </div>
                  {#if session.note}
                    <span
                      class="text-xs text-slate-400 dark:text-slate-500 truncate block"
                      title={session.note}
                    >
                      {session.note.length > 50 ? session.note.slice(0, 50) + "..." : session.note}
                    </span>
                  {/if}
                </div>

                <!-- Arrow Icon -->
                <Icon
                  name="chevron_right"
                  size="sm"
                  class="text-slate-300 dark:text-slate-600 group-hover:text-primary transition-colors"
                />
              </button>

              <!-- Delete Button -->
              <button
                onclick={(e) => promptDeleteSession(session, e)}
                class="p-2 rounded-lg text-slate-400 hover:text-red-500 hover:bg-red-50 dark:hover:bg-red-500/10 transition-colors opacity-0 group-hover:opacity-100"
                title={$t.delete}
              >
                <Icon name="delete" size="sm" />
              </button>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  </div>
</div>

<!-- Session Load Confirmation Dialog -->
{#if showLoadConfirmDialog && pendingSessionToLoad}
  <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50" onclick={cancelLoadSession}>
    <div
      class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl shadow-xl w-full max-w-md mx-4"
      onclick={(e) => e.stopPropagation()}
    >
      <!-- Dialog Header -->
      <div class="flex items-center gap-3 px-5 py-4 border-b border-slate-200 dark:border-[#334a5e]">
        <div class="p-2 bg-amber-500/10 rounded-lg">
          <Icon name="warning" class="text-amber-500" />
        </div>
        <h3 class="text-lg font-bold text-slate-900 dark:text-white">{$t.loadSessionConfirmTitle || "Load Session?"}</h3>
      </div>

      <!-- Dialog Body -->
      <div class="p-5 space-y-4">
        <p class="text-sm text-slate-600 dark:text-slate-400">
          {$t.loadSessionConfirmMessage || "There is existing meter data. Loading this session will replace the current data."}
        </p>

        <!-- Session to load info -->
        <div class="p-3 bg-slate-50 dark:bg-[#0f1821] rounded-lg">
          <p class="text-xs text-slate-500 mb-1">{$t.sessionToLoad || "Session to load"}</p>
          <p class="text-sm font-bold text-slate-900 dark:text-white">
            {pendingSessionToLoad.flag} — {pendingSessionToLoad.serialNumber}
          </p>
          <p class="text-xs text-slate-500">{pendingSessionToLoad.dateTime}</p>
        </div>
      </div>

      <!-- Dialog Footer -->
      <div class="flex justify-end gap-3 px-5 py-4 border-t border-slate-200 dark:border-[#334a5e]">
        <button
          onclick={cancelLoadSession}
          class="px-4 py-2 text-sm font-bold text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-[#334a5e] rounded-lg transition-colors"
        >
          {$t.cancel}
        </button>
        <button
          onclick={confirmLoadSession}
          class="flex items-center gap-2 px-4 py-2 bg-primary hover:bg-primary/90 text-white text-sm font-bold rounded-lg transition-colors"
        >
          <Icon name="folder_open" size="sm" />
          {$t.loadSession || "Load Session"}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Loading Overlay -->
{#if isLoadingSession}
  <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
    <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl shadow-xl p-8 text-center">
      <Icon name="sync" class="text-primary text-4xl mb-3 animate-spin" />
      <p class="text-sm font-bold text-slate-900 dark:text-white">{$t.loadingSession || "Loading session..."}</p>
    </div>
  </div>
{/if}

<!-- Delete Confirmation Dialog -->
{#if showDeleteConfirmDialog && pendingSessionToDelete}
  <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50" onclick={cancelDeleteSession}>
    <div
      class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl shadow-xl w-full max-w-md mx-4"
      onclick={(e) => e.stopPropagation()}
    >
      <!-- Dialog Header -->
      <div class="flex items-center gap-3 px-5 py-4 border-b border-slate-200 dark:border-[#334a5e]">
        <div class="p-2 bg-red-500/10 rounded-lg">
          <Icon name="delete" class="text-red-500" />
        </div>
        <h3 class="text-lg font-bold text-slate-900 dark:text-white">{$t.deleteSessionConfirmTitle || "Delete Session?"}</h3>
      </div>

      <!-- Dialog Body -->
      <div class="p-5 space-y-4">
        <p class="text-sm text-slate-600 dark:text-slate-400">
          {$t.deleteSessionConfirmMessage || "This action cannot be undone. The session file will be permanently deleted."}
        </p>

        <!-- Session to delete info -->
        <div class="p-3 bg-red-50 dark:bg-red-500/10 border border-red-200 dark:border-red-500/20 rounded-lg">
          <p class="text-xs text-red-500 mb-1">{$t.sessionToDelete || "Session to delete"}</p>
          <p class="text-sm font-bold text-slate-900 dark:text-white">
            {pendingSessionToDelete.flag} — {pendingSessionToDelete.serialNumber}
          </p>
          <p class="text-xs text-slate-500">{pendingSessionToDelete.dateTime}</p>
        </div>
      </div>

      <!-- Dialog Footer -->
      <div class="flex justify-end gap-3 px-5 py-4 border-t border-slate-200 dark:border-[#334a5e]">
        <button
          onclick={cancelDeleteSession}
          class="px-4 py-2 text-sm font-bold text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-[#334a5e] rounded-lg transition-colors"
        >
          {$t.cancel}
        </button>
        <button
          onclick={confirmDeleteSession}
          disabled={isDeletingSession}
          class="flex items-center gap-2 px-4 py-2 bg-red-500 hover:bg-red-600 text-white text-sm font-bold rounded-lg transition-colors disabled:opacity-50"
        >
          {#if isDeletingSession}
            <Icon name="sync" size="sm" class="animate-spin" />
          {:else}
            <Icon name="delete" size="sm" />
          {/if}
          {$t.delete}
        </button>
      </div>
    </div>
  </div>
{/if}
