<script lang="ts">
  import Icon from "$lib/components/common/Icon.svelte";
  import { t, isConnected, meterStore, addLog } from "$lib/stores";

  let meterTime = $state("--:--:--");
  let meterDate = $state("----/--/--");
  let computerTime = $state(new Date().toLocaleTimeString("en-GB"));
  let computerDate = $state(new Date().toISOString().split("T")[0]);
  let isSyncing = $state(false);
  let lastSyncTime = $state<string | null>(null);

  // Calculate time drift in seconds
  let timeDrift = $derived.by(() => {
    if (!$meterStore.shortReadData) return 0;

    try {
      const meterDateTime = new Date(`${$meterStore.shortReadData.meterDate}T${$meterStore.shortReadData.meterTime}`);
      const now = new Date();
      return Math.round((now.getTime() - meterDateTime.getTime()) / 1000);
    } catch {
      return 0;
    }
  });

  let driftWarning = $derived(Math.abs(timeDrift) > 30);

  // Update from meter store
  $effect(() => {
    if ($meterStore.shortReadData) {
      meterTime = $meterStore.shortReadData.meterTime || "--:--:--";
      meterDate = $meterStore.shortReadData.meterDate || "----/--/--";
    }
  });

  // Update computer time every second
  $effect(() => {
    const interval = setInterval(() => {
      const now = new Date();
      computerTime = now.toLocaleTimeString("en-GB");
      computerDate = now.toISOString().split("T")[0];
    }, 1000);

    return () => clearInterval(interval);
  });

  async function syncToComputer() {
    if (!$isConnected || isSyncing) return;

    isSyncing = true;
    addLog("info", "Saat senkronizasyonu baslatiliyor...");

    try {
      // TODO: Call actual Tauri command to sync time
      // await syncTime();

      // Simulate sync delay
      await new Promise(r => setTimeout(r, 1500));

      lastSyncTime = new Date().toLocaleString("tr-TR");
      addLog("success", "Saat basariyla senkronize edildi!");
    } catch (error) {
      addLog("error", `Senkronizasyon hatasi: ${error}`);
    } finally {
      isSyncing = false;
    }
  }

  function formatDrift(seconds: number): string {
    const sign = seconds >= 0 ? "+" : "";
    if (Math.abs(seconds) < 60) return `${sign}${seconds}s`;
    const mins = Math.floor(Math.abs(seconds) / 60);
    const secs = Math.abs(seconds) % 60;
    return `${sign}${mins}m ${secs}s`;
  }
</script>

<div class="space-y-6">
  <!-- Header -->
  <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm">
    <div class="flex items-start justify-between">
      <div>
        <h3 class="text-xl font-bold text-slate-900 dark:text-white mb-2">{$t.timeDate}</h3>
        <p class="text-sm text-slate-500 dark:text-slate-400">
          Sayac saatini bilgisayar saati ile senkronize edin veya manuel ayarlayin.
        </p>
      </div>

      {#if lastSyncTime}
        <div class="flex items-center gap-2 px-3 py-2 bg-emerald-500/10 border border-emerald-500/20 rounded-lg">
          <Icon name="check_circle" class="text-emerald-500" size="sm" />
          <div class="text-xs">
            <div class="text-slate-500">{$t.lastSyncTime}</div>
            <div class="font-mono text-emerald-600">{lastSyncTime}</div>
          </div>
        </div>
      {/if}
    </div>

    {#if !$isConnected}
      <div class="mt-4 p-4 bg-amber-500/10 border border-amber-500/20 rounded-xl text-amber-600 dark:text-amber-500 text-sm">
        <div class="flex items-center gap-2">
          <Icon name="warning" />
          <span>{$t.connectFirstWarning}</span>
        </div>
      </div>
    {/if}
  </div>

  <!-- Time Display -->
  <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
    <!-- Meter Time -->
    <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm">
      <h4 class="font-bold text-slate-900 dark:text-white mb-4 flex items-center gap-2">
        <Icon name="electric_meter" class="text-primary" />
        {$t.currentMeterTime}
      </h4>
      <div class="text-center py-6">
        <div class="text-5xl font-mono font-bold text-slate-900 dark:text-white mb-2">
          {meterTime}
        </div>
        <div class="text-xl font-mono text-slate-500">{meterDate}</div>
      </div>
    </div>

    <!-- Computer Time -->
    <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm">
      <h4 class="font-bold text-slate-900 dark:text-white mb-4 flex items-center gap-2">
        <Icon name="computer" class="text-primary" />
        {$t.computerTime}
      </h4>
      <div class="text-center py-6">
        <div class="text-5xl font-mono font-bold text-primary mb-2">{computerTime}</div>
        <div class="text-xl font-mono text-slate-500">{computerDate}</div>
      </div>
    </div>
  </div>

  <!-- Time Drift Indicator -->
  <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm">
    <div class="flex items-center justify-between">
      <div class="flex items-center gap-4">
        <div class="p-3 rounded-xl {driftWarning ? 'bg-amber-500/10' : 'bg-emerald-500/10'}">
          <Icon
            name={driftWarning ? "sync_problem" : "check_circle"}
            class="{driftWarning ? 'text-amber-500' : 'text-emerald-500'} text-2xl"
          />
        </div>
        <div>
          <h4 class="font-bold text-slate-900 dark:text-white">{$t.timeDriftStatus}</h4>
          <p class="text-sm text-slate-500">Sayac ve bilgisayar saati arasindaki fark</p>
        </div>
      </div>
      <div class="text-right">
        <div class="text-3xl font-mono font-bold {driftWarning ? 'text-amber-500' : 'text-emerald-500'}">
          {formatDrift(timeDrift)}
        </div>
        {#if driftWarning}
          <div class="text-xs text-amber-500 mt-1 flex items-center gap-1">
            <Icon name="warning" size="sm" />
            {$t.driftWarning}
          </div>
        {/if}
      </div>
    </div>
  </div>

  <!-- Sync Options -->
  <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm">
    <h4 class="font-bold text-slate-900 dark:text-white mb-4">Senkronizasyon Secenekleri</h4>
    <div class="flex flex-col md:flex-row gap-4">
      <button
        onclick={syncToComputer}
        disabled={!$isConnected || isSyncing}
        class="flex-1 flex items-center justify-center gap-2 px-6 py-4 bg-primary hover:bg-primary/90 text-white font-bold rounded-xl shadow-lg shadow-primary/20 transition-all disabled:opacity-50 disabled:cursor-not-allowed"
      >
        {#if isSyncing}
          <Icon name="sync" class="animate-spin" />
          Senkronize ediliyor...
        {:else}
          <Icon name="sync" />
          {$t.syncToComputerTime}
        {/if}
      </button>
      <button
        disabled={!$isConnected || isSyncing}
        class="flex-1 flex items-center justify-center gap-2 px-6 py-4 bg-slate-100 dark:bg-[#334a5e] hover:bg-slate-200 dark:hover:bg-[#455a6e] text-slate-700 dark:text-white font-bold rounded-xl transition-all disabled:opacity-50 disabled:cursor-not-allowed"
      >
        <Icon name="edit" />
        {$t.manualTimeEntry}
      </button>
    </div>
  </div>

  <!-- DST Status -->
  <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm">
    <div class="flex items-center justify-between">
      <div class="flex items-center gap-4">
        <div class="p-3 rounded-xl bg-amber-500/10">
          <Icon name="wb_sunny" class="text-amber-500 text-2xl" />
        </div>
        <div>
          <h4 class="font-bold text-slate-900 dark:text-white">{$t.dstStatus}</h4>
          <p class="text-sm text-slate-500">Yaz saati uygulamasi durumu</p>
        </div>
      </div>
      <div class="flex items-center gap-3">
        <span class="px-4 py-2 bg-slate-100 dark:bg-[#334a5e] rounded-lg font-bold text-slate-600 dark:text-slate-400">
          {$t.dstInactive}
        </span>
      </div>
    </div>
  </div>
</div>
