<script lang="ts">
  import Icon from "$lib/components/common/Icon.svelte";
  import { t, isConnected } from "$lib/stores";
  import { exportToExcel } from "$lib/utils/export";

  let selectedDay = $state<"weekdays" | "saturday" | "sunday">("weekdays");

  // Mock tariff schedule
  let weekdaysSchedule = $state([
    { start: "00:00", end: "06:00", tariff: 3 },
    { start: "06:00", end: "17:00", tariff: 1 },
    { start: "17:00", end: "22:00", tariff: 2 },
    { start: "22:00", end: "24:00", tariff: 3 },
  ]);

  const tariffColors = {
    1: "bg-blue-500",
    2: "bg-orange-500",
    3: "bg-purple-500",
    4: "bg-green-500",
  };

  const tariffLabels = {
    1: "tariffT1",
    2: "tariffT2",
    3: "tariffT3",
    4: "tariffT4",
  };

  function handleExport() {
    const dayLabels = {
      weekdays: $t.weekdays,
      saturday: $t.saturday,
      sunday: $t.sunday,
    };

    const exportData = weekdaysSchedule.map((slot, index) => ({
      slot: index + 1,
      day: dayLabels[selectedDay],
      start: slot.start,
      end: slot.end,
      tariff: $t[tariffLabels[slot.tariff as keyof typeof tariffLabels] as keyof typeof $t],
    }));

    exportToExcel(exportData, `tariffs_${selectedDay}`, [
      { key: "slot", label: "#" },
      { key: "day", label: "Day" },
      { key: "start", label: $t.startTime },
      { key: "end", label: $t.endTime },
      { key: "tariff", label: "Tariff" },
    ]);
  }
</script>

<div class="space-y-6">
  <div
    class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm"
  >
    <h3 class="text-xl font-bold text-slate-900 dark:text-white mb-2">{$t.tariffSettings}</h3>
    <p class="text-sm text-slate-500 dark:text-slate-400">
      Configure tariff time zones for weekdays, Saturday, and Sunday.
    </p>

    {#if !$isConnected}
      <div
        class="mt-4 p-4 bg-amber-500/10 border border-amber-500/20 rounded-xl text-amber-600 dark:text-amber-500 text-sm"
      >
        <div class="flex items-center gap-2">
          <Icon name="warning" />
          <span>Please connect to a meter first from the Dashboard.</span>
        </div>
      </div>
    {/if}
  </div>

  <!-- Day Selection -->
  <div
    class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm"
  >
    <div class="flex gap-4 mb-6">
      <button
        onclick={() => (selectedDay = "weekdays")}
        class="px-6 py-3 rounded-lg font-bold transition-all
          {selectedDay === 'weekdays'
          ? 'bg-primary text-white'
          : 'bg-slate-100 dark:bg-[#334a5e] text-slate-600 dark:text-slate-300 hover:bg-slate-200 dark:hover:bg-[#455a6e]'}"
      >
        {$t.weekdays}
      </button>
      <button
        onclick={() => (selectedDay = "saturday")}
        class="px-6 py-3 rounded-lg font-bold transition-all
          {selectedDay === 'saturday'
          ? 'bg-primary text-white'
          : 'bg-slate-100 dark:bg-[#334a5e] text-slate-600 dark:text-slate-300 hover:bg-slate-200 dark:hover:bg-[#455a6e]'}"
      >
        {$t.saturday}
      </button>
      <button
        onclick={() => (selectedDay = "sunday")}
        class="px-6 py-3 rounded-lg font-bold transition-all
          {selectedDay === 'sunday'
          ? 'bg-primary text-white'
          : 'bg-slate-100 dark:bg-[#334a5e] text-slate-600 dark:text-slate-300 hover:bg-slate-200 dark:hover:bg-[#455a6e]'}"
      >
        {$t.sunday}
      </button>
    </div>

    <!-- Tariff Legend -->
    <div class="flex gap-6 mb-6">
      {#each [1, 2, 3, 4] as tariff}
        <div class="flex items-center gap-2">
          <div class="w-4 h-4 rounded {tariffColors[tariff as keyof typeof tariffColors]}"></div>
          <span class="text-sm text-slate-600 dark:text-slate-400">
            {$t[tariffLabels[tariff as keyof typeof tariffLabels] as keyof typeof $t]}
          </span>
        </div>
      {/each}
    </div>

    <!-- Timeline Visualization -->
    <div class="mb-6">
      <div class="flex items-center gap-1 h-12 rounded-lg overflow-hidden">
        {#each weekdaysSchedule as slot}
          {@const startHour = parseInt(slot.start.split(":")[0])}
          {@const endHour = slot.end === "24:00" ? 24 : parseInt(slot.end.split(":")[0])}
          {@const width = ((endHour - startHour) / 24) * 100}
          <div
            class="{tariffColors[
              slot.tariff as keyof typeof tariffColors
            ]} h-full flex items-center justify-center text-white text-xs font-bold"
            style="width: {width}%"
          >
            T{slot.tariff}
          </div>
        {/each}
      </div>
      <div class="flex justify-between mt-2 text-xs text-slate-500">
        <span>00:00</span>
        <span>06:00</span>
        <span>12:00</span>
        <span>18:00</span>
        <span>24:00</span>
      </div>
    </div>

    <!-- Time Slots Table -->
    <div class="flex items-center justify-between mb-4">
      <h4 class="font-bold text-slate-900 dark:text-white">{$t.timeSlots}</h4>
      <button
        onclick={handleExport}
        class="flex items-center gap-2 px-4 py-2 bg-emerald-600 hover:bg-emerald-700 text-white text-sm font-bold rounded-lg transition-colors"
      >
        <Icon name="download" size="sm" />
        {$t.exportToExcel}
      </button>
    </div>
    <div class="space-y-4">
      {#each weekdaysSchedule as slot, index}
        <div class="flex items-center gap-4">
          <span class="text-sm font-bold text-slate-500 w-8">{index + 1}.</span>
          <input
            type="time"
            bind:value={slot.start}
            disabled={!$isConnected}
            class="bg-white dark:bg-[#1a2632] text-slate-700 dark:text-white border border-slate-300 dark:border-[#334a5e] rounded-lg px-4 py-2 focus:border-primary focus:ring-1 focus:ring-primary outline-none disabled:opacity-50 disabled:cursor-not-allowed"
          />
          <span class="text-slate-500">-</span>
          <input
            type="time"
            bind:value={slot.end}
            disabled={!$isConnected}
            class="bg-white dark:bg-[#1a2632] text-slate-700 dark:text-white border border-slate-300 dark:border-[#334a5e] rounded-lg px-4 py-2 focus:border-primary focus:ring-1 focus:ring-primary outline-none disabled:opacity-50 disabled:cursor-not-allowed"
          />
          <select
            bind:value={slot.tariff}
            disabled={!$isConnected}
            class="bg-white dark:bg-[#1a2632] text-slate-700 dark:text-white border border-slate-300 dark:border-[#334a5e] rounded-lg px-4 py-2 focus:border-primary focus:ring-1 focus:ring-primary outline-none disabled:opacity-50 disabled:cursor-not-allowed"
          >
            <option value={1}>{$t.tariffT1}</option>
            <option value={2}>{$t.tariffT2}</option>
            <option value={3}>{$t.tariffT3}</option>
            <option value={4}>{$t.tariffT4}</option>
          </select>
          <button
            disabled={!$isConnected}
            class="p-2 text-red-500 hover:bg-red-500/10 rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            <Icon name="delete" size="sm" />
          </button>
        </div>
      {/each}
    </div>

    <button
      disabled={!$isConnected}
      class="mt-4 flex items-center gap-2 text-primary hover:text-primary/80 font-bold text-sm disabled:opacity-50 disabled:cursor-not-allowed"
    >
      <Icon name="add" size="sm" />
      Add Time Slot
    </button>
  </div>

  <!-- Save Button -->
  <div class="flex justify-end">
    <button
      disabled={!$isConnected}
      class="flex items-center gap-2 px-6 py-3 bg-primary hover:bg-primary/90 text-white font-bold rounded-lg transition-all disabled:opacity-50 disabled:cursor-not-allowed"
    >
      <Icon name="save" />
      {$t.saveTariffs}
    </button>
  </div>
</div>
