<script lang="ts">
  import Icon from "$lib/components/common/Icon.svelte";
  import { t, connectionStore, isConnected, isConnecting, addLog } from "$lib/stores";

  // Connection parameters
  let connectionType = $state("auto");
  let selectedPort = $state("COM3");
  let baudRate = $state("auto");

  // Mock serial ports - in real app, this comes from Tauri backend
  const serialPorts = [
    { name: "COM3", description: "USB Serial", active: true },
    { name: "COM1", description: "Communications Port", active: false },
    { name: "COM4", description: "USB Serial", active: false },
  ];

  // Mock previous sessions
  const previousSessions = [
    {
      id: 1,
      flag: "MKS",
      serialNumber: "882190345",
      dateTime: "2024-12-15 14:30",
      success: true,
    },
    {
      id: 2,
      flag: "ADM",
      serialNumber: "776234891",
      dateTime: "2024-12-14 09:15",
      success: true,
    },
    {
      id: 3,
      flag: "MKS",
      serialNumber: "445678123",
      dateTime: "2024-12-13 16:45",
      success: false,
    },
    {
      id: 4,
      flag: "GDZ",
      serialNumber: "998877665",
      dateTime: "2024-12-12 11:20",
      success: true,
    },
  ];

  const connectionTypes = [
    { value: "auto", labelKey: "autoDetect" as const },
    { value: "optical", labelKey: "opticalProbe" as const },
    { value: "rs485", labelKey: "rs485Direct" as const },
  ];

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
      connectionStore.disconnect();
      addLog("info", "Bağlantı kesildi");
    } else {
      connectionStore.setConnecting(true);
      addLog("info", `${selectedPort} portuna bağlanılıyor...`);

      // Simulate connection
      await new Promise((r) => setTimeout(r, 1500));

      connectionStore.connect({
        port: selectedPort,
        baudRate: parseInt(baudRate),
        connectionType: connectionType as "optical" | "rs485",
      });

      connectionStore.setMeterIdentity({
        flag: "MKS",
        manufacturer: "MAKEL",
        baudChar: "6",
        edasId: "ADM",
        model: "M550.2251",
        serialNumber: "882190345",
      });

      addLog("success", "Bağlantı başarılı: MKS — M550.2251");
    }
  }

  function loadSession(session: (typeof previousSessions)[0]) {
    addLog("info", `Oturum yükleniyor: ${session.flag} — ${session.serialNumber}`);
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
              class="w-full px-3 py-2.5 bg-white dark:bg-[#1a2632] border border-slate-300 dark:border-[#334a5e] rounded-lg text-sm text-slate-900 dark:text-white focus:border-primary focus:ring-1 focus:ring-primary transition-colors"
            >
              {#each baudRates as rate}
                <option value={rate.value}>{rate.label || $t.autoDetect}</option>
              {/each}
            </select>
          </div>

          <!-- Connect Button -->
          <button
            onclick={connect}
            disabled={hasNoPorts || $isConnecting}
            class="w-full mt-2 flex items-center justify-center gap-2 px-6 py-3 bg-primary hover:bg-primary/90 disabled:bg-slate-300 dark:disabled:bg-slate-700 text-white font-bold rounded-lg shadow-lg shadow-primary/20 transition-all disabled:cursor-not-allowed"
          >
            {#if $isConnecting}
              <Icon name="sync" class="animate-spin" />
              {$t.reading}
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

          {#if hasNoPorts}
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
        <button class="text-primary text-sm font-bold hover:underline">{$t.viewAll}</button>
      </div>
    </div>

    <!-- Card Body -->
    <div class="p-4">
      {#if previousSessions.length === 0}
        <div class="p-8 text-center">
          <Icon name="folder_off" class="text-slate-300 dark:text-slate-600 text-4xl mb-2" />
          <p class="text-sm text-slate-500">{$t.noRecentEvents}</p>
        </div>
      {:else}
        <div class="space-y-2">
          {#each previousSessions as session}
            <button
              onclick={() => loadSession(session)}
              class="w-full flex items-center gap-4 p-3 rounded-lg border border-slate-200 dark:border-[#334a5e] hover:bg-slate-50 dark:hover:bg-[#1a2632] transition-all text-left group"
            >
              <!-- Meter Flag Badge -->
              <div
                class="w-12 h-12 flex items-center justify-center rounded-lg font-mono font-bold text-sm
                  {session.success
                  ? 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400'
                  : 'bg-red-500/10 text-red-600 dark:text-red-400'}"
              >
                {session.flag}
              </div>

              <!-- Session Info -->
              <div class="flex-1">
                <div class="flex items-center gap-2">
                  <span class="font-mono font-bold text-slate-900 dark:text-white">
                    {session.serialNumber}
                  </span>
                  {#if !session.success}
                    <Icon name="error" size="sm" class="text-red-500" />
                  {/if}
                </div>
                <span class="text-xs text-slate-500">{session.dateTime}</span>
              </div>

              <!-- Arrow Icon -->
              <Icon
                name="chevron_right"
                class="text-slate-300 dark:text-slate-600 group-hover:text-primary transition-colors"
              />
            </button>
          {/each}
        </div>
      {/if}
    </div>
  </div>
</div>
