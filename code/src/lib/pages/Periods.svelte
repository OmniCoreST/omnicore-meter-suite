<script lang="ts">
  import Icon from "$lib/components/common/Icon.svelte";
  import { t, isConnected, meterStore, addLog, errorToast, successToast } from "$lib/stores";
  import { authenticate, writeObis, endSession } from "$lib/utils/tauri";

  // Parse period values from raw meter data
  let periodData = $derived.by(() => {
    // @ts-ignore
    const raw: string | null = $meterStore.fullReadData?.rawData || $meterStore.shortReadData?.rawData || null;
    if (!raw) return null;

    const demandMatch = raw.match(/0\.8\.0\((\d+)\*min\)/);
    const loadProfileMatch = raw.match(/0\.8\.4\((\d+)\*min\)/);
    // 0.9.9 is not in standard readout packets
    const outageMatch = raw.match(/0\.9\.9\((\d+)\)/);

    const hasAny = demandMatch || loadProfileMatch || outageMatch;
    if (!hasAny) return null;

    return {
      demandPeriod: demandMatch ? parseInt(demandMatch[1]) : null,
      loadProfilePeriod: loadProfileMatch ? parseInt(loadProfileMatch[1]) : null,
      outageThreshold: outageMatch ? parseInt(outageMatch[1]) : null,
    };
  });

  // Editable state
  let editDemandPeriod = $state(15);
  let editLoadProfilePeriod = $state(15);
  let editOutageThreshold = $state(180);
  let initialized = $state(false);

  // Initialize from meter data
  $effect(() => {
    if (periodData && !initialized) {
      if (periodData.demandPeriod !== null) editDemandPeriod = periodData.demandPeriod;
      if (periodData.loadProfilePeriod !== null) editLoadProfilePeriod = periodData.loadProfilePeriod;
      if (periodData.outageThreshold !== null) editOutageThreshold = periodData.outageThreshold;
      initialized = true;
    }
  });

  // Password dialog state
  let showPasswordDialog = $state(false);
  let password = $state("");
  let passwordError = $state("");
  let isSaving = $state(false);
  let pendingAction = $state<"save" | "reset">("save");

  // Confirm dialog for demand reset
  let showResetConfirm = $state(false);

  function openSaveDialog() {
    pendingAction = "save";
    password = "";
    passwordError = "";
    showPasswordDialog = true;
  }

  function openResetDialog() {
    showResetConfirm = true;
  }

  function confirmReset() {
    showResetConfirm = false;
    pendingAction = "reset";
    password = "";
    passwordError = "";
    showPasswordDialog = true;
  }

  async function handlePasswordSubmit() {
    if (password.length !== 8 || !/^\d{8}$/.test(password)) {
      passwordError = $t.passwordMustBe8Digits;
      return;
    }

    showPasswordDialog = false;
    isSaving = true;

    if (pendingAction === "save") {
      await handleSave();
    } else {
      await handleDemandReset();
    }

    isSaving = false;
  }

  async function handleSave() {
    addLog("info", $t.savingPeriodSettings);

    try {
      const authOk = await authenticate(password);
      if (!authOk) {
        addLog("error", $t.errorWrongPassword);
        errorToast($t.errorWrongPassword);
        return;
      }

      await writeObis("0.8.0", `${editDemandPeriod}*min`);
      addLog("info", `0.8.0 = ${editDemandPeriod}*min`);

      await writeObis("0.8.4", `${editLoadProfilePeriod}*min`);
      addLog("info", `0.8.4 = ${editLoadProfilePeriod}*min`);

      await writeObis("0.9.9", `${editOutageThreshold}*sec`);
      addLog("info", `0.9.9 = ${editOutageThreshold}*sec`);

      await endSession();
      addLog("success", $t.periodSaveSuccess);
      successToast($t.periodSaveSuccess);
    } catch (error) {
      addLog("error", `${$t.logError}: ${error}`);
      errorToast(`${$t.logError}: ${error}`);
    }
  }

  async function handleDemandReset() {
    addLog("info", $t.demandResetExecuting);

    try {
      const authOk = await authenticate(password);
      if (!authOk) {
        addLog("error", $t.errorWrongPassword);
        errorToast($t.errorWrongPassword);
        return;
      }

      await writeObis("1.6.0", "");
      addLog("info", "1.6.0 = (reset)");

      await endSession();
      addLog("success", $t.demandResetSuccess);
      successToast($t.demandResetSuccess);
    } catch (error) {
      addLog("error", `${$t.logError}: ${error}`);
      errorToast(`${$t.logError}: ${error}`);
    }
  }
</script>

<div class="space-y-6">
  <!-- Header -->
  <div
    class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm"
  >
    <div class="flex items-start justify-between">
      <div>
        <h3 class="text-xl font-bold text-slate-900 dark:text-white mb-2">{$t.periodSettings}</h3>
        <p class="text-sm text-slate-500 dark:text-slate-400">
          {$t.periodSettingsDescription}
        </p>
      </div>
      <button
        onclick={openSaveDialog}
        disabled={!$isConnected || isSaving}
        class="flex items-center gap-2 px-5 py-2.5 bg-primary hover:bg-primary/90 text-white font-bold rounded-xl shadow-lg shadow-primary/20 transition-all disabled:opacity-50 disabled:cursor-not-allowed whitespace-nowrap"
      >
        {#if isSaving && pendingAction === "save"}
          <Icon name="sync" class="animate-spin" />
          {$t.saving}
        {:else}
          <Icon name="edit_note" />
          {$t.writeToMeter}
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

  <!-- Period Settings -->
  <div class="grid grid-cols-1 md:grid-cols-3 gap-6">
    <!-- Demand Period -->
    <div
      class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm"
    >
      <h4 class="font-bold text-slate-900 dark:text-white mb-4 flex items-center gap-2">
        <Icon name="trending_up" class="text-primary" />
        {$t.demandPeriod}
      </h4>
      <div class="space-y-4">
        <select
          bind:value={editDemandPeriod}
          disabled={!$isConnected}
          class="w-full bg-white dark:bg-[#1a2632] text-slate-700 dark:text-white border border-slate-300 dark:border-[#334a5e] rounded-lg px-4 py-3 focus:border-primary focus:ring-1 focus:ring-primary outline-none disabled:opacity-50 disabled:cursor-not-allowed"
        >
          <option value={5}>5 {$t.minutes}</option>
          <option value={10}>10 {$t.minutes}</option>
          <option value={15}>15 {$t.minutes}</option>
          <option value={30}>30 {$t.minutes}</option>
          <option value={60}>60 {$t.minutes}</option>
        </select>
        <p class="text-xs text-slate-400 font-mono">OBIS: 0.8.0</p>
      </div>
    </div>

    <!-- Load Profile Period -->
    <div
      class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm"
    >
      <h4 class="font-bold text-slate-900 dark:text-white mb-4 flex items-center gap-2">
        <Icon name="bar_chart" class="text-primary" />
        {$t.loadProfilePeriod}
      </h4>
      <div class="space-y-4">
        <select
          bind:value={editLoadProfilePeriod}
          disabled={!$isConnected}
          class="w-full bg-white dark:bg-[#1a2632] text-slate-700 dark:text-white border border-slate-300 dark:border-[#334a5e] rounded-lg px-4 py-3 focus:border-primary focus:ring-1 focus:ring-primary outline-none disabled:opacity-50 disabled:cursor-not-allowed"
        >
          <option value={1}>1 {$t.minutes}</option>
          <option value={5}>5 {$t.minutes}</option>
          <option value={10}>10 {$t.minutes}</option>
          <option value={15}>15 {$t.minutes}</option>
          <option value={20}>20 {$t.minutes}</option>
          <option value={30}>30 {$t.minutes}</option>
          <option value={60}>60 {$t.minutes}</option>
          <option value={90}>90 {$t.minutes}</option>
          <option value={120}>120 {$t.minutes}</option>
          <option value={180}>180 {$t.minutes}</option>
          <option value={360}>360 {$t.minutes}</option>
          <option value={720}>720 {$t.minutes}</option>
          <option value={1440}>1440 {$t.minutes}</option>
          <option value={2880}>2880 {$t.minutes}</option>
        </select>
        <p class="text-xs text-slate-400 font-mono">OBIS: 0.8.4</p>
      </div>
    </div>

    <!-- Outage Threshold -->
    <div
      class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm"
    >
      <h4 class="font-bold text-slate-900 dark:text-white mb-4 flex items-center gap-2">
        <Icon name="power_off" class="text-primary" />
        {$t.outageThreshold}
      </h4>
      <div class="space-y-4">
        <div class="flex items-center gap-2">
          <input
            type="number"
            min={0}
            max={3600}
            bind:value={editOutageThreshold}
            disabled={!$isConnected}
            class="flex-1 bg-white dark:bg-[#1a2632] text-slate-700 dark:text-white border border-slate-300 dark:border-[#334a5e] rounded-lg px-4 py-3 focus:border-primary focus:ring-1 focus:ring-primary outline-none disabled:opacity-50 disabled:cursor-not-allowed"
          />
          <span class="text-sm text-slate-500">{$t.seconds}</span>
        </div>
        <p class="text-xs text-slate-400 font-mono">OBIS: 0.9.9</p>
      </div>
    </div>
  </div>

  <!-- Manual Demand Reset -->
  <div
    class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm"
  >
    <h4 class="font-bold text-slate-900 dark:text-white mb-4 flex items-center gap-2">
      <Icon name="restart_alt" class="text-primary" />
      {$t.manualDemandReset}
    </h4>
    <p class="text-sm text-slate-500 dark:text-slate-400 mb-4">
      {$t.demandResetDescription}
    </p>
    <button
      onclick={openResetDialog}
      disabled={!$isConnected || isSaving}
      class="flex items-center gap-2 px-6 py-3 bg-red-500 hover:bg-red-600 text-white font-bold rounded-xl transition-all disabled:opacity-50 disabled:cursor-not-allowed"
    >
      {#if isSaving && pendingAction === "reset"}
        <Icon name="sync" class="animate-spin" />
        {$t.saving}
      {:else}
        <Icon name="restart_alt" />
        {$t.resetDemand}
      {/if}
    </button>
  </div>
</div>

<!-- Demand Reset Confirm Dialog -->
{#if showResetConfirm}
  <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50" role="dialog">
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="absolute inset-0" onclick={() => showResetConfirm = false}></div>
    <div class="relative bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-2xl p-6 w-full max-w-sm shadow-2xl">
      <div class="flex items-center gap-3 mb-4">
        <div class="p-3 rounded-xl bg-red-500/10">
          <Icon name="warning" class="text-red-500 text-2xl" />
        </div>
        <h3 class="text-lg font-bold text-slate-900 dark:text-white">{$t.manualDemandReset}</h3>
      </div>
      <p class="text-sm text-slate-500 mb-6">{$t.demandResetConfirm}</p>
      <div class="flex gap-3">
        <button
          onclick={() => showResetConfirm = false}
          class="flex-1 px-4 py-3 bg-slate-100 dark:bg-[#334a5e] hover:bg-slate-200 dark:hover:bg-[#455a6e] text-slate-700 dark:text-white font-bold rounded-xl transition-colors"
        >
          {$t.cancel}
        </button>
        <button
          onclick={confirmReset}
          class="flex-1 px-4 py-3 bg-red-500 hover:bg-red-600 text-white font-bold rounded-xl transition-all"
        >
          {$t.resetDemand}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Password Dialog -->
{#if showPasswordDialog}
  <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50" role="dialog">
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="absolute inset-0" onclick={() => showPasswordDialog = false}></div>
    <div class="relative bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-2xl p-6 w-full max-w-sm shadow-2xl">
      <h3 class="text-lg font-bold text-slate-900 dark:text-white mb-1">
        {pendingAction === "save" ? $t.periodSettings : $t.manualDemandReset}
      </h3>
      <p class="text-sm text-slate-500 mb-4">{$t.passwordWarning}</p>

      <div class="mb-4">
        <label class="block text-sm font-bold text-slate-700 dark:text-slate-300 mb-2" for="period-password">
          {$t.password}
        </label>
        <input
          id="period-password"
          type="password"
          maxlength={8}
          bind:value={password}
          onkeydown={(e) => { if (e.key === "Enter") handlePasswordSubmit(); }}
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
          onclick={handlePasswordSubmit}
          disabled={password.length !== 8}
          class="flex-1 px-4 py-3 {pendingAction === 'save' ? 'bg-primary hover:bg-primary/90 shadow-primary/20' : 'bg-red-500 hover:bg-red-600 shadow-red-500/20'} text-white font-bold rounded-xl shadow-lg transition-all disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {#if pendingAction === "save"}
            <Icon name="edit_note" size="sm" class="inline mr-1" />
            {$t.writeToMeter}
          {:else}
            <Icon name="restart_alt" size="sm" class="inline mr-1" />
            {$t.resetDemand}
          {/if}
        </button>
      </div>
    </div>
  </div>
{/if}
