<script lang="ts">
  import Icon from "$lib/components/common/Icon.svelte";
  import { t, isConnected, addLog } from "$lib/stores";
  import { readObis } from "$lib/utils/tauri";

  interface ObisRow {
    id: number;
    code: string;
    value: string;
    checked: boolean;
    loading: boolean;
  }

  let rows = $state<ObisRow[]>([
    { id: 1, code: "", value: "", checked: true, loading: false },
    { id: 2, code: "", value: "", checked: true, loading: false },
    { id: 3, code: "", value: "", checked: true, loading: false },
    { id: 4, code: "", value: "", checked: true, loading: false },
    { id: 5, code: "", value: "", checked: true, loading: false },
    { id: 6, code: "", value: "", checked: true, loading: false },
    { id: 7, code: "", value: "", checked: true, loading: false },
    { id: 8, code: "", value: "", checked: true, loading: false },
    { id: 9, code: "", value: "", checked: true, loading: false },
    { id: 10, code: "", value: "", checked: true, loading: false },
  ]);

  let isReading = $state(false);

  // Preset code groups
  const presets = {
    energy: ["1.8.0", "1.8.1", "1.8.2", "1.8.3", "2.8.0", "5.8.0", "6.8.0", "7.8.0", "8.8.0", "1.6.0"],
    instant: ["32.7.0", "52.7.0", "72.7.0", "31.7.0", "51.7.0", "71.7.0", "14.7.0", "33.7.0", "53.7.0", "73.7.0"],
    status: ["96.1.0", "0.9.1", "0.9.2", "96.50.1", "96.3.10", "97.97.0", "96.7.0", "96.7.1", "96.7.2", "96.7.3"],
  };

  function applyPreset(type: keyof typeof presets) {
    const codes = presets[type];
    rows = rows.map((row, i) => ({
      ...row,
      code: codes[i] || "",
      value: "",
      checked: !!codes[i],
    }));
    addLog("info", `${type} preset yuklendi`);
  }

  function clearAll() {
    rows = rows.map(row => ({
      ...row,
      code: "",
      value: "",
      checked: true,
    }));
  }

  function addRow() {
    if (rows.length < 20) {
      rows = [...rows, {
        id: rows.length + 1,
        code: "",
        value: "",
        checked: true,
        loading: false,
      }];
    }
  }

  function removeRow(id: number) {
    if (rows.length > 1) {
      rows = rows.filter(r => r.id !== id);
    }
  }

  async function readSelectedCodes() {
    if (!$isConnected || isReading) return;

    const selectedRows = rows.filter(r => r.checked && r.code.trim());
    if (selectedRows.length === 0) {
      addLog("warn", "Okunacak OBIS kodu secilmedi");
      return;
    }

    isReading = true;

    // Set loading state for selected rows
    rows = rows.map(r => ({
      ...r,
      loading: r.checked && r.code.trim() ? true : false,
      value: r.checked && r.code.trim() ? "" : r.value,
    }));

    addLog("info", `${selectedRows.length} OBIS kodu okunuyor...`);

    for (const row of selectedRows) {
      try {
        const result = await readObis(row.code.trim());

        rows = rows.map(r =>
          r.id === row.id
            ? { ...r, value: result || $t.noValue, loading: false }
            : r
        );
      } catch (error) {
        rows = rows.map(r =>
          r.id === row.id
            ? { ...r, value: `Hata: ${error}`, loading: false }
            : r
        );
      }
    }

    isReading = false;
    addLog("success", "OBIS okuma tamamlandi");
  }

  function toggleRow(id: number) {
    rows = rows.map(r =>
      r.id === id ? { ...r, checked: !r.checked } : r
    );
  }

  function updateCode(id: number, value: string) {
    rows = rows.map(r =>
      r.id === id ? { ...r, code: value } : r
    );
  }
</script>

<div class="space-y-6">
  <!-- Header -->
  <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm">
    <div class="flex items-start justify-between">
      <div>
        <h3 class="text-xl font-bold text-slate-900 dark:text-white mb-2">{$t.obisReader}</h3>
        <p class="text-sm text-slate-500 dark:text-slate-400">{$t.obisReaderDescription}</p>
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

  <!-- Preset Buttons -->
  <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-4 shadow-sm">
    <div class="flex flex-wrap items-center gap-3">
      <span class="text-sm font-bold text-slate-500">Hazir Setler:</span>
      <button
        onclick={() => applyPreset("energy")}
        class="px-4 py-2 bg-blue-500/10 hover:bg-blue-500/20 border border-blue-500/20 rounded-lg text-sm font-medium text-blue-600 dark:text-blue-400 transition-colors"
      >
        <Icon name="bolt" size="sm" class="inline mr-1" />
        {$t.presetEnergy}
      </button>
      <button
        onclick={() => applyPreset("instant")}
        class="px-4 py-2 bg-emerald-500/10 hover:bg-emerald-500/20 border border-emerald-500/20 rounded-lg text-sm font-medium text-emerald-600 dark:text-emerald-400 transition-colors"
      >
        <Icon name="speed" size="sm" class="inline mr-1" />
        {$t.presetInstant}
      </button>
      <button
        onclick={() => applyPreset("status")}
        class="px-4 py-2 bg-purple-500/10 hover:bg-purple-500/20 border border-purple-500/20 rounded-lg text-sm font-medium text-purple-600 dark:text-purple-400 transition-colors"
      >
        <Icon name="info" size="sm" class="inline mr-1" />
        {$t.presetStatus}
      </button>
      <div class="flex-1"></div>
      <button
        onclick={clearAll}
        class="px-4 py-2 bg-slate-100 dark:bg-[#334a5e] hover:bg-slate-200 dark:hover:bg-[#455a6e] rounded-lg text-sm font-medium text-slate-600 dark:text-slate-400 transition-colors"
      >
        <Icon name="clear_all" size="sm" class="inline mr-1" />
        {$t.clearAll}
      </button>
    </div>
  </div>

  <!-- OBIS Code Table -->
  <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl shadow-sm overflow-hidden">
    <div class="overflow-x-auto">
      <table class="w-full">
        <thead class="bg-slate-50 dark:bg-[#0f1821]">
          <tr class="border-b border-slate-200 dark:border-[#334a5e]">
            <th class="px-4 py-3 text-left w-12">
              <input
                type="checkbox"
                checked={rows.every(r => r.checked)}
                onchange={() => {
                  const allChecked = rows.every(r => r.checked);
                  rows = rows.map(r => ({ ...r, checked: !allChecked }));
                }}
                class="w-4 h-4 rounded border-slate-300 dark:border-[#334a5e] text-primary focus:ring-primary"
              />
            </th>
            <th class="px-4 py-3 text-left text-xs font-bold text-slate-500 uppercase">#</th>
            <th class="px-4 py-3 text-left text-xs font-bold text-slate-500 uppercase">{$t.obisCode}</th>
            <th class="px-4 py-3 text-left text-xs font-bold text-slate-500 uppercase">{$t.readValue}</th>
            <th class="px-4 py-3 w-12"></th>
          </tr>
        </thead>
        <tbody>
          {#each rows as row}
            <tr class="border-b border-slate-100 dark:border-[#334a5e]/30 hover:bg-slate-50 dark:hover:bg-[#1a2632] transition-colors">
              <td class="px-4 py-2">
                <input
                  type="checkbox"
                  checked={row.checked}
                  onchange={() => toggleRow(row.id)}
                  class="w-4 h-4 rounded border-slate-300 dark:border-[#334a5e] text-primary focus:ring-primary"
                />
              </td>
              <td class="px-4 py-2 text-sm text-slate-500">{row.id}</td>
              <td class="px-4 py-2">
                <input
                  type="text"
                  value={row.code}
                  oninput={(e) => updateCode(row.id, e.currentTarget.value)}
                  placeholder={$t.obisPlaceholder}
                  class="w-full px-3 py-2 bg-white dark:bg-[#1a2632] border border-slate-200 dark:border-[#334a5e] rounded-lg text-sm font-mono focus:border-primary focus:ring-1 focus:ring-primary outline-none"
                />
              </td>
              <td class="px-4 py-2">
                {#if row.loading}
                  <div class="flex items-center gap-2 text-primary">
                    <Icon name="sync" size="sm" class="animate-spin" />
                    <span class="text-sm">{$t.reading}</span>
                  </div>
                {:else if row.value}
                  <span class="text-sm font-mono text-slate-900 dark:text-white bg-slate-50 dark:bg-[#0f1821] px-3 py-1 rounded">
                    {row.value}
                  </span>
                {:else}
                  <span class="text-sm text-slate-400">-</span>
                {/if}
              </td>
              <td class="px-4 py-2">
                <button
                  onclick={() => removeRow(row.id)}
                  class="p-1 text-slate-400 hover:text-red-500 transition-colors"
                  title="Satiri Sil"
                >
                  <Icon name="close" size="sm" />
                </button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>

    <!-- Add Row Button -->
    <div class="p-4 border-t border-slate-200 dark:border-[#334a5e]">
      <button
        onclick={addRow}
        disabled={rows.length >= 20}
        class="flex items-center gap-2 px-4 py-2 text-sm font-medium text-primary hover:bg-primary/10 rounded-lg transition-colors disabled:opacity-50"
      >
        <Icon name="add" size="sm" />
        {$t.addRow}
      </button>
    </div>
  </div>

  <!-- Read Button -->
  <div class="flex justify-center">
    <button
      onclick={readSelectedCodes}
      disabled={!$isConnected || isReading || !rows.some(r => r.checked && r.code.trim())}
      class="flex items-center gap-3 px-8 py-4 bg-primary hover:bg-primary/90 text-white font-bold rounded-xl shadow-lg shadow-primary/20 transition-all disabled:opacity-50 disabled:cursor-not-allowed"
    >
      {#if isReading}
        <Icon name="sync" class="animate-spin" />
        {$t.reading}
      {:else}
        <Icon name="play_arrow" />
        {$t.readSelectedObis}
      {/if}
    </button>
  </div>
</div>
