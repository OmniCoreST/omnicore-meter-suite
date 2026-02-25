<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "$lib/components/common/Icon.svelte";
  import { t, isConnected, meterStore, addLog, errorToast, successToast } from "$lib/stores";
  import { authenticate, writeObis, endSession, readObisBatch } from "$lib/utils/tauri";

  // Local state derived from store, then updated after OBIS read
  let relayStatus = $state<"active" | "passive" | "">("");
  let demandButtonActive = $state<boolean | null>(null);
  let isLoading = $state(false);
  let isActing = $state(false);

  // Password dialog state
  let showPasswordDialog = $state(false);
  let showConfirmDialog = $state(false);
  let pendingAction = $state<"open" | "close" | null>(null);
  let password = $state("");
  let passwordError = $state("");

  // Initialize relay status from store on mount
  $effect(() => {
    const storeRelay = $meterStore.shortReadData?.relayStatus ?? $meterStore.fullReadData?.relayStatus ?? "";
    if (storeRelay && !relayStatus) {
      relayStatus = storeRelay;
    }
  });

  async function loadRelayData() {
    if (!$isConnected) return;
    isLoading = true;
    try {
      const result = await readObisBatch(["96.3.10", "96.91.0"]);
      const raw3_10 = result["96.3.10"] ?? "";
      const raw91_0 = result["96.91.0"] ?? "";

      // 96.3.10: "0" → active (energy on), "1" → passive (energy off)
      const val3_10 = raw3_10.replace(/[()]/g, "").trim();
      if (val3_10 === "0") relayStatus = "active";
      else if (val3_10 === "1") relayStatus = "passive";

      // 96.91.0: "0" → disabled, "1" → enabled
      const val91_0 = raw91_0.replace(/[()]/g, "").trim();
      if (val91_0 !== "") demandButtonActive = val91_0 !== "0";

      addLog("info", `Röle durumu okundu: ${val3_10 === "0" ? "Aktif" : val3_10 === "1" ? "Pasif" : "Bilinmiyor"}`);
    } catch (error) {
      addLog("error", `Röle verisi okunamadı: ${error}`);
    } finally {
      isLoading = false;
    }
  }

  onMount(() => {
    // Use store value first
    const storeRelay = $meterStore.shortReadData?.relayStatus ?? $meterStore.fullReadData?.relayStatus ?? "";
    if (storeRelay) relayStatus = storeRelay;

    if ($isConnected) {
      loadRelayData();
    }
  });

  function handleRelay(action: "open" | "close") {
    pendingAction = action;
    password = "";
    passwordError = "";
    if (action === "close") {
      showConfirmDialog = true;
    } else {
      showPasswordDialog = true;
    }
  }

  function onConfirmDisconnect() {
    showConfirmDialog = false;
    showPasswordDialog = true;
  }

  async function executeAction() {
    if (!pendingAction) return;
    if (password.length !== 8 || !/^\d{8}$/.test(password)) {
      passwordError = "Şifre tam olarak 8 rakam olmalıdır";
      return;
    }

    showPasswordDialog = false;
    isActing = true;
    const action = pendingAction;
    addLog("info", action === "open" ? "Röle açılıyor (96.3.10=0)..." : "Röle kesiliyor (96.3.10=1)...");

    try {
      const authOk = await authenticate(password, 2); // P2 - Operator
      if (!authOk) {
        addLog("error", $t.errorWrongPassword);
        errorToast($t.errorWrongPassword);
        return;
      }

      await writeObis("96.3.10", action === "open" ? "0" : "1");
      await endSession();

      relayStatus = action === "open" ? "active" : "passive";
      const msg = action === "open" ? "Röle açıldı — enerji aktif" : "Röle kesildi — enerji kesik";
      addLog("success", msg);
      successToast(msg);
    } catch (error) {
      addLog("error", `Röle işlemi başarısız: ${error}`);
      errorToast(`Röle işlemi başarısız: ${error}`);
    } finally {
      isActing = false;
      pendingAction = null;
    }
  }
</script>

<div class="space-y-6">
  <!-- Header -->
  <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm">
    <div class="flex items-start justify-between">
      <div>
        <h3 class="text-xl font-bold text-slate-900 dark:text-white mb-2">{$t.relayControl}</h3>
        <p class="text-sm text-slate-500 dark:text-slate-400">{$t.relayControlDescription}</p>
      </div>
      <button
        onclick={loadRelayData}
        disabled={!$isConnected || isLoading || isActing}
        class="flex items-center gap-2 px-4 py-2.5 bg-slate-100 dark:bg-[#334a5e] hover:bg-slate-200 dark:hover:bg-[#455a6e] text-slate-700 dark:text-white font-bold rounded-xl transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
      >
        <Icon name="refresh" class={isLoading ? "animate-spin" : ""} />
        {$t.refresh}
      </button>
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

  <!-- Large Status Indicator -->
  <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-8 shadow-sm">
    <div class="flex flex-col items-center">
      {#if isLoading}
        <div class="w-48 h-48 rounded-full flex items-center justify-center mb-6 bg-slate-100 dark:bg-[#1a2632]">
          <Icon name="sync" class="text-slate-400 text-7xl animate-spin" />
        </div>
        <div class="text-center">
          <h2 class="text-2xl font-bold text-slate-400 mb-2">{$t.relayState}</h2>
          <p class="text-slate-400 text-sm">Okunuyor...</p>
        </div>
      {:else if relayStatus === "active"}
        <div class="w-48 h-48 rounded-full flex items-center justify-center mb-6 bg-gradient-to-br from-emerald-400 to-emerald-600 shadow-2xl shadow-emerald-500/30">
          <Icon name="power" class="text-white text-7xl" />
        </div>
        <div class="text-center">
          <h2 class="text-3xl font-bold text-emerald-500 mb-2">{$t.relayCurrentState}</h2>
          <p class="text-emerald-600 dark:text-emerald-400 font-bold text-lg">{$t.relayConnected}</p>
        </div>
      {:else if relayStatus === "passive"}
        <div class="w-48 h-48 rounded-full flex items-center justify-center mb-6 bg-gradient-to-br from-red-400 to-red-600 shadow-2xl shadow-red-500/30">
          <Icon name="power_off" class="text-white text-7xl" />
        </div>
        <div class="text-center">
          <h2 class="text-3xl font-bold text-red-500 mb-2">{$t.relayCurrentState}</h2>
          <p class="text-red-600 dark:text-red-400 font-bold text-lg">{$t.relayDisconnected}</p>
        </div>
      {:else}
        <div class="w-48 h-48 rounded-full flex items-center justify-center mb-6 bg-gradient-to-br from-slate-300 to-slate-400 dark:from-slate-600 dark:to-slate-700 shadow-2xl">
          <Icon name="power" class="text-white text-7xl" />
        </div>
        <div class="text-center">
          <h2 class="text-3xl font-bold text-slate-400 mb-2">{$t.relayCurrentState}</h2>
          <p class="text-slate-400">—</p>
        </div>
      {/if}
    </div>
  </div>

  <!-- Control Buttons -->
  <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm">
    <!-- Warning -->
    <div class="flex items-start gap-2 mb-5 p-3 bg-amber-500/10 border border-amber-500/20 rounded-xl text-amber-600 dark:text-amber-400 text-sm">
      <Icon name="warning" class="mt-0.5 shrink-0" />
      <span>{$t.relayWarning}</span>
    </div>

    <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
      <!-- Enerjiyi Aç -->
      <button
        onclick={() => handleRelay("open")}
        disabled={!$isConnected || isActing || isLoading || relayStatus === "active"}
        class="flex items-center justify-center gap-3 px-6 py-6 rounded-xl font-bold text-lg transition-all
          {!$isConnected || isActing || isLoading || relayStatus === 'active'
            ? 'bg-slate-100 dark:bg-[#334a5e] text-slate-400 cursor-not-allowed opacity-50'
            : 'bg-emerald-500 hover:bg-emerald-600 text-white shadow-lg shadow-emerald-500/20'}"
      >
        {#if isActing && pendingAction === "open"}
          <Icon name="sync" class="text-2xl animate-spin" />
        {:else}
          <Icon name="power" class="text-2xl" />
        {/if}
        {$t.connectRelay}
      </button>

      <!-- Enerjiyi Kes -->
      <button
        onclick={() => handleRelay("close")}
        disabled={!$isConnected || isActing || isLoading || relayStatus === "passive"}
        class="flex items-center justify-center gap-3 px-6 py-6 rounded-xl font-bold text-lg transition-all
          {!$isConnected || isActing || isLoading || relayStatus === 'passive'
            ? 'bg-slate-100 dark:bg-[#334a5e] text-slate-400 cursor-not-allowed opacity-50'
            : 'bg-red-500 hover:bg-red-600 text-white shadow-lg shadow-red-500/20'}"
      >
        {#if isActing && pendingAction === "close"}
          <Icon name="sync" class="text-2xl animate-spin" />
        {:else}
          <Icon name="power_off" class="text-2xl" />
        {/if}
        {$t.disconnectRelay}
      </button>
    </div>
  </div>

  <!-- Demand Button Status -->
  <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm">
    <div class="flex items-center justify-between">
      <div class="flex items-center gap-4">
        <div class="p-3 rounded-xl bg-primary/10">
          <Icon name="touch_app" class="text-primary text-2xl" />
        </div>
        <div>
          <h4 class="font-bold text-slate-900 dark:text-white">{$t.demandButtonStatus}</h4>
          <p class="text-sm text-slate-500 font-mono">96.91.0</p>
        </div>
      </div>
      <div>
        {#if demandButtonActive === null}
          <span class="text-sm text-slate-400">—</span>
        {:else if demandButtonActive}
          <span class="px-3 py-1.5 bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 text-sm font-bold rounded-lg">
            {$t.demandButtonEnabled}
          </span>
        {:else}
          <span class="px-3 py-1.5 bg-slate-100 dark:bg-[#334a5e] text-slate-500 dark:text-slate-400 text-sm font-bold rounded-lg">
            {$t.demandButtonDisabled}
          </span>
        {/if}
      </div>
    </div>
  </div>
</div>

<!-- Confirm Disconnect Dialog -->
{#if showConfirmDialog}
  <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50" role="dialog">
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="absolute inset-0" onclick={() => { showConfirmDialog = false; pendingAction = null; }}></div>
    <div class="relative bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-2xl p-6 w-full max-w-sm shadow-2xl">
      <div class="flex items-center gap-3 mb-4">
        <div class="p-3 rounded-xl bg-red-500/10">
          <Icon name="warning" class="text-red-500 text-2xl" />
        </div>
        <h3 class="text-lg font-bold text-slate-900 dark:text-white">Enerji Kesme Onayı</h3>
      </div>
      <p class="text-sm text-slate-600 dark:text-slate-300 mb-6">{$t.confirmDisconnect}</p>
      <div class="flex gap-3">
        <button
          onclick={() => { showConfirmDialog = false; pendingAction = null; }}
          class="flex-1 px-4 py-3 bg-slate-100 dark:bg-[#334a5e] hover:bg-slate-200 dark:hover:bg-[#455a6e] text-slate-700 dark:text-white font-bold rounded-xl transition-colors"
        >
          {$t.cancel}
        </button>
        <button
          onclick={onConfirmDisconnect}
          class="flex-1 px-4 py-3 bg-red-500 hover:bg-red-600 text-white font-bold rounded-xl shadow-lg shadow-red-500/20 transition-all"
        >
          <Icon name="power_off" size="sm" class="inline mr-1" />
          {$t.yes}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Password Dialog -->
{#if showPasswordDialog}
  <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50" role="dialog">
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="absolute inset-0" onclick={() => { showPasswordDialog = false; pendingAction = null; }}></div>
    <div class="relative bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-2xl p-6 w-full max-w-sm shadow-2xl">
      <h3 class="text-lg font-bold text-slate-900 dark:text-white mb-1">
        {pendingAction === "open" ? $t.connectRelay : $t.disconnectRelay}
      </h3>
      <div class="flex items-center gap-2 mb-3">
        <span class="px-2 py-0.5 bg-amber-500/10 text-amber-600 dark:text-amber-400 text-xs font-bold rounded">P2 - Operator</span>
        <span class="text-xs text-slate-400">OBIS: 96.3.10</span>
      </div>
      <p class="text-sm text-slate-500 mb-4">{$t.passwordWarning}</p>

      <div class="mb-4">
        <label class="block text-sm font-bold text-slate-700 dark:text-slate-300 mb-2" for="relay-password">
          P2 {$t.password}
        </label>
        <input
          id="relay-password"
          type="password"
          maxlength={8}
          bind:value={password}
          oninput={(e) => { const inp = e.currentTarget; inp.value = inp.value.replace(/\D/g, ''); password = inp.value; }}
          onkeydown={(e) => { if (e.key === "Enter") executeAction(); }}
          placeholder="00000000"
          class="w-full px-4 py-3 bg-white dark:bg-[#1a2632] border border-slate-200 dark:border-[#334a5e] rounded-xl text-center font-mono text-lg tracking-[0.3em] focus:border-primary focus:ring-1 focus:ring-primary outline-none"
        />
        {#if passwordError}
          <p class="text-xs text-red-500 mt-2">{passwordError}</p>
        {/if}
      </div>

      <div class="flex gap-3">
        <button
          onclick={() => { showPasswordDialog = false; pendingAction = null; }}
          class="flex-1 px-4 py-3 bg-slate-100 dark:bg-[#334a5e] hover:bg-slate-200 dark:hover:bg-[#455a6e] text-slate-700 dark:text-white font-bold rounded-xl transition-colors"
        >
          {$t.cancel}
        </button>
        <button
          onclick={executeAction}
          disabled={password.length !== 8}
          class="flex-1 px-4 py-3 font-bold rounded-xl shadow-lg transition-all disabled:opacity-50 disabled:cursor-not-allowed
            {pendingAction === 'close'
              ? 'bg-red-500 hover:bg-red-600 text-white shadow-red-500/20'
              : 'bg-emerald-500 hover:bg-emerald-600 text-white shadow-emerald-500/20'}"
        >
          <Icon name={pendingAction === "close" ? "power_off" : "power"} size="sm" class="inline mr-1" />
          {$t.confirm}
        </button>
      </div>
    </div>
  </div>
{/if}
