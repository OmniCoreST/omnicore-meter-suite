<script lang="ts">
  import Icon from "$lib/components/common/Icon.svelte";
  import { t, isConnected, meterStore } from "$lib/stores";
  import { exportToExcel } from "$lib/utils/export";

  let expandedSections = $state<Record<string, boolean>>({
    voltage: true,
    current: true,
    magnetic: true,
    cover: true,
    tariff: false,
  });

  function toggleSection(section: string) {
    expandedSections[section] = !expandedSections[section];
  }

  // Parse warnings data from raw OBIS codes
  let warningsData = $derived.by(() => {
    const data = $meterStore.shortReadData;
    // @ts-ignore
    if (!data || !data.rawData) {
      return {
        voltage: { count: 0, records: [] },
        current: { count: 0, records: [] },
        magnetic: { count: 0, records: [] },
        topCover: { count: 0, records: [] },
        terminalCover: { count: 0, history: [] },
        tariffChanges: { count: 0, records: [] },
      };
    }

    // @ts-ignore
    const raw = data.rawData;

    const parseWarningRecords = (baseCode: string, count: number) => {
      const records = [];
      for (let i = 1; i <= count; i++) {
        const match = raw.match(
          new RegExp(`${baseCode.replace(/\./g, '\\.')}\\*${i}\\(([^;]+);([^)]+)\\)`)
        );
        if (match) {
          records.push({
            id: i,
            start: match[1],
            end: match[2],
          });
        }
      }
      return records;
    };

    // Voltage warnings (96.7.4 count, 96.77.4*1-10 records)
    const voltageCount = raw.match(/96\.7\.4\((\d+)\)/);
    const voltage = parseWarningRecords('96.77.4', 10);

    // Current warnings (96.7.5 count, 96.77.5*1-10 records)
    const currentCount = raw.match(/96\.7\.5\((\d+)\)/);
    const current = parseWarningRecords('96.77.5', 10);

    // Magnetic field warnings (96.7.6 count, 96.77.6*1-10 records)
    const magneticCount = raw.match(/96\.7\.6\((\d+)\)/);
    const magnetic = parseWarningRecords('96.77.6', 10);

    // Top cover openings (96.70)
    const topCoverCount = raw.match(/96\.70\((\d+)\)/);

    // Terminal cover monthly history (96.71*1-12)
    const terminalHistory = [];
    for (let m = 1; m <= 12; m++) {
      const match = raw.match(new RegExp(`96\\.71\\*${m}\\((\\d+)\\)`));
      if (match) {
        terminalHistory.push({ month: m, count: parseInt(match[1]) });
      }
    }

    // Tariff changes (96.2.2*1-10)
    const tariffChanges = [];
    for (let i = 1; i <= 10; i++) {
      const match = raw.match(new RegExp(`96\\.2\\.2\\*${i}\\(([^)]+)\\)`));
      if (match) {
        tariffChanges.push({ id: i, timestamp: match[1] });
      }
    }

    return {
      voltage: { count: voltageCount ? parseInt(voltageCount[1]) : 0, records: voltage },
      current: { count: currentCount ? parseInt(currentCount[1]) : 0, records: current },
      magnetic: { count: magneticCount ? parseInt(magneticCount[1]) : 0, records: magnetic },
      topCover: { count: topCoverCount ? parseInt(topCoverCount[1]) : 0, records: [] },
      terminalCover: { count: terminalHistory.reduce((a, b) => a + b.count, 0), history: terminalHistory },
      tariffChanges: { count: tariffChanges.length, records: tariffChanges },
    };
  });

  function handleExport() {
    const allRecords: any[] = [];

    warningsData.voltage.records.forEach(r => {
      allRecords.push({ type: "Gerilim", id: r.id, start: r.start, end: r.end });
    });
    warningsData.current.records.forEach(r => {
      allRecords.push({ type: "Akim", id: r.id, start: r.start, end: r.end });
    });
    warningsData.magnetic.records.forEach(r => {
      allRecords.push({ type: "Manyetik", id: r.id, start: r.start, end: r.end });
    });

    exportToExcel(allRecords, "warnings", [
      { key: "type", label: $t.type },
      { key: "id", label: "#" },
      { key: "start", label: $t.startTime },
      { key: "end", label: $t.endTime },
    ]);
  }
</script>

<div class="space-y-6">
  <!-- Header -->
  <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm">
    <div class="flex items-start justify-between">
      <div>
        <h3 class="text-xl font-bold text-slate-900 dark:text-white mb-2">{$t.warnings}</h3>
        <p class="text-sm text-slate-500 dark:text-slate-400">{$t.warningsDescription}</p>
      </div>
      {#if $meterStore.shortReadData}
        <button
          onclick={handleExport}
          class="flex items-center gap-2 px-4 py-2 bg-emerald-600 hover:bg-emerald-700 text-white text-sm font-bold rounded-lg transition-colors"
        >
          <Icon name="download" size="sm" />
          {$t.exportToExcel}
        </button>
      {/if}
    </div>
  </div>

  {#if !$isConnected}
    <div class="bg-amber-500/10 border border-amber-500/20 rounded-xl p-6 text-center">
      <Icon name="warning" class="text-4xl text-amber-500 mb-3" />
      <p class="text-amber-600 dark:text-amber-400 font-medium">{$t.connectFirstWarning}</p>
    </div>
  {:else}
    <!-- Summary Cards -->
    <div class="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-6 gap-4">
      <div class="bg-amber-500/10 border border-amber-500/20 rounded-xl p-4 text-center">
        <Icon name="flash_on" class="text-amber-500 text-2xl mb-2" />
        <div class="text-2xl font-bold text-amber-600">{warningsData.voltage.count}</div>
        <div class="text-xs text-slate-500">{$t.voltageWarnings}</div>
      </div>
      <div class="bg-blue-500/10 border border-blue-500/20 rounded-xl p-4 text-center">
        <Icon name="bolt" class="text-blue-500 text-2xl mb-2" />
        <div class="text-2xl font-bold text-blue-600">{warningsData.current.count}</div>
        <div class="text-xs text-slate-500">{$t.currentWarnings}</div>
      </div>
      <div class="bg-red-500/10 border border-red-500/20 rounded-xl p-4 text-center">
        <Icon name="sensors" class="text-red-500 text-2xl mb-2" />
        <div class="text-2xl font-bold text-red-600">{warningsData.magnetic.count}</div>
        <div class="text-xs text-slate-500">{$t.magneticField}</div>
      </div>
      <div class="bg-orange-500/10 border border-orange-500/20 rounded-xl p-4 text-center">
        <Icon name="door_open" class="text-orange-500 text-2xl mb-2" />
        <div class="text-2xl font-bold text-orange-600">{warningsData.topCover.count}</div>
        <div class="text-xs text-slate-500">{$t.topCoverOpen}</div>
      </div>
      <div class="bg-purple-500/10 border border-purple-500/20 rounded-xl p-4 text-center">
        <Icon name="sensor_door" class="text-purple-500 text-2xl mb-2" />
        <div class="text-2xl font-bold text-purple-600">{warningsData.terminalCover.count}</div>
        <div class="text-xs text-slate-500">{$t.terminalCover}</div>
      </div>
      <div class="bg-slate-500/10 border border-slate-500/20 rounded-xl p-4 text-center">
        <Icon name="swap_horiz" class="text-slate-500 text-2xl mb-2" />
        <div class="text-2xl font-bold text-slate-600">{warningsData.tariffChanges.count}</div>
        <div class="text-xs text-slate-500">{$t.tariffChange}</div>
      </div>
    </div>

    <!-- Voltage Warnings Section -->
    <div class="bg-gradient-to-br from-amber-50 to-orange-50 dark:from-amber-900/20 dark:to-orange-900/20 rounded-xl border border-amber-200 dark:border-amber-800 overflow-hidden">
      <button
        onclick={() => toggleSection('voltage')}
        class="w-full flex items-center justify-between p-4 hover:bg-amber-100/50 dark:hover:bg-amber-900/30 transition-colors"
      >
        <div class="flex items-center gap-3">
          <Icon name="flash_on" class="text-amber-500" />
          <span class="font-bold text-slate-900 dark:text-white">{$t.voltageWarningsSection}</span>
          <span class="px-2 py-1 bg-amber-500/20 text-amber-600 dark:text-amber-400 rounded-full text-xs font-bold">
            {warningsData.voltage.count}
          </span>
        </div>
        <Icon name={expandedSections.voltage ? "expand_less" : "expand_more"} class="text-slate-400" />
      </button>

      {#if expandedSections.voltage}
        <div class="p-4 pt-0">
          {#if warningsData.voltage.records.length > 0}
            <table class="w-full text-sm">
              <thead class="bg-amber-100/50 dark:bg-amber-900/30">
                <tr class="border-b border-amber-200 dark:border-amber-800">
                  <th class="px-4 py-2 text-left font-bold text-slate-700 dark:text-slate-300">#</th>
                  <th class="px-4 py-2 text-left font-bold text-slate-700 dark:text-slate-300">{$t.startTime}</th>
                  <th class="px-4 py-2 text-left font-bold text-slate-700 dark:text-slate-300">{$t.endTime}</th>
                </tr>
              </thead>
              <tbody>
                {#each warningsData.voltage.records as record}
                  <tr class="border-b border-amber-100 dark:border-amber-900/30">
                    <td class="px-4 py-2 font-mono text-slate-600 dark:text-slate-400">{record.id}</td>
                    <td class="px-4 py-2 font-mono text-slate-900 dark:text-white">{record.start}</td>
                    <td class="px-4 py-2 font-mono text-slate-900 dark:text-white">{record.end}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          {:else}
            <p class="text-center text-slate-500 py-4">{$t.noWarnings}</p>
          {/if}
        </div>
      {/if}
    </div>

    <!-- Current Warnings Section -->
    <div class="bg-gradient-to-br from-blue-50 to-cyan-50 dark:from-blue-900/20 dark:to-cyan-900/20 rounded-xl border border-blue-200 dark:border-blue-800 overflow-hidden">
      <button
        onclick={() => toggleSection('current')}
        class="w-full flex items-center justify-between p-4 hover:bg-blue-100/50 dark:hover:bg-blue-900/30 transition-colors"
      >
        <div class="flex items-center gap-3">
          <Icon name="bolt" class="text-blue-500" />
          <span class="font-bold text-slate-900 dark:text-white">{$t.currentWarningsSection}</span>
          <span class="px-2 py-1 bg-blue-500/20 text-blue-600 dark:text-blue-400 rounded-full text-xs font-bold">
            {warningsData.current.count}
          </span>
        </div>
        <Icon name={expandedSections.current ? "expand_less" : "expand_more"} class="text-slate-400" />
      </button>

      {#if expandedSections.current}
        <div class="p-4 pt-0">
          {#if warningsData.current.records.length > 0}
            <table class="w-full text-sm">
              <thead class="bg-blue-100/50 dark:bg-blue-900/30">
                <tr class="border-b border-blue-200 dark:border-blue-800">
                  <th class="px-4 py-2 text-left font-bold text-slate-700 dark:text-slate-300">#</th>
                  <th class="px-4 py-2 text-left font-bold text-slate-700 dark:text-slate-300">{$t.startTime}</th>
                  <th class="px-4 py-2 text-left font-bold text-slate-700 dark:text-slate-300">{$t.endTime}</th>
                </tr>
              </thead>
              <tbody>
                {#each warningsData.current.records as record}
                  <tr class="border-b border-blue-100 dark:border-blue-900/30">
                    <td class="px-4 py-2 font-mono text-slate-600 dark:text-slate-400">{record.id}</td>
                    <td class="px-4 py-2 font-mono text-slate-900 dark:text-white">{record.start}</td>
                    <td class="px-4 py-2 font-mono text-slate-900 dark:text-white">{record.end}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          {:else}
            <p class="text-center text-slate-500 py-4">{$t.noWarnings}</p>
          {/if}
        </div>
      {/if}
    </div>

    <!-- Magnetic Field Warnings Section -->
    <div class="bg-gradient-to-br from-red-50 to-rose-50 dark:from-red-900/20 dark:to-rose-900/20 rounded-xl border border-red-200 dark:border-red-800 overflow-hidden">
      <button
        onclick={() => toggleSection('magnetic')}
        class="w-full flex items-center justify-between p-4 hover:bg-red-100/50 dark:hover:bg-red-900/30 transition-colors"
      >
        <div class="flex items-center gap-3">
          <Icon name="sensors" class="text-red-500" />
          <span class="font-bold text-slate-900 dark:text-white">{$t.magneticWarnings}</span>
          <span class="px-2 py-1 bg-red-500/20 text-red-600 dark:text-red-400 rounded-full text-xs font-bold">
            {warningsData.magnetic.count}
          </span>
        </div>
        <Icon name={expandedSections.magnetic ? "expand_less" : "expand_more"} class="text-slate-400" />
      </button>

      {#if expandedSections.magnetic}
        <div class="p-4 pt-0">
          {#if warningsData.magnetic.records.length > 0}
            <table class="w-full text-sm">
              <thead class="bg-red-100/50 dark:bg-red-900/30">
                <tr class="border-b border-red-200 dark:border-red-800">
                  <th class="px-4 py-2 text-left font-bold text-slate-700 dark:text-slate-300">#</th>
                  <th class="px-4 py-2 text-left font-bold text-slate-700 dark:text-slate-300">{$t.startTime}</th>
                  <th class="px-4 py-2 text-left font-bold text-slate-700 dark:text-slate-300">{$t.endTime}</th>
                </tr>
              </thead>
              <tbody>
                {#each warningsData.magnetic.records as record}
                  <tr class="border-b border-red-100 dark:border-red-900/30">
                    <td class="px-4 py-2 font-mono text-slate-600 dark:text-slate-400">{record.id}</td>
                    <td class="px-4 py-2 font-mono text-slate-900 dark:text-white">{record.start}</td>
                    <td class="px-4 py-2 font-mono text-slate-900 dark:text-white">{record.end}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          {:else}
            <p class="text-center text-slate-500 py-4">{$t.noWarnings}</p>
          {/if}
        </div>
      {/if}
    </div>

    <!-- Cover Warnings Section -->
    <div class="bg-gradient-to-br from-orange-50 to-amber-50 dark:from-orange-900/20 dark:to-amber-900/20 rounded-xl border border-orange-200 dark:border-orange-800 overflow-hidden">
      <button
        onclick={() => toggleSection('cover')}
        class="w-full flex items-center justify-between p-4 hover:bg-orange-100/50 dark:hover:bg-orange-900/30 transition-colors"
      >
        <div class="flex items-center gap-3">
          <Icon name="door_open" class="text-orange-500" />
          <span class="font-bold text-slate-900 dark:text-white">{$t.coverWarnings}</span>
        </div>
        <Icon name={expandedSections.cover ? "expand_less" : "expand_more"} class="text-slate-400" />
      </button>

      {#if expandedSections.cover}
        <div class="p-4 pt-0">
          <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <!-- Top Cover -->
            <div class="p-4 bg-white/50 dark:bg-black/20 rounded-lg">
              <div class="flex items-center gap-2 mb-2">
                <Icon name="door_open" class="text-orange-500" size="sm" />
                <span class="font-bold text-slate-900 dark:text-white">{$t.topCoverOpen}</span>
              </div>
              <div class="text-3xl font-mono font-bold text-orange-600">{warningsData.topCover.count}</div>
              <div class="text-xs text-slate-500">toplam acilma</div>
            </div>

            <!-- Terminal Cover History -->
            <div class="p-4 bg-white/50 dark:bg-black/20 rounded-lg">
              <div class="flex items-center gap-2 mb-2">
                <Icon name="sensor_door" class="text-purple-500" size="sm" />
                <span class="font-bold text-slate-900 dark:text-white">{$t.terminalCover}</span>
              </div>
              {#if warningsData.terminalCover.history.length > 0}
                <div class="grid grid-cols-6 gap-1 mt-2">
                  {#each warningsData.terminalCover.history as h}
                    <div class="text-center p-1 bg-purple-500/10 rounded text-xs">
                      <div class="font-bold text-purple-600">{h.count}</div>
                      <div class="text-[10px] text-slate-500">{h.month}.Ay</div>
                    </div>
                  {/each}
                </div>
              {:else}
                <div class="text-3xl font-mono font-bold text-purple-600">{warningsData.terminalCover.count}</div>
              {/if}
            </div>
          </div>
        </div>
      {/if}
    </div>

    <!-- Tariff Changes Section -->
    <div class="bg-gradient-to-br from-slate-50 to-gray-50 dark:from-slate-900/40 dark:to-gray-900/40 rounded-xl border border-slate-200 dark:border-slate-700 overflow-hidden">
      <button
        onclick={() => toggleSection('tariff')}
        class="w-full flex items-center justify-between p-4 hover:bg-slate-100/50 dark:hover:bg-slate-800/30 transition-colors"
      >
        <div class="flex items-center gap-3">
          <Icon name="swap_horiz" class="text-slate-500" />
          <span class="font-bold text-slate-900 dark:text-white">{$t.tariffChanges}</span>
          <span class="px-2 py-1 bg-slate-500/20 text-slate-600 dark:text-slate-400 rounded-full text-xs font-bold">
            {warningsData.tariffChanges.count}
          </span>
        </div>
        <Icon name={expandedSections.tariff ? "expand_less" : "expand_more"} class="text-slate-400" />
      </button>

      {#if expandedSections.tariff}
        <div class="p-4 pt-0">
          {#if warningsData.tariffChanges.records.length > 0}
            <div class="grid grid-cols-2 md:grid-cols-5 gap-2">
              {#each warningsData.tariffChanges.records as change}
                <div class="p-3 bg-white/50 dark:bg-black/20 rounded-lg text-center">
                  <div class="text-xs text-slate-500 mb-1">#{change.id}</div>
                  <div class="text-sm font-mono text-slate-900 dark:text-white">{change.timestamp}</div>
                </div>
              {/each}
            </div>
          {:else}
            <p class="text-center text-slate-500 py-4">Tarife degisiklik kaydi yok</p>
          {/if}
        </div>
      {/if}
    </div>
  {/if}
</div>
