<script lang="ts">
  import Icon from "$lib/components/common/Icon.svelte";
  import { themeStore, localeStore, t, isConnected, connectionStore, navigationStore, type Locale } from "$lib/stores";

  const pageTitles: Record<string, keyof typeof import("$lib/i18n/tr").tr> = {
    dashboard: "dashboard",
    "short-read": "shortRead",
    "full-read": "fullRead",
    "load-profile": "loadProfile",
    events: "events",
    alarms: "alarms",
    "time-sync": "timeSync",
    password: "passwordChange",
    dst: "dstSettings",
    periods: "periodSettings",
    tariffs: "tariffSettings",
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

  // Session save dialog
  let showSaveDialog = $state(false);
  let sessionNote = $state("");
  let overwriteExisting = $state(false);

  function openSaveDialog() {
    sessionNote = "";
    overwriteExisting = false;
    showSaveDialog = true;
  }

  function closeSaveDialog() {
    showSaveDialog = false;
  }

  function saveSession() {
    // TODO: Implement actual session saving logic
    console.log("Saving session:", {
      meterInfo,
      note: sessionNote,
      overwrite: overwriteExisting,
    });
    closeSaveDialog();
  }
</script>

<header
  class="flex items-center justify-between px-6 py-3 border-b border-slate-200 dark:border-[#334a5e] bg-white/80 dark:bg-[#0f1821]/80 backdrop-blur-md sticky top-0 z-20 transition-colors duration-300"
>
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

    <!-- Save Session Button -->
    {#if $isConnected}
      <button
        onclick={openSaveDialog}
        class="flex items-center gap-1.5 px-3 py-1.5 bg-primary hover:bg-primary/90 text-white text-xs font-bold rounded-lg transition-colors"
        title={$t.saveSession}
      >
        <Icon name="save" size="sm" />
        <span class="hidden sm:inline">{$t.saveSession}</span>
      </button>
    {/if}

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

    <!-- Notifications -->
    <button
      class="relative p-2 rounded-lg hover:bg-slate-100 dark:hover:bg-[#334a5e] text-slate-500 dark:text-slate-300 transition-colors"
    >
      <Icon name="notifications" size="sm" />
      <span
        class="absolute top-1.5 right-1.5 size-2 bg-red-500 rounded-lg border-2 border-white dark:border-[#0f1821]"
      ></span>
    </button>
  </div>
</header>

<!-- Save Session Dialog -->
{#if showSaveDialog}
  <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50" onclick={closeSaveDialog}>
    <div
      class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl shadow-xl w-full max-w-md mx-4"
      onclick={(e) => e.stopPropagation()}
    >
      <!-- Dialog Header -->
      <div class="flex items-center justify-between px-5 py-4 border-b border-slate-200 dark:border-[#334a5e]">
        <h3 class="text-lg font-bold text-slate-900 dark:text-white">{$t.saveSession}</h3>
        <button
          onclick={closeSaveDialog}
          class="p-1 rounded-lg hover:bg-slate-100 dark:hover:bg-[#334a5e] text-slate-500 transition-colors"
        >
          <Icon name="close" size="sm" />
        </button>
      </div>

      <!-- Dialog Body -->
      <div class="p-5 space-y-4">
        <!-- Meter Info Display -->
        {#if meterInfo}
          <div class="p-3 bg-slate-50 dark:bg-[#0f1821] rounded-lg">
            <p class="text-xs text-slate-500 mb-1">{$t.meterInfo}</p>
            <p class="text-sm font-bold text-slate-900 dark:text-white">
              {meterInfo.flag} — {meterInfo.serialNumber}
            </p>
            <p class="text-xs text-slate-500">{meterInfo.model}</p>
          </div>
        {/if}

        <!-- Session Note -->
        <div class="flex flex-col gap-1.5">
          <label class="text-xs font-bold text-slate-500 dark:text-slate-400 uppercase tracking-wider">
            {$t.sessionNote}
          </label>
          <textarea
            bind:value={sessionNote}
            placeholder={$t.sessionNotePlaceholder}
            rows="3"
            class="w-full px-3 py-2 bg-white dark:bg-[#1a2632] border border-slate-300 dark:border-[#334a5e] rounded-lg text-sm text-slate-900 dark:text-white focus:border-primary focus:ring-1 focus:ring-primary transition-colors resize-none"
          ></textarea>
        </div>

        <!-- Overwrite Option -->
        <label class="flex items-center gap-2 cursor-pointer">
          <input
            type="checkbox"
            bind:checked={overwriteExisting}
            class="w-4 h-4 rounded border-slate-300 dark:border-[#334a5e] text-primary focus:ring-primary"
          />
          <span class="text-sm text-slate-600 dark:text-slate-400">{$t.overwriteExisting}</span>
        </label>
      </div>

      <!-- Dialog Footer -->
      <div class="flex justify-end gap-3 px-5 py-4 border-t border-slate-200 dark:border-[#334a5e]">
        <button
          onclick={closeSaveDialog}
          class="px-4 py-2 text-sm font-bold text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-[#334a5e] rounded-lg transition-colors"
        >
          {$t.cancel}
        </button>
        <button
          onclick={saveSession}
          class="flex items-center gap-2 px-4 py-2 bg-primary hover:bg-primary/90 text-white text-sm font-bold rounded-lg transition-colors"
        >
          <Icon name="save" size="sm" />
          {$t.save}
        </button>
      </div>
    </div>
  </div>
{/if}
