<script lang="ts">
  import Icon from "$lib/components/common/Icon.svelte";
  import { t, isConnected, progressStore, addLog } from "$lib/stores";
  import { exportToExcel } from "$lib/utils/export";
  import { onMount } from "svelte";
  import Chart from "chart.js/auto";

  let selectedProfile = $state<1 | 2 | 3>(1);
  let isReading = $state(false);
  let readComplete = $state(false);
  let chartCanvas: HTMLCanvasElement;
  let chart: Chart | null = null;

  // Date range
  let startDate = $state("2024-12-01");
  let startTime = $state("00:00");
  let endDate = $state("2024-12-15");
  let endTime = $state("23:59");
  let readAllData = $state(false);

  // Pagination
  let currentPage = $state(1);
  const itemsPerPage = 20;

  // Mock load profile data
  let profileData = $state<Array<{
    timestamp: string;
    activeEnergy: number;
    activePower: number;
    voltageL1: number;
    voltageL2: number;
    voltageL3: number;
  }>>([]);

  const loadProfileSteps = [
    { id: "open-port", label: "openingSerialPort" },
    { id: "handshake", label: "sendingHandshake" },
    { id: "identify", label: "identifyingDevice" },
    { id: "baud-switch", label: "switchingBaudRate" },
    { id: "query-profile", label: "queryingLoadProfile" },
    { id: "receive-blocks", label: "receivingDataBlocks" },
    { id: "generate-chart", label: "generatingChart" },
    { id: "complete", label: "completed" },
  ];

  function selectProfile(profile: 1 | 2 | 3) {
    selectedProfile = profile;
    readComplete = false;
    profileData = [];
    if (chart) {
      chart.destroy();
      chart = null;
    }
  }

  async function startReading() {
    if (!$isConnected) return;

    isReading = true;
    readComplete = false;

    const steps = loadProfileSteps.map((s) => ({
      id: s.id,
      label: $t[s.label as keyof typeof $t] as string,
    }));

    progressStore.start($t.loadProfile, steps);

    // Simulate reading process
    for (let i = 0; i < steps.length - 1; i++) {
      progressStore.nextStep();
      addLog("info", steps[i].label);
      await new Promise((r) => setTimeout(r, 400 + Math.random() * 400));
    }

    // Generate mock data
    generateMockData();

    progressStore.nextStep();
    await new Promise((r) => setTimeout(r, 300));
    progressStore.complete();

    isReading = false;
    readComplete = true;

    // Render chart after data is loaded
    setTimeout(() => renderChart(), 100);
  }

  function generateMockData() {
    const data: typeof profileData = [];
    const start = new Date(readAllData ? "2024-12-01" : `${startDate}T${startTime}`);
    const end = new Date(readAllData ? "2024-12-15" : `${endDate}T${endTime}`);
    const interval = 15 * 60 * 1000; // 15 minutes

    let current = new Date(start);
    while (current <= end) {
      const hour = current.getHours();
      const isNight = hour >= 22 || hour < 6;
      const isPeak = hour >= 17 && hour < 22;

      // Simulate realistic consumption patterns
      const basePower = isNight ? 0.5 : isPeak ? 3.5 : 2.0;
      const variance = Math.random() * 1.5;
      const activePower = basePower + variance;

      data.push({
        timestamp: current.toISOString().slice(0, 16).replace("T", " "),
        activeEnergy: parseFloat((Math.random() * 10 + 50).toFixed(3)),
        activePower: parseFloat(activePower.toFixed(3)),
        voltageL1: parseFloat((220 + Math.random() * 5 - 2.5).toFixed(1)),
        voltageL2: parseFloat((220 + Math.random() * 5 - 2.5).toFixed(1)),
        voltageL3: parseFloat((220 + Math.random() * 5 - 2.5).toFixed(1)),
      });

      current = new Date(current.getTime() + interval);
    }

    profileData = data;
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

    chart = new Chart(ctx, {
      type: "line",
      data: {
        labels: sampledData.map((d) => d.timestamp.slice(5, 16)),
        datasets: [
          {
            label: `${$t.activeEnergyImport} (kWh)`,
            data: sampledData.map((d) => d.activePower),
            borderColor: "#279EA7",
            backgroundColor: "rgba(39, 158, 167, 0.1)",
            fill: true,
            tension: 0.3,
            pointRadius: 0,
            pointHoverRadius: 4,
          },
          {
            label: `${$t.voltageL1} (V)`,
            data: sampledData.map((d) => d.voltageL1),
            borderColor: "#f59e0b",
            backgroundColor: "transparent",
            tension: 0.3,
            pointRadius: 0,
            pointHoverRadius: 4,
            yAxisID: "y1",
          },
        ],
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
            grid: {
              color: gridColor,
            },
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
            grid: {
              color: gridColor,
            },
            ticks: {
              color: textColor,
            },
            title: {
              display: true,
              text: "kW",
              color: textColor,
            },
          },
          y1: {
            type: "linear",
            display: true,
            position: "right",
            grid: {
              drawOnChartArea: false,
            },
            ticks: {
              color: textColor,
            },
            title: {
              display: true,
              text: "V",
              color: textColor,
            },
            min: 210,
            max: 230,
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

  function handleExport() {
    exportToExcel(profileData, `load_profile_${selectedProfile}`, [
      { key: "timestamp", label: $t.dateTime },
      { key: "activeEnergy", label: `${$t.activeEnergyImport} (kWh)` },
      { key: "activePower", label: "Active Power (kW)" },
      { key: "voltageL1", label: `${$t.voltageL1} (V)` },
      { key: "voltageL2", label: `${$t.voltageL2} (V)` },
      { key: "voltageL3", label: `${$t.voltageL3} (V)` },
    ]);
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
  <!-- Header -->
  <div
    class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm"
  >
    <div class="flex items-start justify-between">
      <div>
        <h3 class="text-xl font-bold text-slate-900 dark:text-white mb-2">{$t.loadProfile}</h3>
        <p class="text-sm text-slate-500 dark:text-slate-400">
          {$t.loadProfileDescription}
        </p>
      </div>
      {#if readComplete}
        <button
          onclick={handleExport}
          class="flex items-center gap-2 px-4 py-2 bg-emerald-600 hover:bg-emerald-700 text-white text-sm font-bold rounded-lg transition-colors"
        >
          <Icon name="download" size="sm" />
          {$t.exportToExcel}
        </button>
      {/if}
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

  <!-- Profile Selection -->
  <div
    class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm"
  >
    <h4 class="font-bold text-slate-900 dark:text-white mb-4">{$t.selectLoadProfile}</h4>
    <div class="grid grid-cols-3 gap-4">
      {#each [1, 2, 3] as profile}
        <button
          onclick={() => selectProfile(profile as 1 | 2 | 3)}
          class="p-4 rounded-xl font-bold text-center transition-all
            {selectedProfile === profile
            ? 'bg-primary/10 border border-primary/20 text-primary'
            : 'bg-slate-50 dark:bg-[#0f1821] border border-slate-200 dark:border-[#334a5e] text-slate-500 hover:bg-slate-100 dark:hover:bg-[#1f3244]'}"
        >
          {$t[`loadProfile${profile}` as keyof typeof $t]}
        </button>
      {/each}
    </div>
  </div>

  <!-- Date Range & Read Options -->
  <div
    class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm"
  >
    <div class="flex flex-wrap items-end gap-6">
      <!-- Record Period Display -->
      <div class="flex items-center gap-2 px-4 py-2 bg-slate-50 dark:bg-[#0f1821] rounded-lg">
        <Icon name="timer" size="sm" class="text-primary" />
        <span class="text-sm text-slate-500">{$t.recordPeriod}:</span>
        <span class="text-sm font-bold text-slate-900 dark:text-white">15 {$t.minutes}</span>
      </div>

      <!-- Start Date/Time -->
      <div class="flex flex-col gap-1">
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
      <div class="flex flex-col gap-1">
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

      <!-- Read All Checkbox -->
      <label class="flex items-center gap-2 cursor-pointer">
        <input
          type="checkbox"
          bind:checked={readAllData}
          disabled={!$isConnected}
          class="w-4 h-4 rounded border-slate-300 dark:border-[#334a5e] text-primary focus:ring-primary disabled:opacity-50"
        />
        <span class="text-sm font-medium text-slate-600 dark:text-slate-400">{$t.readAll}</span>
      </label>

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
          {readAllData ? $t.readAll : $t.readSelected}
        {/if}
      </button>
    </div>
  </div>

  <!-- Progress Bar -->
  {#if $progressStore.active}
    <div
      class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm"
    >
      <div class="flex items-center justify-between mb-4">
        <h4 class="font-bold text-slate-900 dark:text-white">{$progressStore.title}</h4>
        <div class="flex items-center gap-4">
          <span class="text-sm font-mono text-primary">{$progressStore.percentage}%</span>
          <span class="text-sm text-slate-500 dark:text-slate-400">
            {$progressStore.elapsedTime.toFixed(1)}s
          </span>
        </div>
      </div>

      <div class="h-2 bg-slate-200 dark:bg-[#334a5e] rounded-lg overflow-hidden mb-6">
        <div
          class="h-full bg-gradient-to-r from-primary to-emerald-400 transition-all duration-300"
          style="width: {$progressStore.percentage}%"
        ></div>
      </div>

      <div class="grid grid-cols-2 md:grid-cols-4 gap-2">
        {#each $progressStore.steps as step}
          <div class="flex items-center gap-2">
            {#if step.status === "completed"}
              <Icon name="check_circle" class="text-emerald-500" size="sm" />
            {:else if step.status === "in_progress"}
              <Icon name="sync" class="text-primary animate-spin" size="sm" />
            {:else}
              <Icon name="circle" class="text-slate-300 dark:text-slate-600" size="sm" />
            {/if}
            <span
              class="text-xs truncate {step.status === 'completed'
                ? 'text-emerald-600 dark:text-emerald-500'
                : step.status === 'in_progress'
                  ? 'text-slate-900 dark:text-white font-medium'
                  : 'text-slate-400 dark:text-slate-500'}"
            >
              {step.label}
            </span>
          </div>
        {/each}
      </div>
    </div>
  {/if}

  <!-- Chart -->
  {#if readComplete && profileData.length > 0}
    <div
      class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm"
    >
      <h4 class="font-bold text-slate-900 dark:text-white mb-4 flex items-center gap-2">
        <Icon name="show_chart" class="text-primary" />
        {$t.loadProfile} {selectedProfile} - {profileData.length} {$t.recordPeriod.toLowerCase()}
      </h4>
      <div class="h-80">
        <canvas bind:this={chartCanvas}></canvas>
      </div>
    </div>

    <!-- Data Table -->
    <div
      class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl shadow-sm overflow-hidden"
    >
      <div class="p-4 border-b border-slate-200 dark:border-[#334a5e] flex items-center justify-between">
        <h4 class="font-bold text-slate-900 dark:text-white flex items-center gap-2">
          <Icon name="table_chart" class="text-primary" />
          {$t.historicalData}
        </h4>
        <div class="text-sm text-slate-500">
          {profileData.length} records
        </div>
      </div>

      <div class="overflow-x-auto">
        <table class="w-full">
          <thead class="bg-slate-50 dark:bg-[#0f1821]">
            <tr>
              <th class="px-4 py-3 text-left text-xs font-bold text-slate-500 uppercase">{$t.dateTime}</th>
              <th class="px-4 py-3 text-right text-xs font-bold text-slate-500 uppercase">Active Energy (kWh)</th>
              <th class="px-4 py-3 text-right text-xs font-bold text-slate-500 uppercase">Active Power (kW)</th>
              <th class="px-4 py-3 text-right text-xs font-bold text-slate-500 uppercase">{$t.voltageL1} (V)</th>
              <th class="px-4 py-3 text-right text-xs font-bold text-slate-500 uppercase">{$t.voltageL2} (V)</th>
              <th class="px-4 py-3 text-right text-xs font-bold text-slate-500 uppercase">{$t.voltageL3} (V)</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-slate-200 dark:divide-[#334a5e]">
            {#each paginatedData as row}
              <tr class="hover:bg-slate-50 dark:hover:bg-[#334a5e]/40 transition-colors">
                <td class="px-4 py-3 text-sm font-mono text-slate-600 dark:text-slate-400">{row.timestamp}</td>
                <td class="px-4 py-3 text-sm font-mono text-right text-slate-900 dark:text-white">{row.activeEnergy.toFixed(3)}</td>
                <td class="px-4 py-3 text-sm font-mono text-right text-slate-900 dark:text-white">{row.activePower.toFixed(3)}</td>
                <td class="px-4 py-3 text-sm font-mono text-right text-slate-900 dark:text-white">{row.voltageL1.toFixed(1)}</td>
                <td class="px-4 py-3 text-sm font-mono text-right text-slate-900 dark:text-white">{row.voltageL2.toFixed(1)}</td>
                <td class="px-4 py-3 text-sm font-mono text-right text-slate-900 dark:text-white">{row.voltageL3.toFixed(1)}</td>
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
  {:else if !$progressStore.active && !readComplete}
    <div
      class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-12 shadow-sm text-center"
    >
      <Icon name="bar_chart" class="text-6xl text-slate-300 dark:text-slate-600 mb-4" />
      <p class="text-slate-500 dark:text-slate-400">
        {$t.loadProfilePlaceholder}
      </p>
    </div>
  {/if}
</div>
