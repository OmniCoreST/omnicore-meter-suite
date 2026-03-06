<script lang="ts">
  import Icon from "$lib/components/common/Icon.svelte";
  import { t, isConnected, meterStore, isMeterReading, addLog } from "$lib/stores";
  import { exportToExcel } from "$lib/utils/export";
  import { readPacket } from "$lib/utils/tauri";

  let modeRawData = $state<string | null>(null);
  let isReadingMode = $state(false);

  async function readModePacket() {
    if (isReadingMode || $isMeterReading) return;
    isReadingMode = true;
    meterStore.setReading(true);
    try {
      addLog("info", "Mod 8 (Uyarılar) paketi okunuyor...");
      const result = await readPacket(8);
      modeRawData = result.rawData;
      addLog("success", `Mod 8 okuma tamamlandı: ${result.bytesRead} byte, ${(result.readDurationMs / 1000).toFixed(1)}s`);
    } catch (e) {
      addLog("error", `Mod 8 okuma hatası: ${e}`);
    } finally {
      isReadingMode = false;
      meterStore.setReading(false);
    }
  }

  let expandedSections = $state<Record<string, boolean>>({
    voltage: true,
    current: true,
    magnetic: true,
    cover: true,
    reset: false,
    neutralVoltage: false,
    tariff: false,
  });

  function toggleSection(section: string) {
    expandedSections[section] = !expandedSections[section];
  }

  // Parse warnings data from raw OBIS codes
  let warningsData = $derived.by(() => {
    const data = $meterStore.shortReadData;
    // @ts-ignore
    const raw: string | null = modeRawData || (data && data.rawData) || null;
    if (!raw) {
      return {
        voltage: { count: 0, records: [] as {id: number, start: string, end: string}[] },
        current: { count: 0, records: [] as {id: number, start: string, end: string}[] },
        magnetic: { count: 0, records: [] as {id: number, start: string, end: string}[], duration: "" },
        topCover: { count: 0, records: [] as {id: number, start: string, end: string}[] },
        terminalCover: { count: 0, records: [] as {id: number, start: string, end: string}[] },
        reset: { count: 0, records: [] as {id: number, start: string, end: string}[] },
        neutralVoltage: { count: 0, records: [] as {id: number, start: string, end: string}[] },
        tariffChanges: { count: 0, records: [] as {id: number, timestamp: string}[] },
      };
    }

    const parseWarningRecords = (baseCode: string, count: number) => {
      const records = [];

      // Base record (aktif/devam eden olay): 96.20.16(start;end)
      const baseMatch = raw.match(
        new RegExp(`${baseCode.replace(/\./g, '\\.')}\\(([^;)]+);([^)]+)\\)`)
      );
      if (baseMatch && !baseMatch[1].startsWith('00-00-00')) {
        records.push({ id: 0, start: baseMatch[1], end: baseMatch[2] });
      }

      // İndeksli kayıtlar: 96.20.16*1(start;end) ... *N
      for (let i = 1; i <= count; i++) {
        const match = raw.match(
          new RegExp(`${baseCode.replace(/\./g, '\\.')}\\*${i}\\(([^;]+);([^)]+)\\)`)
        );
        if (match) {
          records.push({ id: i, start: match[1], end: match[2] });
        }
      }
      return records;
    };

    // Voltage warnings (96.77.2 count, 96.77.20*1-10 records) — MMS reference
    const voltageCount = raw.match(/96\.77\.2\((\d+)\)/);
    const voltage = parseWarningRecords('96.77.20', 10);

    // Current warnings (96.77.3 count, 96.77.30*1-10 records) — MMS reference
    const currentCount = raw.match(/96\.77\.3\((\d+)\)/);
    const current = parseWarningRecords('96.77.30', 10);

    // Magnetic field warnings (96.20.15 count, 96.20.16*1-10 records) — MMS reference
    const magneticCount = raw.match(/96\.20\.15\((\d+)\)/);
    const magnetic = parseWarningRecords('96.20.16', 10);

    // Magnetic field total duration (96.20.18)
    const magneticDurationMatch = raw.match(/96\.20\.18\(([^)]+)\)/);
    const magneticDuration = magneticDurationMatch ? magneticDurationMatch[1] : "";

    // Top cover openings (96.20.0 count, 96.20.1*1-10 records) per TEDAŞ spec
    const topCoverCount = raw.match(/96\.20\.0\((\d+)\)/);
    const topCoverRecords = parseWarningRecords('96.20.1', 10);

    // Terminal cover openings (96.20.5 count, 96.20.6*1-24 records) per TEDAŞ spec
    const terminalCoverCount = raw.match(/96\.20\.5\((\d+)\)/);
    const terminalCoverRecords = parseWarningRecords('96.20.6', 24);

    // Reset count (96.11.0 count, 96.11.1*1-10 records) — MMS Category G
    const resetCount = raw.match(/96\.11\.0\((\d+)\)/);
    const resetRecords = parseWarningRecords('96.11.1', 10);

    // Neutral voltage warnings (96.20.26*1-10 records) — TEDAŞ spec
    const neutralVoltageRecords = parseWarningRecords('96.20.26', 10);

    // Tariff changes (96.2.2*1-10), skip null dates
    const tariffChanges = [];
    for (let i = 1; i <= 10; i++) {
      const match = raw.match(new RegExp(`96\\.2\\.2\\*${i}\\(([^)]+)\\)`));
      if (match && !match[1].startsWith("00-00-00")) {
        tariffChanges.push({ id: i, timestamp: match[1] });
      }
    }

    return {
      voltage: { count: voltageCount ? parseInt(voltageCount[1]) : 0, records: voltage },
      current: { count: currentCount ? parseInt(currentCount[1]) : 0, records: current },
      magnetic: { count: magneticCount ? parseInt(magneticCount[1]) : 0, records: magnetic, duration: magneticDuration },
      topCover: { count: topCoverCount ? parseInt(topCoverCount[1]) : 0, records: topCoverRecords },
      terminalCover: { count: terminalCoverCount ? parseInt(terminalCoverCount[1]) : 0, records: terminalCoverRecords },
      reset: { count: resetCount ? parseInt(resetCount[1]) : 0, records: resetRecords },
      neutralVoltage: { count: neutralVoltageRecords.length, records: neutralVoltageRecords },
      tariffChanges: { count: tariffChanges.length, records: tariffChanges },
    };
  });

  async function handleExport() {
    try {
      const allRecords: any[] = [];

      warningsData?.voltage?.records?.forEach(r => {
        allRecords.push({ type: "Gerilim", id: r.id, start: r.start, end: r.end });
      });
      warningsData?.current?.records?.forEach(r => {
        allRecords.push({ type: "Akim", id: r.id, start: r.start, end: r.end });
      });
      warningsData?.magnetic?.records?.forEach(r => {
        allRecords.push({ type: "Manyetik", id: r.id, start: r.start, end: r.end });
      });

      if (allRecords.length === 0) { alert("Uyarı kaydı bulunamadı"); return; }

      await exportToExcel(allRecords, "warnings", [
        { key: "type", label: $t.type },
        { key: "id", label: "#" },
        { key: "start", label: $t.startTime },
        { key: "end", label: $t.endTime },
      ]);
    } catch (e) { alert("Export hatası: " + e); }
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
      <div class="flex items-center gap-2">
        {#if $isConnected}
          <button
            onclick={readModePacket}
            disabled={isReadingMode}
            class="flex items-center gap-2 px-4 py-2 bg-primary hover:bg-primary/90 disabled:opacity-50 disabled:cursor-not-allowed text-white text-sm font-bold rounded-lg transition-colors"
          >
            <Icon name="sync" size="sm" class={isReadingMode ? "animate-spin-reverse" : ""} />
            {isReadingMode ? $t.readingPacket : $t.readWarnings}
          </button>
        {/if}
        {#if $meterStore.shortReadData || modeRawData}
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
    {#if modeRawData}
      <div class="text-xs text-primary font-medium mt-2">{$t.dataSourceMode8}</div>
    {/if}
  </div>

  {#if !$isConnected}
    <div class="bg-amber-500/10 border border-amber-500/20 rounded-xl p-6 text-center">
      <Icon name="warning" class="text-4xl text-amber-500 mb-3" />
      <p class="text-amber-600 dark:text-amber-400 font-medium">{$t.connectFirstWarning}</p>
    </div>
  {:else}
    <!-- Summary Cards -->
    <div class="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-8 gap-4">
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
      <div class="bg-teal-500/10 border border-teal-500/20 rounded-xl p-4 text-center">
        <Icon name="restart_alt" class="text-teal-500 text-2xl mb-2" />
        <div class="text-2xl font-bold text-teal-600">{warningsData.reset.count}</div>
        <div class="text-xs text-slate-500">{$t.resetCount || "Reset"}</div>
      </div>
      <div class="bg-cyan-500/10 border border-cyan-500/20 rounded-xl p-4 text-center">
        <Icon name="electrical_services" class="text-cyan-500 text-2xl mb-2" />
        <div class="text-2xl font-bold text-cyan-600">{warningsData.neutralVoltage.count}</div>
        <div class="text-xs text-slate-500">{$t.neutralVoltage || "Nötr Gerilim"}</div>
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
                  <tr class="border-b border-amber-100 dark:border-amber-900/30 {record.id === 0 ? 'bg-amber-50 dark:bg-amber-900/20' : ''}">
                    <td class="px-4 py-2 font-mono text-slate-600 dark:text-slate-400">
                      {#if record.id === 0}
                        <span class="px-2 py-0.5 bg-amber-500 text-white text-xs rounded-full font-bold">Aktif</span>
                      {:else}
                        {record.id}
                      {/if}
                    </td>
                    <td class="px-4 py-2 font-mono text-slate-900 dark:text-white">{record.start}</td>
                    <td class="px-4 py-2 font-mono text-slate-900 dark:text-white">{record.end || '—'}</td>
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
                  <tr class="border-b border-blue-100 dark:border-blue-900/30 {record.id === 0 ? 'bg-blue-50 dark:bg-blue-900/20' : ''}">
                    <td class="px-4 py-2 font-mono text-slate-600 dark:text-slate-400">
                      {#if record.id === 0}
                        <span class="px-2 py-0.5 bg-blue-500 text-white text-xs rounded-full font-bold">Aktif</span>
                      {:else}
                        {record.id}
                      {/if}
                    </td>
                    <td class="px-4 py-2 font-mono text-slate-900 dark:text-white">{record.start}</td>
                    <td class="px-4 py-2 font-mono text-slate-900 dark:text-white">{record.end || '—'}</td>
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
          {#if warningsData.magnetic.duration}
            <div class="mb-3 p-3 bg-red-100/50 dark:bg-red-900/20 rounded-lg flex items-center gap-2">
              <Icon name="timer" size="sm" class="text-red-500" />
              <span class="text-sm text-slate-600 dark:text-slate-400">{$t.totalDuration || "Toplam Süre"}:</span>
              <span class="text-sm font-mono font-bold text-red-600 dark:text-red-400">{warningsData.magnetic.duration}</span>
            </div>
          {/if}
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
                  <tr class="border-b border-red-100 dark:border-red-900/30 {record.id === 0 ? 'bg-red-50 dark:bg-red-900/20' : ''}">
                    <td class="px-4 py-2 font-mono text-slate-600 dark:text-slate-400">
                      {#if record.id === 0}
                        <span class="px-2 py-0.5 bg-red-500 text-white text-xs rounded-full font-bold">Aktif</span>
                      {:else}
                        {record.id}
                      {/if}
                    </td>
                    <td class="px-4 py-2 font-mono text-slate-900 dark:text-white">{record.start}</td>
                    <td class="px-4 py-2 font-mono text-slate-900 dark:text-white">{record.end || '—'}</td>
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

            <!-- Terminal Cover -->
            <div class="p-4 bg-white/50 dark:bg-black/20 rounded-lg">
              <div class="flex items-center gap-2 mb-2">
                <Icon name="sensor_door" class="text-purple-500" size="sm" />
                <span class="font-bold text-slate-900 dark:text-white">{$t.terminalCover}</span>
              </div>
              <div class="text-3xl font-mono font-bold text-purple-600">{warningsData.terminalCover.count}</div>
              <div class="text-xs text-slate-500">toplam acilma</div>
            </div>
          </div>
        </div>
      {/if}
    </div>

    <!-- Reset Records Section -->
    <div class="bg-gradient-to-br from-teal-50 to-cyan-50 dark:from-teal-900/20 dark:to-cyan-900/20 rounded-xl border border-teal-200 dark:border-teal-800 overflow-hidden">
      <button
        onclick={() => toggleSection('reset')}
        class="w-full flex items-center justify-between p-4 hover:bg-teal-100/50 dark:hover:bg-teal-900/30 transition-colors"
      >
        <div class="flex items-center gap-3">
          <Icon name="restart_alt" class="text-teal-500" />
          <span class="font-bold text-slate-900 dark:text-white">{$t.resetRecords || "Reset Kayıtları"}</span>
          <span class="px-2 py-1 bg-teal-500/20 text-teal-600 dark:text-teal-400 rounded-full text-xs font-bold">
            {warningsData.reset.count}
          </span>
        </div>
        <Icon name={expandedSections.reset ? "expand_less" : "expand_more"} class="text-slate-400" />
      </button>

      {#if expandedSections.reset}
        <div class="p-4 pt-0">
          {#if warningsData.reset.records.length > 0}
            <table class="w-full text-sm">
              <thead class="bg-teal-100/50 dark:bg-teal-900/30">
                <tr class="border-b border-teal-200 dark:border-teal-800">
                  <th class="px-4 py-2 text-left font-bold text-slate-700 dark:text-slate-300">#</th>
                  <th class="px-4 py-2 text-left font-bold text-slate-700 dark:text-slate-300">{$t.startTime}</th>
                  <th class="px-4 py-2 text-left font-bold text-slate-700 dark:text-slate-300">{$t.endTime}</th>
                </tr>
              </thead>
              <tbody>
                {#each warningsData.reset.records as record}
                  <tr class="border-b border-teal-100 dark:border-teal-900/30">
                    <td class="px-4 py-2 font-mono text-slate-600 dark:text-slate-400">
                      {#if record.id === 0}
                        <span class="px-2 py-0.5 bg-teal-500 text-white text-xs rounded-full font-bold">Aktif</span>
                      {:else}
                        {record.id}
                      {/if}
                    </td>
                    <td class="px-4 py-2 font-mono text-slate-900 dark:text-white">{record.start}</td>
                    <td class="px-4 py-2 font-mono text-slate-900 dark:text-white">{record.end || '—'}</td>
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

    <!-- Neutral Voltage Warnings Section -->
    <div class="bg-gradient-to-br from-cyan-50 to-sky-50 dark:from-cyan-900/20 dark:to-sky-900/20 rounded-xl border border-cyan-200 dark:border-cyan-800 overflow-hidden">
      <button
        onclick={() => toggleSection('neutralVoltage')}
        class="w-full flex items-center justify-between p-4 hover:bg-cyan-100/50 dark:hover:bg-cyan-900/30 transition-colors"
      >
        <div class="flex items-center gap-3">
          <Icon name="electrical_services" class="text-cyan-500" />
          <span class="font-bold text-slate-900 dark:text-white">{"Nötr Gerilim Uyarıları"}</span>
          <span class="px-2 py-1 bg-cyan-500/20 text-cyan-600 dark:text-cyan-400 rounded-full text-xs font-bold">
            {warningsData.neutralVoltage.count}
          </span>
        </div>
        <Icon name={expandedSections.neutralVoltage ? "expand_less" : "expand_more"} class="text-slate-400" />
      </button>

      {#if expandedSections.neutralVoltage}
        <div class="p-4 pt-0">
          {#if warningsData.neutralVoltage.records.length > 0}
            <table class="w-full text-sm">
              <thead class="bg-cyan-100/50 dark:bg-cyan-900/30">
                <tr class="border-b border-cyan-200 dark:border-cyan-800">
                  <th class="px-4 py-2 text-left font-bold text-slate-700 dark:text-slate-300">#</th>
                  <th class="px-4 py-2 text-left font-bold text-slate-700 dark:text-slate-300">{$t.startTime}</th>
                  <th class="px-4 py-2 text-left font-bold text-slate-700 dark:text-slate-300">{$t.endTime}</th>
                </tr>
              </thead>
              <tbody>
                {#each warningsData.neutralVoltage.records as record}
                  <tr class="border-b border-cyan-100 dark:border-cyan-900/30">
                    <td class="px-4 py-2 font-mono text-slate-600 dark:text-slate-400">
                      {#if record.id === 0}
                        <span class="px-2 py-0.5 bg-cyan-500 text-white text-xs rounded-full font-bold">Aktif</span>
                      {:else}
                        {record.id}
                      {/if}
                    </td>
                    <td class="px-4 py-2 font-mono text-slate-900 dark:text-white">{record.start}</td>
                    <td class="px-4 py-2 font-mono text-slate-900 dark:text-white">{record.end || '—'}</td>
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
