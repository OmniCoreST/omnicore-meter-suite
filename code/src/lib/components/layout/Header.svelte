<script lang="ts">
  import Icon from "$lib/components/common/Icon.svelte";
  import { themeStore, localeStore, t, isConnected, connectionStore, navigationStore, meterStore, successToast, errorToast, sessionsStore, type Locale } from "$lib/stores";
  import { saveSessionFile } from "$lib/utils/tauri";

  const pageTitles: Record<string, keyof typeof import("$lib/i18n/tr").tr> = {
    dashboard: "dashboard",
    overview: "overview",
    "live-measurements": "liveMeasurements",
    energy: "energy",
    demand: "demand",
    "load-profile": "loadProfile",
    warnings: "warnings",
    outages: "outages",
    "status-codes": "statusCodes",
    "time-date": "timeDate",
    password: "passwordChange",
    dst: "dstSettings",
    periods: "periodSettings",
    tariffs: "tariffSettings",
    "relay-control": "relayControl",
    "obis-reader": "obisReader",
  };

  function toggleTheme() {
    themeStore.toggle();
  }

  function setLocale(locale: Locale) {
    localeStore.setLocale(locale);
  }

  let currentLocale = $derived($localeStore);
  let pageTitle = $derived($t[pageTitles[$navigationStore] || "dashboard"]);

  // Get connected meter info
  let meterInfo = $derived($connectionStore.meterIdentity);
  let isViewingSession = $derived(false); // Will be used for viewing saved sessions

  // Check if reading data exists
  let hasReadingData = $derived($meterStore.shortReadData !== null);

  // Session save panel (slide-down)
  let showSavePanel = $state(false);
  let sessionNote = $state("");
  let overwriteExisting = $state(false);
  let isSaving = $state(false);

  function toggleSavePanel() {
    if (showSavePanel) {
      closeSavePanel();
    } else {
      sessionNote = "";
      overwriteExisting = false;
      showSavePanel = true;
    }
  }

  function closeSavePanel() {
    showSavePanel = false;
  }

  async function saveSession() {
    if (!meterInfo || !$meterStore.shortReadData) return;

    isSaving = true;
    try {
      const meterData = {
        shortReadData: $meterStore.shortReadData,
        fullReadData: $meterStore.fullReadData,
        loadProfileData: $meterStore.loadProfileData,
        meterType: $meterStore.meterType,
        isBidirectional: $meterStore.isBidirectional,
      };

      const connectionInfo = {
        connectionType: $connectionStore.params.connectionType,
        port: $connectionStore.selectedPort,
        meterIdentity: meterInfo,
      };

      const filename = await saveSessionFile(
        meterInfo.flag || "UNK",
        $meterStore.shortReadData.serialNumber || meterInfo.serialNumber || "UNKNOWN",
        meterInfo.model || "UNKNOWN",
        sessionNote,
        meterData,
        connectionInfo,
        overwriteExisting
      );

      successToast(`${$t.sessionSaved}: ${filename}`);

      // Refresh the sessions list
      sessionsStore.refresh();

      closeSavePanel();
    } catch (error) {
      console.error("Failed to save session:", error);
      errorToast(`${$t.sessionSaveError}: ${error}`);
    } finally {
      isSaving = false;
    }
  }
</script>

<header
  class="flex flex-col border-b border-slate-200 dark:border-[#334a5e] bg-white/80 dark:bg-[#0f1821]/80 backdrop-blur-md sticky top-0 z-20 transition-colors duration-300"
>
  <!-- Main Header Row -->
  <div class="flex items-center justify-between px-6 py-3">
    <h2 class="text-slate-900 dark:text-white text-xl font-bold tracking-tight">
      {pageTitle}
    </h2>

    <div class="flex items-center gap-3">
      <!-- Language Toggle -->
      <div
        class="hidden lg:flex items-center bg-slate-100 dark:bg-[#111c26] p-1 rounded-lg border border-slate-200 dark:border-[#334a5e]"
      >
        <button
          onclick={() => setLocale("en")}
          class="px-3 py-1 rounded text-xs font-bold transition-all
            {currentLocale === 'en'
            ? 'bg-primary text-white shadow-sm'
            : 'text-slate-500 dark:text-slate-400 hover:text-primary dark:hover:text-white hover:bg-slate-200 dark:hover:bg-[#334a5e]/50'}"
        >
          EN
        </button>
        <button
          onclick={() => setLocale("tr")}
          class="px-3 py-1 rounded text-xs font-bold transition-all
            {currentLocale === 'tr'
            ? 'bg-primary text-white shadow-sm'
            : 'text-slate-500 dark:text-slate-400 hover:text-primary dark:hover:text-white hover:bg-slate-200 dark:hover:bg-[#334a5e]/50'}"
        >
          TR
        </button>
      </div>

      <!-- Connection Status Badge -->
      <div
        class="flex items-center gap-2 px-3 py-1.5 rounded-lg
          {$isConnected
          ? 'bg-emerald-500/10 border border-emerald-500/20'
          : isViewingSession
            ? 'bg-blue-500/10 border border-blue-500/20'
            : 'bg-slate-100 dark:bg-[#1a2632] border border-slate-200 dark:border-[#334a5e]'}"
      >
        <span class="relative flex h-2 w-2">
          {#if $isConnected}
            <span
              class="animate-ping absolute inline-flex h-full w-full rounded-lg bg-emerald-400 opacity-75"
            ></span>
            <span class="relative inline-flex rounded-lg h-2 w-2 bg-emerald-500"></span>
          {:else if isViewingSession}
            <span class="relative inline-flex rounded-lg h-2 w-2 bg-blue-500"></span>
          {:else}
            <span class="relative inline-flex rounded-lg h-2 w-2 bg-slate-400"></span>
          {/if}
        </span>
        <span
          class="text-xs font-bold
            {$isConnected ? 'text-emerald-600 dark:text-emerald-400' : isViewingSession ? 'text-blue-600 dark:text-blue-400' : 'text-slate-500 dark:text-slate-400'}"
        >
          {#if $isConnected && meterInfo}
            {meterInfo.flag} — {meterInfo.serialNumber}
          {:else if isViewingSession}
            MKS — 123456789
          {:else}
            {$t.notConnected}
          {/if}
        </span>
      </div>

      <!-- Save Session Button (enabled only when reading data exists) -->
      <button
        onclick={toggleSavePanel}
        disabled={!hasReadingData}
        class="flex items-center gap-1.5 px-3 py-1.5 text-xs font-bold rounded-lg transition-colors
          {hasReadingData
            ? 'bg-primary hover:bg-primary/90 text-white'
            : 'bg-slate-200 dark:bg-[#1a2632] text-slate-400 dark:text-slate-500 cursor-not-allowed'}"
        title={hasReadingData ? $t.saveSession : $t.noDataToSave}
      >
        <Icon name={showSavePanel ? "expand_less" : "save"} size="sm" />
        <span class="hidden sm:inline">{$t.saveSession}</span>
      </button>

      <!-- Theme Toggle -->
      <button
        onclick={toggleTheme}
        class="p-2 rounded-lg hover:bg-slate-100 dark:hover:bg-[#334a5e] text-slate-500 dark:text-slate-300 transition-colors"
      >
        {#if $themeStore === "light"}
          <Icon name="dark_mode" size="sm" />
        {:else}
          <Icon name="light_mode" size="sm" />
        {/if}
      </button>

    </div>
  </div>

  <!-- Save Session Slide-down Panel -->
  {#if showSavePanel}
    <div class="px-6 py-5 bg-gradient-to-b from-slate-50 to-slate-100 dark:from-[#111c26] dark:to-[#0f1821] border-t border-slate-200 dark:border-[#334a5e] animate-slide-down">
      <div class="max-w-4xl mx-auto">
        <!-- Header Row -->
        <div class="flex items-center justify-between mb-4">
          <div class="flex items-center gap-3">
            <div class="p-2 bg-primary/10 rounded-lg">
              <Icon name="save" class="text-primary" />
            </div>
            <div>
              <h4 class="text-sm font-bold text-slate-900 dark:text-white">{$t.saveSession}</h4>
              {#if meterInfo}
                <p class="text-xs text-slate-500 dark:text-slate-400">
                  {meterInfo.flag} — {$meterStore.shortReadData?.serialNumber || meterInfo.serialNumber || "-"} • {meterInfo.model}
                </p>
              {/if}
            </div>
          </div>
          <button
            onclick={closeSavePanel}
            class="p-1.5 rounded-lg hover:bg-slate-200 dark:hover:bg-[#334a5e] text-slate-400 hover:text-slate-600 dark:hover:text-slate-300 transition-colors"
          >
            <Icon name="close" size="sm" />
          </button>
        </div>

        <!-- Input Row -->
        <div class="flex flex-col sm:flex-row gap-4 items-stretch sm:items-end">
          <!-- Session Note Input -->
          <div class="flex-grow">
            <input
              type="text"
              bind:value={sessionNote}
              placeholder={$t.sessionNotePlaceholder}
              class="w-full px-4 py-2.5 bg-white dark:bg-[#1a2632] border border-slate-200 dark:border-[#334a5e] rounded-xl text-sm text-slate-900 dark:text-white placeholder:text-slate-400 focus:border-primary focus:ring-2 focus:ring-primary/20 transition-all shadow-sm"
            />
          </div>

          <!-- Overwrite Checkbox -->
          <label class="flex items-center gap-2.5 cursor-pointer px-4 py-2.5 bg-white dark:bg-[#1a2632] border border-slate-200 dark:border-[#334a5e] rounded-xl hover:border-primary/30 transition-colors shadow-sm">
            <input
              type="checkbox"
              bind:checked={overwriteExisting}
              class="w-4 h-4 rounded border-slate-300 dark:border-[#334a5e] text-primary focus:ring-primary focus:ring-offset-0"
            />
            <span class="text-xs font-medium text-slate-600 dark:text-slate-400 whitespace-nowrap">{$t.overwriteExisting}</span>
          </label>

          <!-- Save Button -->
          <button
            onclick={saveSession}
            disabled={isSaving}
            class="flex items-center justify-center gap-2 px-6 py-2.5 bg-primary hover:bg-primary/90 text-white text-sm font-bold rounded-xl transition-all shadow-md shadow-primary/20 hover:shadow-lg hover:shadow-primary/30 disabled:opacity-50 disabled:cursor-not-allowed disabled:shadow-none"
          >
            {#if isSaving}
              <Icon name="sync" size="sm" class="animate-spin" />
              {$t.saving || "Saving..."}
            {:else}
              <Icon name="save" size="sm" />
              {$t.save}
            {/if}
          </button>
        </div>
      </div>
    </div>
  {/if}
</header>

<style>
  @keyframes slide-down {
    from {
      opacity: 0;
      transform: translateY(-10px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .animate-slide-down {
    animation: slide-down 0.2s ease-out;
  }
</style>
