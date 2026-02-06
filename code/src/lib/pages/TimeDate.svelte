<script lang="ts">
  import Icon from "$lib/components/common/Icon.svelte";
  import { t, isConnected, meterStore, addLog } from "$lib/stores";
  import { readShort, authenticate, syncTime, endSession } from "$lib/utils/tauri";

  let meterTime = $state("--:--:--");
  let meterDate = $state("----/--/--");
  let computerTime = $state(new Date().toLocaleTimeString("en-GB"));
  let computerDate = $state(new Date().toISOString().split("T")[0]);
  let isSyncing = $state(false);
  let isRefreshing = $state(false);
  let lastSyncTime = $state<string | null>(null);

  // Password dialog
  let showPasswordDialog = $state(false);
  let password = $state("");
  let passwordError = $state("");

  // Convert YY-MM-DD to YYYY-MM-DD (meter returns 2-digit year)
  function toFullYear(d: string): string {
    if (d.length === 8 && d[2] === "-") return `20${d}`;
    return d;
  }

  // Calculate time drift in seconds
  let timeDrift = $derived.by(() => {
    if (!$meterStore.shortReadData) return 0;

    try {
      const data = $meterStore.shortReadData;
      const fullDate = toFullYear(data.meterDate);
      const meterDateTime = new Date(`${fullDate}T${data.meterTime}`);
      if (isNaN(meterDateTime.getTime())) return 0;
      const referenceTime = data.timeOf09xRead || Date.now();
      return Math.round((referenceTime - meterDateTime.getTime()) / 1000);
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

  async function handleRefresh() {
    if (!$isConnected || isRefreshing) return;
    isRefreshing = true;
    addLog("info", $t.reading);
    try {
      const result = await readShort();
      const currentState = $meterStore;
      meterStore.setShortReadData(result, currentState.meterType, currentState.isBidirectional);
      addLog("success", $t.readComplete);
    } catch (error) {
      addLog("error", `${$t.logError}: ${error}`);
    } finally {
      isRefreshing = false;
    }
  }

  function openSyncDialog() {
    password = "";
    passwordError = "";
    showPasswordDialog = true;
  }

  async function handleSync() {
    if (password.length !== 8 || !/^\d{8}$/.test(password)) {
      passwordError = $t.passwordMustBe8Digits;
      return;
    }

    showPasswordDialog = false;
    isSyncing = true;
    addLog("info", $t.syncing);

    try {
      const authOk = await authenticate(password);
      if (!authOk) {
        addLog("error", $t.errorWrongPassword.replace("{0}", "?"));
        return;
      }

      await syncTime();
      lastSyncTime = new Date().toLocaleString("tr-TR");
      addLog("success", $t.syncComplete);

      await endSession();
    } catch (error) {
      addLog("error", `${$t.logError}: ${error}`);
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
          {$t.timeDateDescription}
        </p>
      </div>

      <div class="flex items-center gap-2">
        {#if lastSyncTime}
          <div class="flex items-center gap-2 px-3 py-2 bg-emerald-500/10 border border-emerald-500/20 rounded-lg">
            <Icon name="check_circle" class="text-emerald-500" size="sm" />
            <div class="text-xs">
              <div class="text-slate-500">{$t.lastSyncTime}</div>
              <div class="font-mono text-emerald-600">{lastSyncTime}</div>
            </div>
          </div>
        {/if}
        <button
          onclick={handleRefresh}
          disabled={!$isConnected || isRefreshing}
          class="flex items-center gap-2 px-4 py-2 bg-slate-100 dark:bg-[#334a5e] hover:bg-slate-200 dark:hover:bg-[#455a6e] text-slate-700 dark:text-white text-sm font-bold rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
        >
          <Icon name="refresh" size="sm" class={isRefreshing ? "animate-spin" : ""} />
          {$t.refresh}
        </button>
      </div>
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

  <!-- Time Drift Indicator + Sync -->
  <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm">
    <div class="flex items-center justify-between mb-4">
      <div class="flex items-center gap-4">
        <div class="p-3 rounded-xl {driftWarning ? 'bg-amber-500/10' : 'bg-emerald-500/10'}">
          <Icon
            name={driftWarning ? "sync_problem" : "check_circle"}
            class="{driftWarning ? 'text-amber-500' : 'text-emerald-500'} text-2xl"
          />
        </div>
        <div>
          <h4 class="font-bold text-slate-900 dark:text-white">{$t.timeDriftStatus}</h4>
          <p class="text-sm text-slate-500">{$t.timeDriftDescription}</p>
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
    <button
      onclick={openSyncDialog}
      disabled={!$isConnected || isSyncing}
      class="w-full flex items-center justify-center gap-2 px-6 py-3 bg-primary hover:bg-primary/90 text-white font-bold rounded-xl shadow-lg shadow-primary/20 transition-all disabled:opacity-50 disabled:cursor-not-allowed"
    >
      {#if isSyncing}
        <Icon name="sync" class="animate-spin" />
        {$t.syncing}
      {:else}
        <Icon name="sync" />
        {$t.syncToComputerTime}
      {/if}
    </button>
  </div>
</div>

<!-- Password Dialog -->
{#if showPasswordDialog}
  <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50" role="dialog">
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="absolute inset-0" onclick={() => showPasswordDialog = false}></div>
    <div class="relative bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-2xl p-6 w-full max-w-sm shadow-2xl">
      <h3 class="text-lg font-bold text-slate-900 dark:text-white mb-1">{$t.syncToComputerTime}</h3>
      <p class="text-sm text-slate-500 mb-4">{$t.passwordWarning}</p>

      <div class="mb-4">
        <label class="block text-sm font-bold text-slate-700 dark:text-slate-300 mb-2" for="sync-password">
          {$t.password}
        </label>
        <input
          id="sync-password"
          type="password"
          maxlength={8}
          bind:value={password}
          onkeydown={(e) => { if (e.key === "Enter") handleSync(); }}
          placeholder="00000000"
          class="w-full px-4 py-3 bg-white dark:bg-[#1a2632] border border-slate-200 dark:border-[#334a5e] rounded-xl text-center font-mono text-lg tracking-[0.3em] focus:border-primary focus:ring-1 focus:ring-primary outline-none"
        />
        {#if passwordError}
          <p class="text-xs text-red-500 mt-2">{passwordError}</p>
        {/if}
      </div>

      <div class="flex gap-3">
        <button
          onclick={() => showPasswordDialog = false}
          class="flex-1 px-4 py-3 bg-slate-100 dark:bg-[#334a5e] hover:bg-slate-200 dark:hover:bg-[#455a6e] text-slate-700 dark:text-white font-bold rounded-xl transition-colors"
        >
          {$t.cancel}
        </button>
        <button
          onclick={handleSync}
          disabled={password.length !== 8}
          class="flex-1 px-4 py-3 bg-primary hover:bg-primary/90 text-white font-bold rounded-xl shadow-lg shadow-primary/20 transition-all disabled:opacity-50 disabled:cursor-not-allowed"
        >
          <Icon name="sync" size="sm" class="inline mr-1" />
          {$t.syncToComputerTime}
        </button>
      </div>
    </div>
  </div>
{/if}
