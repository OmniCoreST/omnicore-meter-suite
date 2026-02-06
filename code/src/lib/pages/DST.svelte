<script lang="ts">
  import Icon from "$lib/components/common/Icon.svelte";
  import { t, isConnected, meterStore, addLog } from "$lib/stores";
  import { authenticate, writeObis, endSession } from "$lib/utils/tauri";

  interface DstPeriod {
    id: number;
    offset: string;
    forward: string;
    backward: string;
  }

  // Parse DST data from raw meter data (96.90.x OBIS codes)
  let dstData = $derived.by(() => {
    // @ts-ignore
    const raw: string | null = $meterStore.fullReadData?.rawData || $meterStore.shortReadData?.rawData || null;

    if (!raw) {
      return { enabled: false, periods: [] as DstPeriod[], hasData: false };
    }

    const enabledMatch = raw.match(/96\.90\.0\((\d+)\)/);
    const enabled = enabledMatch ? enabledMatch[1] !== "0" : false;

    const periods: DstPeriod[] = [];
    for (let i = 1; i <= 12; i++) {
      const pattern = new RegExp(`96\\.90\\.${i}(?!\\d)\\(([^)]+)\\)`);
      const match = raw.match(pattern);
      if (match) {
        const content = match[1];
        const [forwardPart, backwardPart] = content.split(";");
        if (forwardPart && backwardPart) {
          const forwardTokens = forwardPart.split(",");
          const backwardTokens = backwardPart.split(",");

          const offset = forwardTokens[0] || "";
          const forwardDate = forwardTokens[1] || "";
          const forwardTime = forwardTokens[2] || "";
          const backwardDate = backwardTokens[0] || "";
          const backwardTime = backwardTokens[1] || "";

          const fmtDate = (d: string) => {
            if (!d || d.startsWith("00-00-00")) return "";
            return `20${d}`;
          };

          const fwd = fmtDate(forwardDate);
          const bwd = fmtDate(backwardDate);

          periods.push({
            id: i,
            offset,
            forward: fwd ? `${fwd} ${forwardTime}` : "",
            backward: bwd ? `${bwd} ${backwardTime}` : "",
          });
        }
      }
    }

    return { enabled, periods, hasData: enabledMatch !== null };
  });

  // Editable state - initialized from meter data
  let editEnabled = $state(false);
  let editPeriods = $state<DstPeriod[]>(
    Array.from({ length: 12 }, (_, i) => ({ id: i + 1, offset: "", forward: "", backward: "" }))
  );
  let initialized = $state(false);

  // Initialize editable state from meter data
  $effect(() => {
    if (dstData.hasData && !initialized) {
      editEnabled = dstData.enabled;
      editPeriods = Array.from({ length: 12 }, (_, i) => {
        const existing = dstData.periods.find(p => p.id === i + 1);
        return existing ? { ...existing } : { id: i + 1, offset: "", forward: "", backward: "" };
      });
      initialized = true;
    }
  });

  // Password dialog
  let showPasswordDialog = $state(false);
  let password = $state("");
  let passwordError = $state("");
  let isSaving = $state(false);

  function openSaveDialog() {
    password = "";
    passwordError = "";
    showPasswordDialog = true;
  }

  function periodToObisValue(period: DstPeriod): string {
    const toShortDate = (d: string) => {
      const parts = d.trim().split(" ");
      if (parts.length !== 2) return "00-00-00,00:00";
      const date = parts[0].length === 10 ? parts[0].slice(2) : parts[0];
      return `${date},${parts[1]}`;
    };

    const offset = period.offset || "+00:00";
    const fwd = period.forward ? toShortDate(period.forward) : "00-00-00,00:00";
    const bwd = period.backward ? toShortDate(period.backward) : "00-00-00,00:00";
    return `${offset},${fwd};${bwd}`;
  }

  async function handleSave() {
    if (password.length !== 8 || !/^\d{8}$/.test(password)) {
      passwordError = $t.passwordMustBe8Digits;
      return;
    }

    showPasswordDialog = false;
    isSaving = true;
    addLog("info", $t.savingDstSettings);

    try {
      const authOk = await authenticate(password);
      if (!authOk) {
        addLog("error", $t.errorWrongPassword.replace("{0}", "?"));
        return;
      }

      await writeObis("96.90.0", editEnabled ? "1" : "0");
      addLog("info", `96.90.0 = ${editEnabled ? "1" : "0"}`);

      for (const period of editPeriods) {
        if (period.offset || period.forward || period.backward) {
          const value = periodToObisValue(period);
          await writeObis(`96.90.${period.id}`, value);
          addLog("info", `96.90.${period.id} = ${value}`);
        }
      }

      await endSession();
      addLog("success", $t.dstSaveSuccess);
    } catch (error) {
      addLog("error", `${$t.logError}: ${error}`);
    } finally {
      isSaving = false;
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
        <h3 class="text-xl font-bold text-slate-900 dark:text-white mb-2">{$t.dstSettings}</h3>
        <p class="text-sm text-slate-500 dark:text-slate-400">
          {$t.dstStatusDescription}
        </p>
      </div>
      <button
        onclick={openSaveDialog}
        disabled={!$isConnected || isSaving}
        class="flex items-center gap-2 px-5 py-2.5 bg-primary hover:bg-primary/90 text-white font-bold rounded-xl shadow-lg shadow-primary/20 transition-all disabled:opacity-50 disabled:cursor-not-allowed whitespace-nowrap"
      >
        {#if isSaving}
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

  <!-- DST Status + Toggle -->
  <div
    class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm"
  >
    <div class="flex items-center justify-between">
      <div>
        <h4 class="font-bold text-slate-900 dark:text-white mb-1">{$t.dstStatus}</h4>
        <p class="text-sm text-slate-500 dark:text-slate-400">
          {$t.dstSettingsDescription}
        </p>
      </div>
      <div class="flex items-center gap-3">
        <button
          onclick={() => (editEnabled = !editEnabled)}
          disabled={!$isConnected}
          class="relative w-14 h-8 rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed
            {editEnabled ? 'bg-primary' : 'bg-slate-300 dark:bg-slate-600'}"
        >
          <span
            class="absolute top-1 left-1 w-6 h-6 bg-white rounded-lg transition-transform shadow-md
              {editEnabled ? 'translate-x-6' : ''}"
          ></span>
        </button>
        <span class="text-sm font-bold {editEnabled ? 'text-emerald-500' : 'text-slate-500'}">
          {editEnabled ? $t.enabled : $t.disabled}
        </span>
      </div>
    </div>
  </div>

  <!-- DST Periods -->
  <div
    class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm"
  >
    <div class="mb-4">
      <h4 class="font-bold text-slate-900 dark:text-white">{$t.dstPeriods}</h4>
    </div>

    <div class="overflow-x-auto">
      <table class="w-full">
        <thead>
          <tr class="border-b border-slate-200 dark:border-[#334a5e]">
            <th class="px-4 py-3 text-left text-xs font-bold text-slate-500 uppercase">
              {$t.periodNumber}
            </th>
            <th class="px-4 py-3 text-left text-xs font-bold text-slate-500 uppercase">
              {$t.timeOffset}
            </th>
            <th class="px-4 py-3 text-left text-xs font-bold text-slate-500 uppercase">
              {$t.forwardDate}
            </th>
            <th class="px-4 py-3 text-left text-xs font-bold text-slate-500 uppercase">
              {$t.backwardDate}
            </th>
          </tr>
        </thead>
        <tbody class="divide-y divide-slate-200 dark:divide-[#334a5e]">
          {#each editPeriods as period}
            <tr>
              <td class="px-4 py-3 text-sm font-bold text-slate-900 dark:text-white">
                {period.id}
              </td>
              <td class="px-4 py-3">
                <input
                  type="text"
                  bind:value={period.offset}
                  placeholder="+01:00"
                  disabled={!$isConnected}
                  class="w-24 bg-white dark:bg-[#1a2632] text-slate-700 dark:text-white border border-slate-300 dark:border-[#334a5e] rounded-lg px-3 py-2 text-sm font-mono focus:border-primary focus:ring-1 focus:ring-primary outline-none disabled:opacity-50 disabled:cursor-not-allowed"
                />
              </td>
              <td class="px-4 py-3">
                <input
                  type="text"
                  bind:value={period.forward}
                  placeholder="2026-03-29 03:00"
                  disabled={!$isConnected}
                  class="w-44 bg-white dark:bg-[#1a2632] text-slate-700 dark:text-white border border-slate-300 dark:border-[#334a5e] rounded-lg px-3 py-2 text-sm font-mono focus:border-primary focus:ring-1 focus:ring-primary outline-none disabled:opacity-50 disabled:cursor-not-allowed"
                />
              </td>
              <td class="px-4 py-3">
                <input
                  type="text"
                  bind:value={period.backward}
                  placeholder="2026-10-25 04:00"
                  disabled={!$isConnected}
                  class="w-44 bg-white dark:bg-[#1a2632] text-slate-700 dark:text-white border border-slate-300 dark:border-[#334a5e] rounded-lg px-3 py-2 text-sm font-mono focus:border-primary focus:ring-1 focus:ring-primary outline-none disabled:opacity-50 disabled:cursor-not-allowed"
                />
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  </div>
</div>

<!-- Password Dialog -->
{#if showPasswordDialog}
  <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50" role="dialog">
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="absolute inset-0" onclick={() => showPasswordDialog = false}></div>
    <div class="relative bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-2xl p-6 w-full max-w-sm shadow-2xl">
      <h3 class="text-lg font-bold text-slate-900 dark:text-white mb-1">{$t.saveDstSettings}</h3>
      <p class="text-sm text-slate-500 mb-4">{$t.passwordWarning}</p>

      <div class="mb-4">
        <label class="block text-sm font-bold text-slate-700 dark:text-slate-300 mb-2" for="dst-password">
          {$t.password}
        </label>
        <input
          id="dst-password"
          type="password"
          maxlength={8}
          bind:value={password}
          onkeydown={(e) => { if (e.key === "Enter") handleSave(); }}
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
          onclick={handleSave}
          disabled={password.length !== 8}
          class="flex-1 px-4 py-3 bg-primary hover:bg-primary/90 text-white font-bold rounded-xl shadow-lg shadow-primary/20 transition-all disabled:opacity-50 disabled:cursor-not-allowed"
        >
          <Icon name="edit_note" size="sm" class="inline mr-1" />
          {$t.writeToMeter}
        </button>
      </div>
    </div>
  </div>
{/if}
