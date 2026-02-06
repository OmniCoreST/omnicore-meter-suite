<script lang="ts">
  import Icon from "$lib/components/common/Icon.svelte";
  import { t, isConnected, meterStore } from "$lib/stores";
  import { exportToExcel } from "$lib/utils/export";

  let activePhase = $state<"three" | "l1" | "l2" | "l3">("three");
  let showLong = $state(true);

  function parseDateTime(dateTimeStr: string): Date {
    // Format: yy-mm-dd,hh:mm
    const [datePart, timePart] = dateTimeStr.split(',');
    const [year, month, day] = datePart.split('-').map(Number);
    const [hour, minute] = timePart.split(':').map(Number);
    return new Date(2000 + year, month - 1, day, hour, minute);
  }

  function formatDuration(minutes: number): string {
    if (minutes < 60) return `${minutes} dk`;
    const hours = Math.floor(minutes / 60);
    const mins = minutes % 60;
    if (hours < 24) return `${hours}s ${mins}dk`;
    const days = Math.floor(hours / 24);
    const remainingHours = hours % 24;
    return `${days}g ${remainingHours}s`;
  }

  function getDurationClass(minutes: number): string {
    if (minutes < 5) return "text-amber-500";
    if (minutes < 60) return "text-orange-500";
    return "text-red-500";
  }

  // Parse outage records from raw OBIS codes
  let outagesData = $derived.by(() => {
    const data = $meterStore.shortReadData;
    // @ts-ignore
    if (!data || !data.rawData) {
      return {
        threePhase: { long: { count: 0, records: [] }, short: { count: 0, records: [] } },
        phase1: { long: { count: 0, records: [] }, short: { count: 0, records: [] } },
        phase2: { long: { count: 0, records: [] }, short: { count: 0, records: [] } },
        phase3: { long: { count: 0, records: [] }, short: { count: 0, records: [] } },
      };
    }

    // @ts-ignore
    const raw = data.rawData;

    const parseOutageRecords = (baseCode: string, maxCount: number) => {
      const records = [];
      for (let i = 1; i <= maxCount; i++) {
        const match = raw.match(
          new RegExp(`${baseCode.replace(/\./g, '\\.')}\\*${i}\\(([^;]+);([^)]+)\\)`)
        );
        if (match) {
          const start = match[1];
          const end = match[2];
          try {
            const startDate = parseDateTime(start);
            const endDate = parseDateTime(end);
            const durationMs = endDate.getTime() - startDate.getTime();
            const durationMin = Math.floor(durationMs / 60000);

            records.push({
              id: i,
              start,
              end,
              duration: durationMin,
            });
          } catch {
            records.push({ id: i, start, end, duration: 0 });
          }
        }
      }
      return records;
    };

    // Three-phase outages
    const threePhaseLongCount = raw.match(/96\.7\.0\((\d+)\)/);
    const threePhaseShortCount = raw.match(/96\.7\.00\((\d+)\)/);

    // Phase 1 outages
    const phase1LongCount = raw.match(/96\.7\.1\((\d+)\)/);
    const phase1ShortCount = raw.match(/96\.7\.10\((\d+)\)/);

    // Phase 2 outages
    const phase2LongCount = raw.match(/96\.7\.2\((\d+)\)/);
    const phase2ShortCount = raw.match(/96\.7\.20\((\d+)\)/);

    // Phase 3 outages
    const phase3LongCount = raw.match(/96\.7\.3\((\d+)\)/);
    const phase3ShortCount = raw.match(/96\.7\.30\((\d+)\)/);

    return {
      threePhase: {
        long: { count: threePhaseLongCount ? parseInt(threePhaseLongCount[1]) : 0, records: parseOutageRecords('96.77.0', 99) },
        short: { count: threePhaseShortCount ? parseInt(threePhaseShortCount[1]) : 0, records: parseOutageRecords('96.77.00', 99) },
      },
      phase1: {
        long: { count: phase1LongCount ? parseInt(phase1LongCount[1]) : 0, records: parseOutageRecords('96.77.1', 99) },
        short: { count: phase1ShortCount ? parseInt(phase1ShortCount[1]) : 0, records: parseOutageRecords('96.77.10', 99) },
      },
      phase2: {
        long: { count: phase2LongCount ? parseInt(phase2LongCount[1]) : 0, records: parseOutageRecords('96.77.2', 99) },
        short: { count: phase2ShortCount ? parseInt(phase2ShortCount[1]) : 0, records: parseOutageRecords('96.77.20', 99) },
      },
      phase3: {
        long: { count: phase3LongCount ? parseInt(phase3LongCount[1]) : 0, records: parseOutageRecords('96.77.3', 99) },
        short: { count: phase3ShortCount ? parseInt(phase3ShortCount[1]) : 0, records: parseOutageRecords('96.77.30', 99) },
      },
    };
  });

  let currentPhaseData = $derived.by(() => {
    switch (activePhase) {
      case "three": return outagesData.threePhase;
      case "l1": return outagesData.phase1;
      case "l2": return outagesData.phase2;
      case "l3": return outagesData.phase3;
      default: return outagesData.threePhase;
    }
  });


  function handleExport() {
    const allRecords: any[] = [];

    const addRecords = (phase: string, type: string, records: any[]) => {
      records.forEach(r => {
        allRecords.push({
          phase,
          type,
          id: r.id,
          start: r.start,
          end: r.end,
          duration: formatDuration(r.duration),
        });
      });
    };

    addRecords("3-Faz", "Uzun", outagesData.threePhase.long.records);
    addRecords("3-Faz", "Kisa", outagesData.threePhase.short.records);
    addRecords("L1", "Uzun", outagesData.phase1.long.records);
    addRecords("L1", "Kisa", outagesData.phase1.short.records);
    addRecords("L2", "Uzun", outagesData.phase2.long.records);
    addRecords("L2", "Kisa", outagesData.phase2.short.records);
    addRecords("L3", "Uzun", outagesData.phase3.long.records);
    addRecords("L3", "Kisa", outagesData.phase3.short.records);

    exportToExcel(allRecords, "outages", [
      { key: "phase", label: "Faz" },
      { key: "type", label: $t.type },
      { key: "id", label: "#" },
      { key: "start", label: $t.startTime },
      { key: "end", label: $t.endTime },
      { key: "duration", label: $t.duration },
    ]);
  }
</script>

<div class="space-y-6">
  <!-- Header -->
  <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm">
    <div class="flex items-start justify-between">
      <div>
        <h3 class="text-xl font-bold text-slate-900 dark:text-white mb-2">{$t.outages}</h3>
        <p class="text-sm text-slate-500 dark:text-slate-400">{$t.outagesDescription}</p>
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
    <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
      <button
        onclick={() => activePhase = "three"}
        class="p-4 rounded-xl text-left transition-all {activePhase === 'three' ? 'bg-slate-900 dark:bg-primary text-white' : 'bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e]'}"
      >
        <div class="flex items-center gap-2 mb-2">
          <Icon name="power_off" class="{activePhase === 'three' ? 'text-white' : 'text-slate-500'}" />
          <span class="font-bold {activePhase === 'three' ? 'text-white' : 'text-slate-900 dark:text-white'}">{$t.threePhaseOutage}</span>
        </div>
        <div class="text-3xl font-mono font-bold">
          {outagesData.threePhase.long.count + outagesData.threePhase.short.count}
        </div>
        <div class="text-xs {activePhase === 'three' ? 'text-white/70' : 'text-slate-500'}">
          {outagesData.threePhase.long.count} uzun / {outagesData.threePhase.short.count} kisa
        </div>
      </button>

      <button
        onclick={() => activePhase = "l1"}
        class="p-4 rounded-xl text-left transition-all {activePhase === 'l1' ? 'bg-red-500 text-white' : 'bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e]'}"
      >
        <div class="flex items-center gap-2 mb-2">
          <div class="w-3 h-3 rounded-full {activePhase === 'l1' ? 'bg-white' : 'bg-red-500'}"></div>
          <span class="font-bold {activePhase === 'l1' ? 'text-white' : 'text-slate-900 dark:text-white'}">{$t.l1Outage}</span>
        </div>
        <div class="text-3xl font-mono font-bold">
          {outagesData.phase1.long.count + outagesData.phase1.short.count}
        </div>
        <div class="text-xs {activePhase === 'l1' ? 'text-white/70' : 'text-slate-500'}">
          {outagesData.phase1.long.count} uzun / {outagesData.phase1.short.count} kisa
        </div>
      </button>

      <button
        onclick={() => activePhase = "l2"}
        class="p-4 rounded-xl text-left transition-all {activePhase === 'l2' ? 'bg-yellow-500 text-white' : 'bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e]'}"
      >
        <div class="flex items-center gap-2 mb-2">
          <div class="w-3 h-3 rounded-full {activePhase === 'l2' ? 'bg-white' : 'bg-yellow-500'}"></div>
          <span class="font-bold {activePhase === 'l2' ? 'text-white' : 'text-slate-900 dark:text-white'}">{$t.l2Outage}</span>
        </div>
        <div class="text-3xl font-mono font-bold">
          {outagesData.phase2.long.count + outagesData.phase2.short.count}
        </div>
        <div class="text-xs {activePhase === 'l2' ? 'text-white/70' : 'text-slate-500'}">
          {outagesData.phase2.long.count} uzun / {outagesData.phase2.short.count} kisa
        </div>
      </button>

      <button
        onclick={() => activePhase = "l3"}
        class="p-4 rounded-xl text-left transition-all {activePhase === 'l3' ? 'bg-blue-500 text-white' : 'bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e]'}"
      >
        <div class="flex items-center gap-2 mb-2">
          <div class="w-3 h-3 rounded-full {activePhase === 'l3' ? 'bg-white' : 'bg-blue-500'}"></div>
          <span class="font-bold {activePhase === 'l3' ? 'text-white' : 'text-slate-900 dark:text-white'}">{$t.l3Outage}</span>
        </div>
        <div class="text-3xl font-mono font-bold">
          {outagesData.phase3.long.count + outagesData.phase3.short.count}
        </div>
        <div class="text-xs {activePhase === 'l3' ? 'text-white/70' : 'text-slate-500'}">
          {outagesData.phase3.long.count} uzun / {outagesData.phase3.short.count} kisa
        </div>
      </button>
    </div>

    <!-- Outage Type Toggle -->
    <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-2 shadow-sm">
      <div class="flex gap-2">
        <button
          onclick={() => showLong = true}
          class="flex-1 flex items-center justify-center gap-2 px-4 py-3 rounded-lg font-bold transition-colors
            {showLong ? 'bg-red-500 text-white' : 'text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-[#334a5e]'}"
        >
          <Icon name="timer" size="sm" />
          {$t.longOutages} ({currentPhaseData.long.count})
        </button>
        <button
          onclick={() => showLong = false}
          class="flex-1 flex items-center justify-center gap-2 px-4 py-3 rounded-lg font-bold transition-colors
            {!showLong ? 'bg-amber-500 text-white' : 'text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-[#334a5e]'}"
        >
          <Icon name="timer_off" size="sm" />
          {$t.shortOutages} ({currentPhaseData.short.count})
        </button>
      </div>
    </div>

    <!-- Outage Records Table -->
    <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl shadow-sm overflow-hidden">
      <div class="p-4 border-b border-slate-200 dark:border-[#334a5e]">
        <h4 class="font-bold text-slate-900 dark:text-white flex items-center gap-2">
          <Icon name={showLong ? "timer" : "timer_off"} class={showLong ? "text-red-500" : "text-amber-500"} />
          {showLong ? $t.longOutages : $t.shortOutages}
          <span class="text-sm font-normal text-slate-500">
            ({showLong ? ">= 180sn" : "< 180sn"})
          </span>
        </h4>
      </div>

      {#if showLong ? currentPhaseData.long.records.length > 0 : currentPhaseData.short.records.length > 0}
        {@const records = showLong ? currentPhaseData.long.records : currentPhaseData.short.records}
        <div class="overflow-x-auto">
          <table class="w-full text-sm">
            <thead class="bg-slate-50 dark:bg-[#0f1821]">
              <tr class="border-b border-slate-200 dark:border-[#334a5e]">
                <th class="px-4 py-3 text-left font-bold text-slate-700 dark:text-slate-300">#</th>
                <th class="px-4 py-3 text-left font-bold text-slate-700 dark:text-slate-300">{$t.startTime}</th>
                <th class="px-4 py-3 text-left font-bold text-slate-700 dark:text-slate-300">{$t.endTime}</th>
                <th class="px-4 py-3 text-right font-bold text-slate-700 dark:text-slate-300">{$t.duration}</th>
              </tr>
            </thead>
            <tbody>
              {#each records as record}
                <tr class="border-b border-slate-100 dark:border-[#334a5e]/30 hover:bg-slate-50 dark:hover:bg-[#1a2632] transition-colors">
                  <td class="px-4 py-3 font-mono text-slate-600 dark:text-slate-400">{record.id}</td>
                  <td class="px-4 py-3 font-mono text-slate-900 dark:text-white">{record.start}</td>
                  <td class="px-4 py-3 font-mono text-slate-900 dark:text-white">{record.end}</td>
                  <td class="px-4 py-3 text-right font-mono font-bold {getDurationClass(record.duration)}">
                    {formatDuration(record.duration)}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {:else}
        <div class="text-center py-12 text-slate-400">
          <Icon name="check_circle" class="text-4xl text-emerald-500 mb-2" />
          <p class="text-sm">{$t.noOutages}</p>
        </div>
      {/if}
    </div>

    <!-- Duration Legend -->
    <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-4 shadow-sm">
      <div class="flex flex-wrap items-center justify-center gap-6 text-sm">
        <div class="flex items-center gap-2">
          <div class="w-3 h-3 rounded-full bg-amber-500"></div>
          <span class="text-slate-600 dark:text-slate-400">&lt; 5 dakika</span>
        </div>
        <div class="flex items-center gap-2">
          <div class="w-3 h-3 rounded-full bg-orange-500"></div>
          <span class="text-slate-600 dark:text-slate-400">5 - 60 dakika</span>
        </div>
        <div class="flex items-center gap-2">
          <div class="w-3 h-3 rounded-full bg-red-500"></div>
          <span class="text-slate-600 dark:text-slate-400">&gt; 60 dakika</span>
        </div>
      </div>
    </div>
  {/if}
</div>
