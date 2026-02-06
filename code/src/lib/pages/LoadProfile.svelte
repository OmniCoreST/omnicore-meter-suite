<script lang="ts">
  import Icon from "$lib/components/common/Icon.svelte";
  import { t, isConnected, progressStore, addLog, meterStore } from "$lib/stores";
  import { exportToExcel } from "$lib/utils/export";
  import { readLoadProfile, onReadProgress, onCommLog } from "$lib/utils/tauri";
  import { onMount } from "svelte";
  import Chart from "chart.js/auto";

  // Profile definitions parsed from meter data (97.1.0, 97.2.0, 97.3.0)
  interface ProfileColumn {
    obis: string;
    label: string;
    unit: string;
  }

  interface ProfileDefinition {
    id: number;
    name: string;
    columns: ProfileColumn[];
    available: boolean;
  }

  // OBIS code to label/unit mapping
  const obisLabels: Record<string, { label: string; unit: string }> = {
    "1.8.0": { label: "activeEnergyImport", unit: "kWh" },
    "2.8.0": { label: "activeEnergyExport", unit: "kWh" },
    "32.7.0": { label: "voltageL1", unit: "V" },
    "52.7.0": { label: "voltageL2", unit: "V" },
    "72.7.0": { label: "voltageL3", unit: "V" },
    "31.7.0": { label: "currentL1", unit: "A" },
    "51.7.0": { label: "currentL2", unit: "A" },
    "71.7.0": { label: "currentL3", unit: "A" },
    "33.7.0": { label: "powerFactorL1", unit: "" },
    "53.7.0": { label: "powerFactorL2", unit: "" },
    "73.7.0": { label: "powerFactorL3", unit: "" },
    "14.7.0": { label: "frequency", unit: "Hz" },
  };

  // Parse profile definition from OBIS response like "97.1.0(1.8.0*kWh)"
  function parseProfileDefinition(profileId: number, rawValue: string): ProfileDefinition {
    const columns: ProfileColumn[] = [];

    // Extract content between parentheses
    const match = rawValue.match(/\(([^)]+)\)/);
    if (match) {
      const content = match[1];
      // Split by comma and parse each column
      const parts = content.split(",");
      for (const part of parts) {
        const [obisWithUnit] = part.trim().split("*");
        const obis = obisWithUnit.trim();
        const info = obisLabels[obis] || { label: obis, unit: "" };
        columns.push({
          obis,
          label: info.label,
          unit: info.unit,
        });
      }
    }

    const names = ["", "loadProfile1", "loadProfile2", "loadProfile3"];
    return {
      id: profileId,
      name: names[profileId] || `Load Profile ${profileId}`,
      columns,
      available: columns.length > 0,
    };
  }

  // Get profile definitions from meter data or use defaults
  let profileDefinitions = $derived.by(() => {
    const rawData = $meterStore.shortReadData?.rawData || "";
    const definitions: ProfileDefinition[] = [];

    // Try to parse from raw data
    const profile1Match = rawData.match(/97\.1\.0\([^)]+\)/);
    const profile2Match = rawData.match(/97\.2\.0\([^)]+\)/);
    const profile3Match = rawData.match(/97\.3\.0\([^)]+\)/);

    if (profile1Match) {
      definitions.push(parseProfileDefinition(1, profile1Match[0]));
    } else {
      // Default Profile 1: Energy
      definitions.push({
        id: 1,
        name: "loadProfile1",
        columns: [{ obis: "1.8.0", label: "activeEnergyImport", unit: "kWh" }],
        available: true,
      });
    }

    if (profile2Match) {
      definitions.push(parseProfileDefinition(2, profile2Match[0]));
    } else {
      // Default Profile 2: Instantaneous values
      definitions.push({
        id: 2,
        name: "loadProfile2",
        columns: [
          { obis: "32.7.0", label: "voltageL1", unit: "V" },
          { obis: "52.7.0", label: "voltageL2", unit: "V" },
          { obis: "72.7.0", label: "voltageL3", unit: "V" },
          { obis: "31.7.0", label: "currentL1", unit: "A" },
          { obis: "51.7.0", label: "currentL2", unit: "A" },
          { obis: "71.7.0", label: "currentL3", unit: "A" },
          { obis: "33.7.0", label: "powerFactorL1", unit: "" },
          { obis: "53.7.0", label: "powerFactorL2", unit: "" },
          { obis: "73.7.0", label: "powerFactorL3", unit: "" },
          { obis: "14.7.0", label: "frequency", unit: "Hz" },
        ],
        available: true,
      });
    }

    if (profile3Match) {
      definitions.push(parseProfileDefinition(3, profile3Match[0]));
    }

    return definitions;
  });

  let selectedProfileId = $state(1);
  let selectedProfile = $derived(
    profileDefinitions.find(p => p.id === selectedProfileId) || profileDefinitions[0]
  );

  let isReading = $state(false);
  let readComplete = $state(false);
  let receivedBytes = $state(0);
  let receivedLines = $state(0);
  let chartCanvas: HTMLCanvasElement;
  let chart: Chart | null = null;

  // Date range — default to today
  const today = new Date().toISOString().slice(0, 10);
  let startDate = $state(today);
  let startTime = $state("00:00");
  let endDate = $state(today);
  let endTime = $state("23:59");
  let readAllData = $state(false);

  // Pagination
  let currentPage = $state(1);
  const itemsPerPage = 20;

  // Profile data - dynamic based on profile structure
  let profileData = $state<Array<Record<string, number | string>>>([]);

  const loadProfileSteps = [
    { id: "check-connection", label: "checkingConnection" },
    { id: "reconnect", label: "reconnectingForProgramming" },
    { id: "prog-mode", label: "enteringProgrammingMode" },
    { id: "query-profile", label: "queryingLoadProfile" },
    { id: "receive-blocks", label: "receivingDataBlocks" },
    { id: "parse-data", label: "parsingLoadProfile" },
    { id: "complete", label: "completed" },
  ];

  function selectProfile(profileId: number) {
    selectedProfileId = profileId;
    readComplete = false;
    profileData = [];
    currentPage = 1;
    if (chart) {
      chart.destroy();
      chart = null;
    }
  }

  async function startReading() {
    if (!$isConnected) return;

    isReading = true;
    readComplete = false;
    profileData = [];
    receivedBytes = 0;
    receivedLines = 0;

    const steps = loadProfileSteps.map((s) => ({
      id: s.id,
      label: $t[s.label as keyof typeof $t] as string,
    }));

    progressStore.start($t.loadProfile, steps);

    // Setup event listeners
    const unlistenProgress = await onReadProgress((event) => {
      const stepIndex = event.step - 1;
      if (stepIndex >= 0 && stepIndex < steps.length) {
        for (let i = 0; i <= stepIndex; i++) {
          progressStore.nextStep();
        }
      }
    });

    // Track received data from backend log messages (logging handled globally by CommLog)
    const unlistenLog = await onCommLog((event) => {
      const byteMatch = event.message.match(/(\d+)\s*byte/);
      if (byteMatch) {
        const bytes = parseInt(byteMatch[1]);
        if (bytes > receivedBytes) receivedBytes = bytes;
      }
      const lineMatch = event.message.match(/(\d+)\s*satır/);
      if (lineMatch) {
        const lines = parseInt(lineMatch[1]);
        if (lines > receivedLines) receivedLines = lines;
      }
    });

    try {
      // Format dates for IEC protocol: yy-mm-dd,hh:mm
      let startTimeParam: string | null = null;
      let endTimeParam: string | null = null;

      if (!readAllData) {
        // Convert from YYYY-MM-DD to yy-mm-dd
        startTimeParam = `${startDate.slice(2)},${startTime}`;
        endTimeParam = `${endDate.slice(2)},${endTime}`;
      }

      const result = await readLoadProfile(selectedProfileId, startTimeParam, endTimeParam);

      // Convert entries to our internal format
      const columns = selectedProfile.columns;
      const data: Array<Record<string, number | string>> = [];

      for (const entry of result.entries) {
        const row: Record<string, number | string> = {
          timestamp: entry.timestamp,
        };

        // Map values to columns
        entry.values.forEach((value, idx) => {
          if (idx < columns.length) {
            row[columns[idx].obis] = value;
          }
        });

        data.push(row);
      }

      profileData = data;
      addLog("success", `${$t.loadProfile} ${$t.completed}: ${data.length} ${$t.records || "records"}`);

    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      addLog("error", `${$t.loadProfile}: ${errorMessage}`);
    } finally {
      unlistenProgress();
      unlistenLog();
      progressStore.reset();
      isReading = false;
      readComplete = profileData.length > 0;

      // Render chart after data is loaded
      if (readComplete) {
        setTimeout(() => renderChart(), 100);
      }
    }
  }

  function renderChart() {
    if (!chartCanvas || profileData.length === 0) return;

    if (chart) {
      chart.destroy();
    }

    const ctx = chartCanvas.getContext("2d");
    if (!ctx) return;

    const isDark = document.documentElement.classList.contains("dark");
    const gridColor = isDark ? "rgba(255, 255, 255, 0.1)" : "rgba(0, 0, 0, 0.1)";
    const textColor = isDark ? "#94a3b8" : "#64748b";

    // Take every nth point for better visualization
    const step = Math.max(1, Math.floor(profileData.length / 100));
    const sampledData = profileData.filter((_, i) => i % step === 0);

    // Generate datasets based on profile columns
    const colors = ["#279EA7", "#f59e0b", "#8b5cf6", "#ef4444", "#10b981", "#ec4899", "#06b6d4", "#84cc16", "#f97316", "#6366f1"];
    const datasets = selectedProfile.columns.slice(0, 4).map((col, idx) => {
      const label = ($t[col.label as keyof typeof $t] || col.label) + (col.unit ? ` (${col.unit})` : "");
      return {
        label,
        data: sampledData.map((d) => d[col.obis] as number),
        borderColor: colors[idx % colors.length],
        backgroundColor: idx === 0 ? `${colors[0]}20` : "transparent",
        fill: idx === 0,
        tension: 0.3,
        pointRadius: 0,
        pointHoverRadius: 4,
        yAxisID: idx === 0 ? "y" : undefined,
      };
    });

    chart = new Chart(ctx, {
      type: "line",
      data: {
        labels: sampledData.map((d) => (d.timestamp as string).slice(5, 16)),
        datasets,
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        interaction: {
          mode: "index",
          intersect: false,
        },
        plugins: {
          legend: {
            position: "top",
            labels: {
              color: textColor,
              usePointStyle: true,
              padding: 20,
            },
          },
          tooltip: {
            backgroundColor: isDark ? "#1F3244" : "#ffffff",
            titleColor: isDark ? "#ffffff" : "#0f172a",
            bodyColor: isDark ? "#94a3b8" : "#64748b",
            borderColor: isDark ? "#334a5e" : "#e2e8f0",
            borderWidth: 1,
          },
        },
        scales: {
          x: {
            grid: { color: gridColor },
            ticks: {
              color: textColor,
              maxRotation: 45,
              minRotation: 45,
              maxTicksLimit: 12,
            },
          },
          y: {
            type: "linear",
            display: true,
            position: "left",
            grid: { color: gridColor },
            ticks: { color: textColor },
          },
        },
      },
    });
  }

  // Pagination computed values
  let totalPages = $derived(Math.ceil(profileData.length / itemsPerPage));
  let paginatedData = $derived(
    profileData.slice((currentPage - 1) * itemsPerPage, currentPage * itemsPerPage)
  );

  function getColumnLabel(col: ProfileColumn): string {
    const label = $t[col.label as keyof typeof $t] || col.label;
    return col.unit ? `${label} (${col.unit})` : String(label);
  }

  function handleExport() {
    const columns = [
      { key: "timestamp", label: $t.dateTime },
      ...selectedProfile.columns.map(col => ({
        key: col.obis,
        label: getColumnLabel(col),
      })),
    ];

    exportToExcel(profileData, `load_profile_${selectedProfileId}`, columns);
  }

  // Re-render chart when theme changes
  onMount(() => {
    const observer = new MutationObserver(() => {
      if (readComplete && profileData.length > 0) {
        renderChart();
      }
    });
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ["class"] });
    return () => observer.disconnect();
  });
</script>

<div class="space-y-6">
  {#if !$isConnected}
    <div class="p-4 bg-amber-500/10 border border-amber-500/20 rounded-xl text-amber-600 dark:text-amber-500 text-sm">
      <div class="flex items-center gap-2">
        <Icon name="warning" />
        <span>{$t.connectFirstWarning}</span>
      </div>
    </div>
  {/if}

  <!-- Profile Selection with Structure Display -->
  <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm">
    <h4 class="font-bold text-slate-900 dark:text-white mb-4">{$t.selectLoadProfile}</h4>
    <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
      {#each profileDefinitions.filter(p => p.available) as profile}
        <button
          onclick={() => selectProfile(profile.id)}
          class="p-4 rounded-xl text-left transition-all
            {selectedProfileId === profile.id
            ? 'bg-primary/10 border-2 border-primary/30 ring-2 ring-primary/10'
            : 'bg-slate-50 dark:bg-[#0f1821] border-2 border-slate-200 dark:border-[#334a5e] hover:border-primary/30'}"
        >
          <div class="flex items-center gap-3 mb-3">
            <div class="p-2 rounded-lg {selectedProfileId === profile.id ? 'bg-primary/20' : 'bg-slate-200 dark:bg-[#334a5e]'}">
              <Icon
                name={profile.id === 1 ? "bolt" : "speed"}
                class={selectedProfileId === profile.id ? "text-primary" : "text-slate-500"}
              />
            </div>
            <div>
              <span class="font-bold text-slate-900 dark:text-white">
                {$t[profile.name as keyof typeof $t] || profile.name}
              </span>
              <span class="text-xs text-slate-500 ml-2">({profile.columns.length} {$t.columns || "columns"})</span>
            </div>
          </div>

          <!-- Profile Structure -->
          <div class="flex flex-wrap gap-1.5">
            {#each profile.columns as col}
              <span class="inline-flex items-center gap-1 px-2 py-1 bg-white dark:bg-[#1a2632] rounded text-xs border border-slate-200 dark:border-[#334a5e]">
                <span class="font-mono text-primary">{col.obis}</span>
                <span class="text-slate-500">
                  {$t[col.label as keyof typeof $t] || col.label}
                  {#if col.unit}
                    <span class="text-slate-400">({col.unit})</span>
                  {/if}
                </span>
              </span>
            {/each}
          </div>
        </button>
      {/each}
    </div>
  </div>

  <!-- Date Range & Read Options -->
  <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm">
    <div class="flex flex-wrap items-end gap-6">
      <!-- Read All Toggle -->
      <div class="flex flex-col gap-1">
        <label class="text-xs font-bold text-slate-500 uppercase tracking-wider">{$t.dataRange}</label>
        <div class="flex rounded-lg overflow-hidden border border-slate-300 dark:border-[#334a5e]">
          <button
            onclick={() => readAllData = true}
            disabled={!$isConnected}
            class="px-4 py-2 text-sm font-medium transition-colors disabled:opacity-50
              {readAllData
              ? 'bg-primary text-white'
              : 'bg-white dark:bg-[#1a2632] text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-[#334a5e]'}"
          >
            {$t.allData}
          </button>
          <button
            onclick={() => readAllData = false}
            disabled={!$isConnected}
            class="px-4 py-2 text-sm font-medium transition-colors disabled:opacity-50 border-l border-slate-300 dark:border-[#334a5e]
              {!readAllData
              ? 'bg-primary text-white'
              : 'bg-white dark:bg-[#1a2632] text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-[#334a5e]'}"
          >
            {$t.dateRange}
          </button>
        </div>
      </div>

      <!-- Start Date/Time -->
      <div class="flex flex-col gap-1 {readAllData ? 'opacity-50' : ''}">
        <label class="text-xs font-bold text-slate-500 uppercase tracking-wider">{$t.startDateTime}</label>
        <div class="flex gap-2">
          <input
            type="date"
            bind:value={startDate}
            disabled={readAllData || !$isConnected}
            class="bg-white dark:bg-[#1a2632] text-slate-700 dark:text-white border border-slate-300 dark:border-[#334a5e] rounded-lg px-3 py-2 text-sm focus:border-primary focus:ring-1 focus:ring-primary outline-none disabled:opacity-50 disabled:cursor-not-allowed"
          />
          <input
            type="time"
            bind:value={startTime}
            disabled={readAllData || !$isConnected}
            class="bg-white dark:bg-[#1a2632] text-slate-700 dark:text-white border border-slate-300 dark:border-[#334a5e] rounded-lg px-3 py-2 text-sm focus:border-primary focus:ring-1 focus:ring-primary outline-none disabled:opacity-50 disabled:cursor-not-allowed"
          />
        </div>
      </div>

      <!-- End Date/Time -->
      <div class="flex flex-col gap-1 {readAllData ? 'opacity-50' : ''}">
        <label class="text-xs font-bold text-slate-500 uppercase tracking-wider">{$t.endDateTime}</label>
        <div class="flex gap-2">
          <input
            type="date"
            bind:value={endDate}
            disabled={readAllData || !$isConnected}
            class="bg-white dark:bg-[#1a2632] text-slate-700 dark:text-white border border-slate-300 dark:border-[#334a5e] rounded-lg px-3 py-2 text-sm focus:border-primary focus:ring-1 focus:ring-primary outline-none disabled:opacity-50 disabled:cursor-not-allowed"
          />
          <input
            type="time"
            bind:value={endTime}
            disabled={readAllData || !$isConnected}
            class="bg-white dark:bg-[#1a2632] text-slate-700 dark:text-white border border-slate-300 dark:border-[#334a5e] rounded-lg px-3 py-2 text-sm focus:border-primary focus:ring-1 focus:ring-primary outline-none disabled:opacity-50 disabled:cursor-not-allowed"
          />
        </div>
      </div>

      <!-- Read Button -->
      <button
        onclick={startReading}
        disabled={!$isConnected || isReading}
        class="flex items-center gap-2 px-6 py-2.5 bg-primary hover:bg-primary/90 text-white font-bold rounded-lg shadow-lg shadow-primary/20 transition-all disabled:opacity-50 disabled:cursor-not-allowed ml-auto"
      >
        {#if isReading}
          <Icon name="sync" class="animate-spin" />
          {$t.reading}
        {:else}
          <Icon name="play_arrow" />
          {$t.readLoadProfile}
        {/if}
      </button>
    </div>
  </div>

  <!-- Reading Indicator -->
  {#if isReading}
    <div class="bg-white dark:bg-surface-dark border border-primary/20 rounded-xl p-6 shadow-sm">
      <div class="flex items-center gap-4">
        <div class="relative flex items-center justify-center w-12 h-12 shrink-0">
          <span class="animate-ping absolute h-10 w-10 rounded-full bg-primary/20"></span>
          <span class="relative flex items-center justify-center h-10 w-10 rounded-full bg-primary/10">
            <Icon name="sync" class="text-primary animate-spin" />
          </span>
        </div>
        <div class="flex-grow min-w-0">
          <div class="font-bold text-slate-900 dark:text-white">{$t.reading}...</div>
          <div class="text-sm text-slate-500 dark:text-slate-400">
            {$t[selectedProfile.name as keyof typeof $t] || selectedProfile.name}
          </div>
        </div>
        <div class="flex items-center gap-6 shrink-0">
          {#if receivedBytes > 0}
            <div class="text-right">
              <div class="text-lg font-mono font-bold text-primary">
                {receivedBytes >= 1024 ? `${(receivedBytes / 1024).toFixed(1)} KB` : `${receivedBytes} B`}
              </div>
              <div class="text-[10px] uppercase tracking-wider text-slate-400">{$t.dataReceived || "Veri"}</div>
            </div>
          {/if}
          {#if receivedLines > 0}
            <div class="text-right">
              <div class="text-lg font-mono font-bold text-slate-700 dark:text-slate-300">
                {receivedLines}
              </div>
              <div class="text-[10px] uppercase tracking-wider text-slate-400">{$t.lines || "Satır"}</div>
            </div>
          {/if}
          <div class="text-right">
            <div class="text-lg font-mono text-slate-500">
              {$progressStore.elapsedTime.toFixed(0)}s
            </div>
            <div class="text-[10px] uppercase tracking-wider text-slate-400">{$t.elapsed || "Süre"}</div>
          </div>
        </div>
      </div>
    </div>
  {/if}

  <!-- Chart -->
  {#if readComplete && profileData.length > 0}
    <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm">
      <h4 class="font-bold text-slate-900 dark:text-white mb-4 flex items-center gap-2">
        <Icon name="show_chart" class="text-primary" />
        {$t[selectedProfile.name as keyof typeof $t] || selectedProfile.name} - {profileData.length} {$t.recordPeriod.toLowerCase()}
      </h4>
      <div class="h-80">
        <canvas bind:this={chartCanvas}></canvas>
      </div>
    </div>

    <!-- Data Table -->
    <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl shadow-sm overflow-hidden">
      <div class="p-4 border-b border-slate-200 dark:border-[#334a5e] flex items-center justify-between">
        <h4 class="font-bold text-slate-900 dark:text-white flex items-center gap-2">
          <Icon name="table_chart" class="text-primary" />
          {$t.historicalData}
          <span class="text-sm font-normal text-slate-500 ml-1">({profileData.length} {$t.records || "records"})</span>
        </h4>
        <button
          onclick={handleExport}
          class="flex items-center gap-2 px-4 py-2 bg-emerald-600 hover:bg-emerald-700 text-white text-sm font-bold rounded-lg transition-colors"
        >
          <Icon name="download" size="sm" />
          {$t.exportToExcel}
        </button>
      </div>

      <div class="overflow-x-auto">
        <table class="w-full">
          <thead class="bg-slate-50 dark:bg-[#0f1821]">
            <tr>
              <th class="px-4 py-3 text-left text-xs font-bold text-slate-500 uppercase whitespace-nowrap">{$t.dateTime}</th>
              {#each selectedProfile.columns as col}
                <th class="px-4 py-3 text-right text-xs font-bold text-slate-500 uppercase whitespace-nowrap">
                  {getColumnLabel(col)}
                </th>
              {/each}
            </tr>
          </thead>
          <tbody class="divide-y divide-slate-200 dark:divide-[#334a5e]">
            {#each paginatedData as row}
              <tr class="hover:bg-slate-50 dark:hover:bg-[#334a5e]/40 transition-colors">
                <td class="px-4 py-3 text-sm font-mono text-slate-600 dark:text-slate-400 whitespace-nowrap">{row.timestamp}</td>
                {#each selectedProfile.columns as col}
                  <td class="px-4 py-3 text-sm font-mono text-right text-slate-900 dark:text-white">
                    {typeof row[col.obis] === "number" ? (row[col.obis] as number).toFixed(col.unit === "Hz" || col.unit === "" ? 2 : col.unit === "A" ? 2 : col.unit === "V" ? 1 : 3) : row[col.obis]}
                  </td>
                {/each}
              </tr>
            {/each}
          </tbody>
        </table>
      </div>

      <!-- Pagination -->
      {#if totalPages > 1}
        <div class="p-4 border-t border-slate-200 dark:border-[#334a5e] flex items-center justify-between">
          <div class="text-sm text-slate-500">
            Page {currentPage} of {totalPages}
          </div>
          <div class="flex gap-2">
            <button
              onclick={() => currentPage = Math.max(1, currentPage - 1)}
              disabled={currentPage === 1}
              class="px-3 py-1 bg-slate-100 dark:bg-[#334a5e] text-slate-600 dark:text-slate-300 rounded-lg text-sm font-medium hover:bg-slate-200 dark:hover:bg-[#455a6e] disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
            >
              <Icon name="chevron_left" size="sm" />
            </button>
            {#each Array.from({ length: Math.min(5, totalPages) }, (_, i) => {
              const start = Math.max(1, Math.min(currentPage - 2, totalPages - 4));
              return start + i;
            }).filter(p => p <= totalPages) as page}
              <button
                onclick={() => currentPage = page}
                class="px-3 py-1 rounded-lg text-sm font-medium transition-colors
                  {page === currentPage
                  ? 'bg-primary text-white'
                  : 'bg-slate-100 dark:bg-[#334a5e] text-slate-600 dark:text-slate-300 hover:bg-slate-200 dark:hover:bg-[#455a6e]'}"
              >
                {page}
              </button>
            {/each}
            <button
              onclick={() => currentPage = Math.min(totalPages, currentPage + 1)}
              disabled={currentPage === totalPages}
              class="px-3 py-1 bg-slate-100 dark:bg-[#334a5e] text-slate-600 dark:text-slate-300 rounded-lg text-sm font-medium hover:bg-slate-200 dark:hover:bg-[#455a6e] disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
            >
              <Icon name="chevron_right" size="sm" />
            </button>
          </div>
        </div>
      {/if}
    </div>
  {:else if !isReading && !readComplete}
    <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-12 shadow-sm text-center">
      <Icon name="bar_chart" class="text-6xl text-slate-300 dark:text-slate-600 mb-4" />
      <p class="text-slate-500 dark:text-slate-400">{$t.loadProfilePlaceholder}</p>
    </div>
  {/if}
</div>
