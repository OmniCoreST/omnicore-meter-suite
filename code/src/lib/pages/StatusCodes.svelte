<script lang="ts">
  import Icon from "$lib/components/common/Icon.svelte";
  import { t, isConnected, meterStore } from "$lib/stores";
  import { exportToExcel } from "$lib/utils/export";

  let activeTab = $state<"ff" | "gf">("ff");
  let showActiveOnly = $state(false);

  // FF Code bit definitions - which bits are urgent/critical
  const ffBitUrgent = [0, 1, 2, 6, 8, 9, 10, 11, 12, 13, 37, 38, 55];

  // Parse FF code from meter data
  let ffBits = $derived.by(() => {
    const data = $meterStore.shortReadData;
    if (!data || !data.ffCode) return [];

    try {
      const ffValue = BigInt("0x" + data.ffCode);

      const bits = [];
      for (let i = 0; i < 56; i++) {
        const isSet = (ffValue & (1n << BigInt(i))) !== 0n;
        bits.push({
          bit: i,
          key: `ffBit${i}`,
          status: isSet,
          urgent: ffBitUrgent.includes(i),
        });
      }
      return bits;
    } catch {
      return [];
    }
  });

  let filteredBits = $derived(
    showActiveOnly ? ffBits.filter(b => b.status) : ffBits
  );

  // Parse GF code fields
  let gfFields = $derived.by(() => {
    const data = $meterStore.shortReadData;
    if (!data || !data.gfCode) {
      return {
        edasId: 0,
        edasName: "Bilinmiyor",
        trafoMerkez: 0,
        trafoId: 0,
        deparId: 0,
        fazId: 0,
        fazName: "N/A",
        maxCurrent: 0,
      };
    }

    try {
      const gfValue = BigInt("0x" + data.gfCode);

      // EDAŞ ID (bits 0-4)
      const edasId = Number(gfValue & 0x1Fn);
      const edasNames: Record<number, string> = {
        0: "Belirtilmemis",
        1: "BASKENT (BEDAS)",
        2: "ISTANBUL (AYEDAS)",
        3: "SAKARYA (SEDAS)",
        4: "AYDEM (ADM)",
        5: "GEDIZ (GDZ)",
        6: "TOROSLAR (TREDAS)",
        7: "AKDENIZ (AKEDAS)",
        8: "ULUDAG (UEDAS)",
        9: "TRAKYA (TEDAS)",
        10: "DICLE (DEDAS)",
        11: "VANGOLU (VEDAS)",
        12: "FIRAT (FDAS)",
        13: "CAMLIBEL (CEDAS)",
        14: "MERAM (MEDAS)",
        15: "YESILIRMAK (YEDAS)",
        16: "CORUH (CUDAS)",
        17: "OSMANGAZI (OEDAS)",
        18: "BOGAZICI (BEDAS)",
        19: "ARAS (ARAS)",
        20: "KAYSERI (KEAS)",
        21: "ENERJISA (ESA)",
      };

      // Trafo Merkez ID (bits 5-19)
      const trafoMerkez = Number((gfValue >> 5n) & 0x7FFFn);

      // Trafo ID (bits 20-23)
      const trafoId = Number((gfValue >> 20n) & 0xFn);

      // Depar ID (bits 24-29)
      const deparId = Number((gfValue >> 24n) & 0x3Fn);

      // Faz ID (bits 30-31)
      const fazId = Number((gfValue >> 30n) & 0x3n);
      const fazNames = ["N/A", "R (L1)", "S (L2)", "T (L3)"];

      // Max Current (bits 34-43)
      const maxCurrent = Number((gfValue >> 34n) & 0x3FFn);

      return {
        edasId,
        edasName: edasNames[edasId] || `ID: ${edasId}`,
        trafoMerkez,
        trafoId,
        deparId,
        fazId,
        fazName: fazNames[fazId] || "N/A",
        maxCurrent,
      };
    } catch {
      return {
        edasId: 0,
        edasName: "Parse Hatasi",
        trafoMerkez: 0,
        trafoId: 0,
        deparId: 0,
        fazId: 0,
        fazName: "N/A",
        maxCurrent: 0,
      };
    }
  });

  function getBitName(key: string): string {
    return ($t as Record<string, string>)[key] || key;
  }

  function handleExportFF() {
    const exportData = ffBits.map(bit => ({
      bit: bit.bit,
      name: getBitName(bit.key),
      status: bit.status ? $t.active : $t.ok,
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
      { field: $t.edasIdField, value: `${gfFields.edasId} - ${gfFields.edasName}` },
      { field: $t.trafoMerkezField, value: gfFields.trafoMerkez.toString() },
      { field: $t.trafoIdField, value: gfFields.trafoId.toString() },
      { field: $t.deparIdField, value: gfFields.deparId.toString() },
      { field: $t.fazIdField, value: `${gfFields.fazId} - ${gfFields.fazName}` },
      { field: $t.maxCurrentField, value: `${gfFields.maxCurrent} A` },
    ];

    exportToExcel(exportData, "gf_codes", [
      { key: "field", label: "Alan" },
      { key: "value", label: "Deger" },
    ]);
  }
</script>

<div class="space-y-6">
  <!-- Header -->
  <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm">
    <div class="flex items-start justify-between">
      <div>
        <h3 class="text-xl font-bold text-slate-900 dark:text-white mb-2">{$t.statusCodes}</h3>
        <p class="text-sm text-slate-500 dark:text-slate-400">{$t.statusCodesDescription}</p>
      </div>
    </div>
  </div>

  {#if !$isConnected}
    <div class="bg-amber-500/10 border border-amber-500/20 rounded-xl p-6 text-center">
      <Icon name="warning" class="text-4xl text-amber-500 mb-3" />
      <p class="text-amber-600 dark:text-amber-400 font-medium">{$t.connectFirstWarning}</p>
    </div>
  {:else}
    <!-- Tab Selection -->
    <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-2 shadow-sm">
      <div class="flex gap-2">
        <button
          onclick={() => activeTab = "ff"}
          class="flex-1 flex items-center justify-center gap-2 px-4 py-3 rounded-lg font-bold transition-colors
            {activeTab === 'ff' ? 'bg-primary text-white' : 'text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-[#334a5e]'}"
        >
          <Icon name="report_problem" size="sm" />
          {$t.ffCodeTab}
        </button>
        <button
          onclick={() => activeTab = "gf"}
          class="flex-1 flex items-center justify-center gap-2 px-4 py-3 rounded-lg font-bold transition-colors
            {activeTab === 'gf' ? 'bg-primary text-white' : 'text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-[#334a5e]'}"
        >
          <Icon name="location_on" size="sm" />
          {$t.gfCodeTab}
        </button>
      </div>
    </div>

    {#if activeTab === "ff"}
      <!-- FF Codes Tab -->
      <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm">
        <div class="flex items-center justify-between mb-6">
          <div>
            <h4 class="font-bold text-slate-900 dark:text-white flex items-center gap-2">
              <Icon name="report_problem" class="text-primary" />
              {$t.ffBits}
              <span class="text-xs font-mono font-normal text-slate-400 ml-1">(OBIS: F.F.0)</span>
            </h4>
            <div class="mt-2 p-3 bg-slate-50 dark:bg-[#0f1821] rounded-lg inline-block">
              <span class="text-xs text-slate-500 mr-2">{$t.hexValue}:</span>
              <span class="font-mono text-sm text-slate-900 dark:text-white">
                0x{$meterStore.shortReadData?.ffCode || "0000000000000000"}
              </span>
            </div>
          </div>
          <div class="flex items-center gap-3">
            <label class="flex items-center gap-2 cursor-pointer">
              <input
                type="checkbox"
                bind:checked={showActiveOnly}
                class="w-4 h-4 rounded border-slate-300 dark:border-[#334a5e] text-primary focus:ring-primary"
              />
              <span class="text-sm font-medium text-slate-600 dark:text-slate-400">{$t.showActiveOnly}</span>
            </label>
            <button
              onclick={handleExportFF}
              class="flex items-center gap-2 px-4 py-2 bg-emerald-600 hover:bg-emerald-700 text-white text-sm font-bold rounded-lg transition-colors"
            >
              <Icon name="download" size="sm" />
              {$t.exportToExcel}
            </button>
          </div>
        </div>

        <!-- Compact Bitfield Row -->
        <div class="mb-6 p-3 bg-slate-50 dark:bg-[#0f1821] rounded-lg flex items-center gap-4 overflow-x-auto">
          <span class="text-xs font-bold text-slate-500 whitespace-nowrap">{$t.bitField}:</span>
          <div class="flex gap-0.5">
            {#each ffBits as bit}
              <div
                class="w-3 h-5 rounded-sm cursor-help transition-all
                  {bit.status
                    ? bit.urgent
                      ? 'bg-red-500'
                      : 'bg-amber-500'
                    : 'bg-slate-200 dark:bg-[#334a5e]'}"
                title="{bit.bit}: {getBitName(bit.key)}"
              ></div>
            {/each}
          </div>
          <div class="flex items-center gap-3 text-[10px] ml-auto whitespace-nowrap">
            <div class="flex items-center gap-1">
              <div class="w-3 h-3 rounded-sm bg-slate-200 dark:bg-[#334a5e]"></div>
              <span class="text-slate-500">{$t.ok}</span>
            </div>
            <div class="flex items-center gap-1">
              <div class="w-3 h-3 rounded-sm bg-amber-500"></div>
              <span class="text-slate-500">{$t.active}</span>
            </div>
            <div class="flex items-center gap-1">
              <div class="w-3 h-3 rounded-sm bg-red-500"></div>
              <span class="text-slate-500">{$t.urgent}</span>
            </div>
          </div>
        </div>

        <!-- Decoded Table -->
        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-3">
          {#each filteredBits as bit}
            <div
              class="p-3 rounded-lg border transition-colors
                {bit.status
                ? bit.urgent
                  ? 'bg-red-500/10 border-red-500/20'
                  : 'bg-amber-500/10 border-amber-500/20'
                : 'bg-slate-50 dark:bg-[#0f1821] border-slate-200 dark:border-[#334a5e]'}"
            >
              <div class="flex items-center justify-between mb-1">
                <span class="text-xs font-mono text-slate-500">Bit {bit.bit}</span>
                {#if bit.urgent}
                  <span class="text-[10px] font-bold text-red-500 uppercase">{$t.urgent}</span>
                {/if}
              </div>
              <div class="text-sm font-medium text-slate-900 dark:text-white mb-2 line-clamp-2">
                {getBitName(bit.key)}
              </div>
              <div class="flex items-center gap-2">
                <div
                  class="size-2 rounded-full {bit.status
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
    {:else}
      <!-- GF Codes Tab -->
      <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm">
        <div class="flex items-center justify-between mb-6">
          <div>
            <h4 class="font-bold text-slate-900 dark:text-white flex items-center gap-2">
              <Icon name="location_on" class="text-primary" />
              {$t.gfFields}
              <span class="text-xs font-mono font-normal text-slate-400 ml-1">(OBIS: F.F.1)</span>
            </h4>
            <div class="mt-2 p-3 bg-slate-50 dark:bg-[#0f1821] rounded-lg inline-block">
              <span class="text-xs text-slate-500 mr-2">{$t.hexValue}:</span>
              <span class="font-mono text-sm text-slate-900 dark:text-white">
                0x{$meterStore.shortReadData?.gfCode || "0000000000000000"}
              </span>
            </div>
          </div>
          <button
            onclick={handleExportGF}
            class="flex items-center gap-2 px-4 py-2 bg-emerald-600 hover:bg-emerald-700 text-white text-sm font-bold rounded-lg transition-colors"
          >
            <Icon name="download" size="sm" />
            {$t.exportToExcel}
          </button>
        </div>

        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          <!-- EDAŞ ID -->
          <div class="p-4 bg-gradient-to-br from-primary/10 to-emerald-500/10 rounded-xl border border-primary/20">
            <span class="text-xs text-slate-500 block mb-1">{$t.edasIdField} (Bits 0-4)</span>
            <span class="text-lg font-bold text-slate-900 dark:text-white">{gfFields.edasId}</span>
            <span class="text-sm text-primary ml-2">- {gfFields.edasName}</span>
          </div>

          <!-- Trafo Merkez ID -->
          <div class="p-4 bg-slate-50 dark:bg-[#0f1821] rounded-xl">
            <span class="text-xs text-slate-500 block mb-1">{$t.trafoMerkezField} (Bits 5-19)</span>
            <span class="text-lg font-mono font-bold text-slate-900 dark:text-white">{gfFields.trafoMerkez}</span>
          </div>

          <!-- Trafo ID -->
          <div class="p-4 bg-slate-50 dark:bg-[#0f1821] rounded-xl">
            <span class="text-xs text-slate-500 block mb-1">{$t.trafoIdField} (Bits 20-23)</span>
            <span class="text-lg font-mono font-bold text-slate-900 dark:text-white">{gfFields.trafoId}</span>
          </div>

          <!-- Depar ID -->
          <div class="p-4 bg-slate-50 dark:bg-[#0f1821] rounded-xl">
            <span class="text-xs text-slate-500 block mb-1">{$t.deparIdField} (Bits 24-29)</span>
            <span class="text-lg font-mono font-bold text-slate-900 dark:text-white">{gfFields.deparId}</span>
          </div>

          <!-- Faz ID -->
          <div class="p-4 bg-slate-50 dark:bg-[#0f1821] rounded-xl">
            <span class="text-xs text-slate-500 block mb-1">{$t.fazIdField} (Bits 30-31)</span>
            <span class="text-lg font-bold text-slate-900 dark:text-white">{gfFields.fazId}</span>
            <span class="text-sm text-slate-500 ml-2">- {gfFields.fazName}</span>
          </div>

          <!-- Max Current -->
          <div class="p-4 bg-slate-50 dark:bg-[#0f1821] rounded-xl">
            <span class="text-xs text-slate-500 block mb-1">{$t.maxCurrentField} (Bits 34-43)</span>
            <span class="text-lg font-mono font-bold text-slate-900 dark:text-white">{gfFields.maxCurrent} A</span>
          </div>
        </div>
      </div>
    {/if}
  {/if}
</div>
