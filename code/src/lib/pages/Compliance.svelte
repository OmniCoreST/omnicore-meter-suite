<script lang="ts">
  import { t } from "$lib/stores";
  import { complianceStore } from "$lib/stores/compliance";
  import { meterStore } from "$lib/stores/meter";
  import {
    checkCompliance,
    getComplianceRulesPath,
    reloadComplianceRules,
    updateComplianceRules,
    isTauri,
  } from "$lib/utils/tauri";
  import { warningToast, successToast, errorToast } from "$lib/stores/toast";

  let reloading = $state(false);
  let updating = $state(false);

  const currentData = $derived(
    $meterStore.shortReadData ?? $meterStore.fullReadData
  );
  const result = $derived($complianceStore.result);
  const loading = $derived($complianceStore.loading);

  const sortedIssues = $derived(
    result?.issues.slice().sort((a, b) => {
      const order = { error: 0, warning: 1, info: 2 };
      return (order[a.severity as keyof typeof order] ?? 3) -
             (order[b.severity as keyof typeof order] ?? 3);
    }) ?? []
  );

  async function runCheck() {
    if (!currentData) return;
    complianceStore.setLoading();
    try {
      const res = await checkCompliance(currentData as any);
      complianceStore.setResult(res);
      if (res.rulesStatus === "tooOld") {
        errorToast($t.complianceTooOld);
      } else if (res.errorCount === 0 && res.warningCount === 0) {
        successToast($t.complianceAllPassed);
      } else {
        const parts: string[] = [];
        if (res.errorCount > 0) parts.push(`${res.errorCount} ${$t.complianceErrors}`);
        if (res.warningCount > 0) parts.push(`${res.warningCount} ${$t.complianceWarnings}`);
        warningToast(parts.join(", "));
      }
    } catch (e) {
      complianceStore.setError(String(e));
      errorToast(String(e));
    }
  }

  async function openRulesFile() {
    if (!isTauri()) return;
    try {
      const path = await getComplianceRulesPath();
      const { openPath } = await import("@tauri-apps/plugin-opener");
      await openPath(path);
    } catch (e) {
      errorToast(String(e));
    }
  }

  async function reloadRules() {
    reloading = true;
    try {
      const msg = await reloadComplianceRules();
      successToast(msg);
      if (currentData) await runCheck();
    } catch (e) {
      errorToast(String(e));
    } finally {
      reloading = false;
    }
  }

  async function updateRules() {
    updating = true;
    try {
      const msg = await updateComplianceRules();
      successToast(msg);
      if (currentData) await runCheck();
    } catch (e) {
      errorToast(String(e));
    } finally {
      updating = false;
    }
  }

  function formatTime(iso: string): string {
    try {
      return new Date(iso).toLocaleTimeString("tr-TR", {
        hour: "2-digit", minute: "2-digit", second: "2-digit",
      });
    } catch { return iso; }
  }

  function severityIcon(s: string) {
    return s === "error" ? "cancel" : s === "warning" ? "warning" : "info";
  }
  function severityText(s: string) {
    return s === "error" ? "text-red-500" : s === "warning" ? "text-yellow-500" : "text-blue-400";
  }
  function severityBorder(s: string) {
    return s === "error"
      ? "bg-red-500/10 border-red-500/20"
      : s === "warning"
      ? "bg-yellow-500/10 border-yellow-500/20"
      : "bg-blue-500/10 border-blue-500/20";
  }
</script>

<div class="space-y-6">
  <!-- Başlık -->
  <div class="flex items-start justify-between gap-4 flex-wrap">
    <div>
      <h2 class="text-xl font-bold text-slate-900 dark:text-white">
        TEDAŞ Uyumluluk Kontrolü
      </h2>
      <p class="text-sm text-slate-500 dark:text-slate-400 mt-1">
        {$t.complianceDescription}
      </p>
    </div>

    <button
      onclick={runCheck}
      disabled={!currentData || loading}
      class="flex items-center gap-2 px-4 py-2 rounded-lg font-medium text-sm transition-all
        {!currentData || loading
          ? 'bg-slate-200 dark:bg-slate-700 text-slate-400 cursor-not-allowed'
          : 'bg-primary text-white hover:bg-primary/90 active:scale-95'}"
    >
      {#if loading}
        <span class="material-symbols-outlined text-base animate-spin">autorenew</span>
        Kontrol ediliyor...
      {:else}
        <span class="material-symbols-outlined text-base">verified_user</span>
        {$t.complianceCheck}
      {/if}
    </button>
  </div>

  <!-- Veri yok -->
  {#if !currentData}
    <div class="rounded-xl border border-dashed border-slate-300 dark:border-slate-600 p-8 text-center">
      <span class="material-symbols-outlined text-4xl text-slate-400 mb-3 block">sensors_off</span>
      <p class="text-slate-500 dark:text-slate-400 text-sm">{$t.complianceNoData}</p>
    </div>

  {:else if result}

    <!-- Kurallar çok eski → kilitli -->
    {#if result.rulesStatus === "tooOld"}
      <div class="rounded-xl border border-red-500/30 bg-red-500/10 p-6 text-center space-y-3">
        <span class="material-symbols-outlined text-4xl text-red-500 block">block</span>
        <p class="font-bold text-red-500">{$t.complianceTooOld}</p>
        <p class="text-sm text-slate-500 dark:text-slate-400">{$t.complianceTooOldDesc}</p>
        <p class="text-xs text-slate-400">
          Yerel: v{result.rulesVersion}
          {#if result.latestVersion} → Güncel: v{result.latestVersion}{/if}
        </p>
        <button
          onclick={updateRules}
          disabled={updating}
          class="mx-auto flex items-center gap-2 px-4 py-2 rounded-lg bg-red-500 text-white text-sm font-medium hover:bg-red-600 transition-all disabled:opacity-50"
        >
          <span class="material-symbols-outlined text-base {updating ? 'animate-spin' : ''}">cloud_download</span>
          {updating ? $t.complianceUpdating : $t.complianceUpdate}
        </button>
      </div>

    {:else}
      <!-- Durum bildirimleri -->
      {#if result.rulesStatus === "offline"}
        <div class="rounded-lg border border-yellow-500/30 bg-yellow-500/10 px-4 py-2 flex items-center gap-2 text-sm text-yellow-600 dark:text-yellow-400">
          <span class="material-symbols-outlined text-base">wifi_off</span>
          {$t.complianceOffline}
        </div>
      {:else if result.latestVersion && result.latestVersion !== result.rulesVersion}
        <div class="rounded-lg border border-blue-500/30 bg-blue-500/10 px-4 py-2 flex items-center gap-2 text-sm text-blue-600 dark:text-blue-400">
          <span class="material-symbols-outlined text-base">update</span>
          {$t.complianceOutdated}: v{result.latestVersion}
          <button onclick={updateRules} disabled={updating} class="ml-auto text-xs underline disabled:opacity-50">
            {updating ? $t.complianceUpdating : $t.complianceUpdate}
          </button>
        </div>
      {/if}

      <!-- Meta + araçlar -->
      <div class="flex items-center gap-3 flex-wrap">
        <span class="text-xs text-slate-500 dark:text-slate-400">
          <span class="font-medium">{$t.complianceRulesVersion}:</span> {result.rulesVersion}
        </span>
        <span class="text-slate-300 dark:text-slate-600">•</span>
        <span class="text-xs text-slate-500 dark:text-slate-400">
          <span class="font-medium">{$t.complianceLastCheck}:</span> {formatTime(result.checkedAt)}
        </span>
        <div class="ml-auto flex items-center gap-2">
          <button onclick={openRulesFile}
            class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium border border-slate-200 dark:border-slate-600 text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700 transition-all">
            <span class="material-symbols-outlined text-sm">open_in_new</span>
            {$t.complianceOpenRules}
          </button>
          <button onclick={reloadRules} disabled={reloading}
            class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium border border-slate-200 dark:border-slate-600 text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700 transition-all disabled:opacity-50">
            <span class="material-symbols-outlined text-sm {reloading ? 'animate-spin' : ''}">refresh</span>
            {reloading ? $t.complianceReloading : $t.complianceReload}
          </button>
          <button onclick={updateRules} disabled={updating}
            class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium border border-slate-200 dark:border-slate-600 text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700 transition-all disabled:opacity-50">
            <span class="material-symbols-outlined text-sm {updating ? 'animate-spin' : ''}">cloud_download</span>
            {updating ? $t.complianceUpdating : $t.complianceUpdate}
          </button>
        </div>
      </div>

      <!-- Özet -->
      <div class="grid grid-cols-2 gap-3">
        <div class="rounded-xl p-4 border {result.errorCount > 0 ? 'bg-red-500/10 border-red-500/20' : 'bg-slate-50 dark:bg-slate-800/50 border-slate-200 dark:border-slate-700'}">
          <div class="flex items-center gap-3">
            <span class="material-symbols-outlined text-2xl {result.errorCount > 0 ? 'text-red-500' : 'text-slate-300 dark:text-slate-600'}">
              {result.errorCount > 0 ? 'cancel' : 'check_circle'}
            </span>
            <div>
              <p class="text-2xl font-bold {result.errorCount > 0 ? 'text-red-500' : 'text-slate-400'}">
                {result.errorCount}
              </p>
              <p class="text-xs text-slate-500">{$t.complianceErrors}</p>
            </div>
          </div>
        </div>
        <div class="rounded-xl p-4 border {result.warningCount > 0 ? 'bg-yellow-500/10 border-yellow-500/20' : 'bg-slate-50 dark:bg-slate-800/50 border-slate-200 dark:border-slate-700'}">
          <div class="flex items-center gap-3">
            <span class="material-symbols-outlined text-2xl {result.warningCount > 0 ? 'text-yellow-500' : 'text-slate-300 dark:text-slate-600'}">
              {result.warningCount > 0 ? 'warning' : 'check_circle'}
            </span>
            <div>
              <p class="text-2xl font-bold {result.warningCount > 0 ? 'text-yellow-500' : 'text-slate-400'}">
                {result.warningCount}
              </p>
              <p class="text-xs text-slate-500">{$t.complianceWarnings}</p>
            </div>
          </div>
        </div>
      </div>

      <!-- Tüm kurallar geçti -->
      {#if result.issues.length === 0}
        <div class="rounded-xl border border-emerald-500/20 bg-emerald-500/10 p-6 text-center">
          <span class="material-symbols-outlined text-4xl text-emerald-500 mb-2 block">verified</span>
          <p class="font-bold text-emerald-600 dark:text-emerald-400">{$t.complianceAllPassed}</p>
          <p class="text-sm text-slate-500 mt-1">{$t.complianceAllPassedDesc}</p>
        </div>
      {:else}
        <!-- İhlaller listesi -->
        <div class="space-y-2">
          {#each sortedIssues as issue (issue.code)}
            <div class="rounded-xl border p-4 {severityBorder(issue.severity)}">
              <div class="flex items-start gap-3">
                <span class="material-symbols-outlined text-xl mt-0.5 {severityText(issue.severity)} flex-shrink-0">
                  {severityIcon(issue.severity)}
                </span>
                <div class="flex-1 min-w-0">
                  <div class="flex items-center gap-2 mb-1">
                    <span class="font-mono text-xs font-bold {severityText(issue.severity)}">{issue.code}</span>
                    <span class="text-xs text-slate-500 font-medium">{issue.field}</span>
                  </div>
                  <p class="text-sm text-slate-700 dark:text-slate-200 font-medium mb-2">{issue.description}</p>
                  <div class="grid grid-cols-2 gap-2 text-xs">
                    <div class="bg-white/50 dark:bg-black/20 rounded-lg p-2">
                      <p class="text-slate-400 mb-0.5">{$t.complianceExpected}</p>
                      <p class="font-mono text-slate-600 dark:text-slate-300">{issue.expected}</p>
                    </div>
                    <div class="bg-white/50 dark:bg-black/20 rounded-lg p-2">
                      <p class="text-slate-400 mb-0.5">{$t.complianceActual}</p>
                      <p class="font-mono {severityText(issue.severity)} font-medium">{issue.actual}</p>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          {/each}
        </div>

        <!-- Rapor notu (Excel çıktısında da yer alacak) -->
        <p class="text-xs text-slate-400 dark:text-slate-500 text-right">
          {$t.complianceReportNote} v{result.rulesVersion}
          {#if result.latestVersion && result.latestVersion !== result.rulesVersion}
            · {$t.complianceOutdated}: v{result.latestVersion}
          {/if}
        </p>
      {/if}
    {/if}

  {:else if !loading}
    <!-- Henüz kontrol yapılmadı -->
    <div class="rounded-xl border border-dashed border-slate-300 dark:border-slate-600 p-8 text-center space-y-3">
      <span class="material-symbols-outlined text-4xl text-slate-400 block">policy</span>
      <p class="text-slate-500 dark:text-slate-400 text-sm">
        Uyumluluk kontrolü için "Kontrol Et" butonuna tıklayın.
      </p>
      <div class="flex items-center justify-center gap-2">
        <button onclick={openRulesFile}
          class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium border border-slate-200 dark:border-slate-600 text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700 transition-all">
          <span class="material-symbols-outlined text-sm">open_in_new</span>
          {$t.complianceOpenRules}
        </button>
      </div>
    </div>
  {/if}
</div>
