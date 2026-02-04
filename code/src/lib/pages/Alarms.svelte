<script lang="ts">
  import Icon from "$lib/components/common/Icon.svelte";
  import { t, meterStore } from "$lib/stores";
  import { exportToExcel } from "$lib/utils/export";

  // FF Code bit definitions (which ones are urgent alarms)
  const ffBitUrgent = [6, 8, 9, 10, 11, 12, 13, 14, 15, 54]; // Critical alarms

  // Parse FF code from meter data
  let ffBits = $derived(() => {
    const data = $meterStore.shortReadData;
    if (!data || !data.ffCode) return [];

    // Parse hex string to BigInt
    const ffValue = BigInt("0x" + data.ffCode);

    // Extract all 64 bits
    const bits = [];
    for (let i = 0; i < 64; i++) {
      const isSet = (ffValue & (1n << BigInt(i))) !== 0n;
      bits.push({
        bit: i,
        key: `ffBit${i}`,
        status: isSet,
        urgent: ffBitUrgent.includes(i),
      });
    }
    return bits;
  });

  // Helper to get translation
  function getBitName(key: string, translations: Record<string, string>): string {
    return (translations as Record<string, string>)[key] || key;
  }

  // Mock GF data for export
  const gfData = {
    edasId: "04 - AYDEM (ADM)",
    trafoMerkezId: "0",
    trafoId: "0",
    deparId: "0",
    fazId: "0 - N/A",
    maxCurrent: "0 A",
  };

  function handleExportFF() {
    const exportData = ffBits.map(bit => ({
      bit: bit.bit,
      name: getBitName(bit.key, $t),
      status: bit.status ? ($t.active) : ($t.ok),
      urgent: bit.urgent ? $t.urgent : "-",
    }));

    exportToExcel(exportData, "ff_codes", [
      { key: "bit", label: $t.bitNumber },
      { key: "name", label: $t.bitName },
      { key: "status", label: $t.currentState },
      { key: "urgent", label: $t.urgent },
    ]);
  }

  function handleExportGF() {
    const exportData = [
      { field: "EDAŞ ID (Bits 0-4)", value: gfData.edasId },
      { field: "Trafo Merkez ID (Bits 5-19)", value: gfData.trafoMerkezId },
      { field: "Trafo ID (Bits 20-23)", value: gfData.trafoId },
      { field: "Depar ID (Bits 24-29)", value: gfData.deparId },
      { field: "Faz ID (Bits 30-31)", value: gfData.fazId },
      { field: "Max Current (Bits 34-43)", value: gfData.maxCurrent },
    ];

    exportToExcel(exportData, "gf_codes", [
      { key: "field", label: "Field" },
      { key: "value", label: "Value" },
    ]);
  }
</script>

<div class="space-y-6">
  <div
    class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm"
  >
    <div class="flex items-start justify-between">
      <div>
        <h3 class="text-xl font-bold text-slate-900 dark:text-white mb-2">{$t.alarms}</h3>
        <p class="text-sm text-slate-500 dark:text-slate-400">
          {$t.alarmsDescription}
        </p>
      </div>
    </div>
  </div>

  <!-- FF Codes -->
  <div
    class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm"
  >
    <div class="flex items-center justify-between mb-4">
      <h4 class="font-bold text-slate-900 dark:text-white flex items-center gap-2">
        <Icon name="report_problem" class="text-primary" />
        {$t.ffBits}
      </h4>
      <button
        onclick={handleExportFF}
        class="flex items-center gap-2 px-4 py-2 bg-emerald-600 hover:bg-emerald-700 text-white text-sm font-bold rounded-lg transition-colors"
      >
        <Icon name="download" size="sm" />
        {$t.exportToExcel}
      </button>
    </div>
    <div class="mb-4 p-3 bg-slate-50 dark:bg-[#0f1821] rounded-lg">
      <span class="text-xs text-slate-500 mr-2">FF Code:</span>
      <span class="font-mono text-sm text-slate-900 dark:text-white">0x0000000000000090</span>
    </div>
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
      {#each ffBits as bit}
        <div
          class="p-4 rounded-xl border transition-colors
            {bit.status
            ? bit.urgent
              ? 'bg-red-500/10 border-red-500/20'
              : 'bg-amber-500/10 border-amber-500/20'
            : 'bg-slate-50 dark:bg-[#0f1821] border-slate-200 dark:border-[#334a5e]'}"
        >
          <div class="flex items-center justify-between mb-2">
            <span class="text-xs font-mono text-slate-500">Bit {bit.bit}</span>
            {#if bit.urgent}
              <span class="text-[10px] font-bold text-red-500 uppercase">{$t.urgent}</span>
            {/if}
          </div>
          <div class="text-sm font-medium text-slate-900 dark:text-white mb-2">{getBitName(bit.key, $t)}</div>
          <div class="flex items-center gap-2">
            <div
              class="size-2 rounded-lg {bit.status
                ? bit.urgent
                  ? 'bg-red-500'
                  : 'bg-amber-500'
                : 'bg-emerald-500'}"
            ></div>
            <span
              class="text-xs font-bold {bit.status
                ? bit.urgent
                  ? 'text-red-500'
                  : 'text-amber-500'
                : 'text-emerald-500'}"
            >
              {bit.status ? $t.active : $t.ok}
            </span>
          </div>
        </div>
      {/each}
    </div>
  </div>

  <!-- GF Codes -->
  <div
    class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm"
  >
    <div class="flex items-center justify-between mb-4">
      <h4 class="font-bold text-slate-900 dark:text-white flex items-center gap-2">
        <Icon name="location_on" class="text-primary" />
        {$t.gfFields}
      </h4>
      <button
        onclick={handleExportGF}
        class="flex items-center gap-2 px-4 py-2 bg-emerald-600 hover:bg-emerald-700 text-white text-sm font-bold rounded-lg transition-colors"
      >
        <Icon name="download" size="sm" />
        {$t.exportToExcel}
      </button>
    </div>
    <div class="mb-4 p-3 bg-slate-50 dark:bg-[#0f1821] rounded-lg">
      <span class="text-xs text-slate-500 mr-2">GF Code:</span>
      <span class="font-mono text-sm text-slate-900 dark:text-white">0x0000000000000004</span>
    </div>
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
      <div class="p-4 bg-slate-50 dark:bg-[#0f1821] rounded-xl">
        <span class="text-xs text-slate-500 block mb-1">EDAŞ ID (Bits 0-4)</span>
        <span class="text-lg font-bold text-slate-900 dark:text-white">04 - AYDEM (ADM)</span>
      </div>
      <div class="p-4 bg-slate-50 dark:bg-[#0f1821] rounded-xl">
        <span class="text-xs text-slate-500 block mb-1">Trafo Merkez ID (Bits 5-19)</span>
        <span class="text-lg font-mono font-bold text-slate-900 dark:text-white">0</span>
      </div>
      <div class="p-4 bg-slate-50 dark:bg-[#0f1821] rounded-xl">
        <span class="text-xs text-slate-500 block mb-1">Trafo ID (Bits 20-23)</span>
        <span class="text-lg font-mono font-bold text-slate-900 dark:text-white">0</span>
      </div>
      <div class="p-4 bg-slate-50 dark:bg-[#0f1821] rounded-xl">
        <span class="text-xs text-slate-500 block mb-1">Depar ID (Bits 24-29)</span>
        <span class="text-lg font-mono font-bold text-slate-900 dark:text-white">0</span>
      </div>
      <div class="p-4 bg-slate-50 dark:bg-[#0f1821] rounded-xl">
        <span class="text-xs text-slate-500 block mb-1">Faz ID (Bits 30-31)</span>
        <span class="text-lg font-bold text-slate-900 dark:text-white">0 - N/A</span>
      </div>
      <div class="p-4 bg-slate-50 dark:bg-[#0f1821] rounded-xl">
        <span class="text-xs text-slate-500 block mb-1">Max Current (Bits 34-43)</span>
        <span class="text-lg font-mono font-bold text-slate-900 dark:text-white">0 A</span>
      </div>
    </div>
  </div>
</div>
