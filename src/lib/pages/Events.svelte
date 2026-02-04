<script lang="ts">
  import Icon from "$lib/components/common/Icon.svelte";
  import { t } from "$lib/stores";
  import { exportToExcel } from "$lib/utils/export";

  // Mock event data
  const events = [
    {
      id: 1,
      type: "voltageWarnings",
      icon: "electric_bolt",
      start: "2024-06-30 13:30",
      end: "2024-06-30 13:35",
      duration: "5 min",
      detail: "Phase sequence error",
    },
    {
      id: 2,
      type: "currentWarnings",
      icon: "warning",
      start: "2024-06-28 09:15",
      end: "2024-06-28 09:18",
      duration: "3 min",
      detail: "Overcurrent L1",
    },
    {
      id: 3,
      type: "topCoverOpen",
      icon: "lock_open",
      start: "2024-06-25 14:00",
      end: "2024-06-25 14:02",
      duration: "2 min",
      detail: "Cover opened",
    },
  ];

  function handleExport() {
    const exportData = events.map(e => ({
      id: e.id,
      type: $t[e.type as keyof typeof $t],
      start: e.start,
      end: e.end,
      duration: e.duration,
      detail: e.detail,
    }));

    exportToExcel(exportData, "events", [
      { key: "id", label: "#" },
      { key: "type", label: $t.type },
      { key: "start", label: $t.startTime },
      { key: "end", label: $t.endTime },
      { key: "duration", label: $t.duration },
      { key: "detail", label: $t.detail },
    ]);
  }
</script>

<div class="space-y-6">
  <div
    class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm"
  >
    <div class="flex items-start justify-between">
      <div>
        <h3 class="text-xl font-bold text-slate-900 dark:text-white mb-2">{$t.events}</h3>
        <p class="text-sm text-slate-500 dark:text-slate-400">
          {$t.eventsDescription}
        </p>
      </div>
    </div>
  </div>

  <!-- Event Filters -->
  <div
    class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm"
  >
    <div class="flex flex-wrap items-center justify-between gap-4">
      <div class="flex flex-wrap gap-2">
        <button class="px-4 py-2 bg-primary text-white rounded-lg text-sm font-bold">All</button>
        <button
          class="px-4 py-2 bg-slate-100 dark:bg-[#334a5e] text-slate-600 dark:text-slate-300 rounded-lg text-sm font-medium hover:bg-slate-200 dark:hover:bg-[#455a6e] transition-colors"
        >
          {$t.voltageWarnings}
        </button>
        <button
          class="px-4 py-2 bg-slate-100 dark:bg-[#334a5e] text-slate-600 dark:text-slate-300 rounded-lg text-sm font-medium hover:bg-slate-200 dark:hover:bg-[#455a6e] transition-colors"
        >
          {$t.currentWarnings}
        </button>
        <button
          class="px-4 py-2 bg-slate-100 dark:bg-[#334a5e] text-slate-600 dark:text-slate-300 rounded-lg text-sm font-medium hover:bg-slate-200 dark:hover:bg-[#455a6e] transition-colors"
        >
          {$t.magneticField}
        </button>
        <button
          class="px-4 py-2 bg-slate-100 dark:bg-[#334a5e] text-slate-600 dark:text-slate-300 rounded-lg text-sm font-medium hover:bg-slate-200 dark:hover:bg-[#455a6e] transition-colors"
        >
          {$t.topCoverOpen}
        </button>
      </div>
      <button
        onclick={handleExport}
        class="flex items-center gap-2 px-4 py-2 bg-emerald-600 hover:bg-emerald-700 text-white text-sm font-bold rounded-lg transition-colors"
      >
        <Icon name="download" size="sm" />
        {$t.exportToExcel}
      </button>
    </div>
  </div>

  <!-- Events Table -->
  <div
    class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl shadow-sm overflow-hidden"
  >
    <table class="w-full" id="events-table">
      <thead class="bg-slate-50 dark:bg-[#0f1821]">
        <tr>
          <th
            class="px-6 py-4 text-left text-xs font-bold text-slate-500 uppercase tracking-wider"
          >
            #
          </th>
          <th
            class="px-6 py-4 text-left text-xs font-bold text-slate-500 uppercase tracking-wider"
          >
            {$t.type}
          </th>
          <th
            class="px-6 py-4 text-left text-xs font-bold text-slate-500 uppercase tracking-wider"
          >
            {$t.startTime}
          </th>
          <th
            class="px-6 py-4 text-left text-xs font-bold text-slate-500 uppercase tracking-wider"
          >
            {$t.endTime}
          </th>
          <th
            class="px-6 py-4 text-left text-xs font-bold text-slate-500 uppercase tracking-wider"
          >
            {$t.duration}
          </th>
          <th
            class="px-6 py-4 text-left text-xs font-bold text-slate-500 uppercase tracking-wider"
          >
            {$t.detail}
          </th>
        </tr>
      </thead>
      <tbody class="divide-y divide-slate-200 dark:divide-[#334a5e]">
        {#each events as event}
          <tr class="hover:bg-slate-50 dark:hover:bg-[#334a5e]/40 transition-colors">
            <td class="px-6 py-4 text-sm text-slate-900 dark:text-white">{event.id}</td>
            <td class="px-6 py-4">
              <div class="flex items-center gap-2">
                <Icon name={event.icon} class="text-amber-500" size="sm" />
                <span class="text-sm text-slate-900 dark:text-white">
                  {$t[event.type as keyof typeof $t]}
                </span>
              </div>
            </td>
            <td class="px-6 py-4 text-sm font-mono text-slate-600 dark:text-slate-400">
              {event.start}
            </td>
            <td class="px-6 py-4 text-sm font-mono text-slate-600 dark:text-slate-400">
              {event.end}
            </td>
            <td class="px-6 py-4 text-sm text-slate-600 dark:text-slate-400">{event.duration}</td>
            <td class="px-6 py-4 text-sm text-slate-600 dark:text-slate-400">{event.detail}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
</div>
