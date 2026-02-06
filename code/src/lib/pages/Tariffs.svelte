<script lang="ts">
  import Icon from "$lib/components/common/Icon.svelte";
  import { t, isConnected, meterStore, addLog, errorToast, successToast } from "$lib/stores";
  import { authenticate, writeObis, endSession } from "$lib/utils/tauri";
  interface TimeSlot {
    start: string; // "HH:MM"
    tariff: number; // 1-4
  }

  type DayType = "weekdays" | "saturday" | "sunday";

  const obisMapping = {
    weekdays: { times: "96.50", assignments: "96.60" },
    saturday: { times: "96.51", assignments: "96.61" },
    sunday: { times: "96.52", assignments: "96.62" },
  };

  const tariffColors: Record<number, string> = {
    1: "bg-blue-500",
    2: "bg-orange-500",
    3: "bg-purple-500",
    4: "bg-green-500",
  };

  const tariffLabels: Record<number, string> = {
    1: "tariffT1",
    2: "tariffT2",
    3: "tariffT3",
    4: "tariffT4",
  };

  // Parse switching times: "00000600170022009999999999999999" → ["00:00", "06:00", "17:00", "22:00"]
  function parseSwitchingTimes(value: string): string[] {
    const times: string[] = [];
    for (let i = 0; i < value.length; i += 4) {
      const chunk = value.substring(i, i + 4);
      if (chunk === "9999") break;
      times.push(`${chunk.substring(0, 2)}:${chunk.substring(2, 4)}`);
    }
    return times;
  }

  // Parse tariff assignments: "31230000" → [3, 1, 2, 3]
  function parseTariffAssignments(value: string): number[] {
    const assignments: number[] = [];
    for (const ch of value) {
      const digit = parseInt(ch);
      if (digit === 0) break;
      assignments.push(digit);
    }
    return assignments;
  }

  // Build time slots from switching times + assignments
  function buildSlots(times: string[], assignments: number[]): TimeSlot[] {
    const count = Math.min(times.length, assignments.length);
    return Array.from({ length: count }, (_, i) => ({
      start: times[i],
      tariff: assignments[i],
    }));
  }

  // Parse tariff data from raw meter data (96.50-52 switching times, 96.60-62 assignments)
  let tariffData = $derived.by(() => {
    // @ts-ignore
    const raw: string | null = $meterStore.fullReadData?.rawData || $meterStore.shortReadData?.rawData || null;
    if (!raw) return null;

    const result: Record<DayType, TimeSlot[]> = {
      weekdays: [],
      saturday: [],
      sunday: [],
    };

    let hasAny = false;
    for (const [day, codes] of Object.entries(obisMapping)) {
      const timesMatch = raw.match(new RegExp(`${codes.times.replace(/\./g, '\\.')}\\(([^)]+)\\)`));
      const assignMatch = raw.match(new RegExp(`${codes.assignments.replace(/\./g, '\\.')}\\(([^)]+)\\)`));

      if (timesMatch && assignMatch) {
        const times = parseSwitchingTimes(timesMatch[1]);
        const assignments = parseTariffAssignments(assignMatch[1]);
        result[day as DayType] = buildSlots(times, assignments);
        if (result[day as DayType].length > 0) hasAny = true;
      }
    }

    return hasAny ? result : null;
  });

  // Editable state
  let selectedDay = $state<DayType>("weekdays");

  const defaultSlots: TimeSlot[] = [
    { start: "00:00", tariff: 3 },
    { start: "06:00", tariff: 1 },
    { start: "17:00", tariff: 2 },
    { start: "22:00", tariff: 3 },
  ];

  let editSlots = $state<Record<DayType, TimeSlot[]>>({
    weekdays: defaultSlots.map(s => ({ ...s })),
    saturday: defaultSlots.map(s => ({ ...s })),
    sunday: defaultSlots.map(s => ({ ...s })),
  });

  let initialized = $state(false);

  // Initialize editable state from meter data
  $effect(() => {
    if (tariffData && !initialized) {
      for (const day of ["weekdays", "saturday", "sunday"] as DayType[]) {
        if (tariffData[day].length > 0) {
          editSlots[day] = tariffData[day].map(s => ({ ...s }));
        }
      }
      initialized = true;
    }
  });

  // Get end time for a slot (= start of next slot, or "24:00" for last)
  function getEndTime(slots: TimeSlot[], index: number): string {
    if (index + 1 < slots.length) return slots[index + 1].start;
    return "24:00";
  }

  function addSlot() {
    const slots = editSlots[selectedDay];
    if (slots.length >= 8) return;
    const lastStart = slots[slots.length - 1]?.start || "00:00";
    const [hh, mm] = lastStart.split(":").map(Number);
    const newMinutes = Math.min(hh * 60 + mm + 60, 23 * 60 + 59);
    const newHH = String(Math.floor(newMinutes / 60)).padStart(2, "0");
    const newMM = String(newMinutes % 60).padStart(2, "0");
    editSlots[selectedDay] = [...slots, { start: `${newHH}:${newMM}`, tariff: 1 }];
  }

  function removeSlot(index: number) {
    const slots = editSlots[selectedDay];
    if (slots.length <= 1 || index === 0) return;
    editSlots[selectedDay] = slots.filter((_, i) => i !== index);
  }

  // Encode slots back to OBIS values
  function encodeSwitchingTimes(slots: TimeSlot[]): string {
    let result = "";
    for (let i = 0; i < 8; i++) {
      if (i < slots.length) {
        result += slots[i].start.replace(":", "");
      } else {
        result += "9999";
      }
    }
    return result;
  }

  function encodeTariffAssignments(slots: TimeSlot[]): string {
    let result = "";
    for (let i = 0; i < 8; i++) {
      result += i < slots.length ? String(slots[i].tariff) : "0";
    }
    return result;
  }

  // Password dialog
  let showPasswordDialog = $state(false);
  let password = $state("");
  let passwordError = $state("");
  let isSaving = $state(false);

  function openSaveDialog() {
    password = "";
    passwordError = "";
    showPasswordDialog = true;
  }

  async function handleSave() {
    if (password.length !== 8 || !/^\d{8}$/.test(password)) {
      passwordError = $t.passwordMustBe8Digits;
      return;
    }

    showPasswordDialog = false;
    isSaving = true;
    addLog("info", $t.savingTariffSettings);

    try {
      const authOk = await authenticate(password);
      if (!authOk) {
        addLog("error", $t.errorWrongPassword);
        errorToast($t.errorWrongPassword);
        return;
      }

      for (const [day, codes] of Object.entries(obisMapping)) {
        const slots = editSlots[day as DayType];
        const timesValue = encodeSwitchingTimes(slots);
        const assignValue = encodeTariffAssignments(slots);
        await writeObis(codes.times, timesValue);
        addLog("info", `${codes.times} = ${timesValue}`);
        await writeObis(codes.assignments, assignValue);
        addLog("info", `${codes.assignments} = ${assignValue}`);
      }

      await endSession();
      addLog("success", $t.tariffSaveSuccess);
      successToast($t.tariffSaveSuccess);
    } catch (error) {
      addLog("error", `${$t.logError}: ${error}`);
      errorToast(`${$t.logError}: ${error}`);
    } finally {
      isSaving = false;
    }
  }

  // Timeline helper: compute width % for a slot
  function slotWidth(slots: TimeSlot[], index: number): number {
    const startParts = slots[index].start.split(":").map(Number);
    const startMins = startParts[0] * 60 + startParts[1];
    const endStr = getEndTime(slots, index);
    const endParts = endStr === "24:00" ? [24, 0] : endStr.split(":").map(Number);
    const endMins = endParts[0] * 60 + endParts[1];
    return ((endMins - startMins) / 1440) * 100;
  }
</script>

<div class="space-y-6">
  <!-- Header -->
  <div
    class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm"
  >
    <div class="flex items-start justify-between">
      <div>
        <h3 class="text-xl font-bold text-slate-900 dark:text-white mb-2">{$t.tariffSettings}</h3>
        <p class="text-sm text-slate-500 dark:text-slate-400">
          {$t.tariffSettingsDescription}
        </p>
      </div>
      <button
        onclick={openSaveDialog}
        disabled={!$isConnected || isSaving}
        class="flex items-center gap-2 px-5 py-2.5 bg-primary hover:bg-primary/90 text-white font-bold rounded-xl shadow-lg shadow-primary/20 transition-all disabled:opacity-50 disabled:cursor-not-allowed whitespace-nowrap"
      >
        {#if isSaving}
          <Icon name="sync" class="animate-spin" />
          {$t.saving}
        {:else}
          <Icon name="edit_note" />
          {$t.writeToMeter}
        {/if}
      </button>
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

  <!-- Day Selection + Timeline + Table -->
  <div
    class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm"
  >
    <!-- Day Tabs -->
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
          <div class="w-4 h-4 rounded {tariffColors[tariff]}"></div>
          <span class="text-sm text-slate-600 dark:text-slate-400">
            {$t[tariffLabels[tariff] as keyof typeof $t]}
          </span>
        </div>
      {/each}
    </div>

    <!-- Timeline Visualization -->
    <div class="mb-6">
      <div class="flex items-center gap-1 h-12 rounded-lg overflow-hidden">
        {#each editSlots[selectedDay] as slot, index}
          {@const width = slotWidth(editSlots[selectedDay], index)}
          <div
            class="{tariffColors[slot.tariff] || 'bg-slate-400'} h-full flex items-center justify-center text-white text-xs font-bold"
            style="width: {width}%"
          >
            {#if width > 5}T{slot.tariff}{/if}
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
      <span class="text-xs text-slate-400 font-mono">
        {obisMapping[selectedDay].times} / {obisMapping[selectedDay].assignments}
      </span>
    </div>

    <div class="overflow-x-auto">
      <table class="w-full">
        <thead>
          <tr class="border-b border-slate-200 dark:border-[#334a5e]">
            <th class="px-4 py-3 text-left text-xs font-bold text-slate-500 uppercase">#</th>
            <th class="px-4 py-3 text-left text-xs font-bold text-slate-500 uppercase">{$t.startTime}</th>
            <th class="px-4 py-3 text-left text-xs font-bold text-slate-500 uppercase">{$t.endTime}</th>
            <th class="px-4 py-3 text-left text-xs font-bold text-slate-500 uppercase">{$t.tariff}</th>
            <th class="px-4 py-3 w-12"></th>
          </tr>
        </thead>
        <tbody class="divide-y divide-slate-200 dark:divide-[#334a5e]">
          {#each editSlots[selectedDay] as slot, index}
            <tr>
              <td class="px-4 py-3 text-sm font-bold text-slate-900 dark:text-white">
                {index + 1}
              </td>
              <td class="px-4 py-3">
                {#if index === 0}
                  <span class="text-sm font-mono text-slate-500">00:00</span>
                {:else}
                  <input
                    type="time"
                    bind:value={slot.start}
                    disabled={!$isConnected}
                    class="bg-white dark:bg-[#1a2632] text-slate-700 dark:text-white border border-slate-300 dark:border-[#334a5e] rounded-lg px-3 py-2 text-sm font-mono focus:border-primary focus:ring-1 focus:ring-primary outline-none disabled:opacity-50 disabled:cursor-not-allowed"
                  />
                {/if}
              </td>
              <td class="px-4 py-3">
                <span class="text-sm font-mono text-slate-500">
                  {getEndTime(editSlots[selectedDay], index)}
                </span>
              </td>
              <td class="px-4 py-3">
                <select
                  bind:value={slot.tariff}
                  disabled={!$isConnected}
                  class="bg-white dark:bg-[#1a2632] text-slate-700 dark:text-white border border-slate-300 dark:border-[#334a5e] rounded-lg px-3 py-2 text-sm focus:border-primary focus:ring-1 focus:ring-primary outline-none disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  <option value={1}>{$t.tariffT1}</option>
                  <option value={2}>{$t.tariffT2}</option>
                  <option value={3}>{$t.tariffT3}</option>
                  <option value={4}>{$t.tariffT4}</option>
                </select>
              </td>
              <td class="px-4 py-3">
                {#if index > 0}
                  <button
                    onclick={() => removeSlot(index)}
                    disabled={!$isConnected}
                    class="p-2 text-red-500 hover:bg-red-500/10 rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                  >
                    <Icon name="delete" size="sm" />
                  </button>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>

    {#if editSlots[selectedDay].length < 8}
      <button
        onclick={addSlot}
        disabled={!$isConnected}
        class="mt-4 flex items-center gap-2 text-primary hover:text-primary/80 font-bold text-sm disabled:opacity-50 disabled:cursor-not-allowed"
      >
        <Icon name="add" size="sm" />
        {$t.addTimeSlot}
      </button>
    {/if}
  </div>
</div>

<!-- Password Dialog -->
{#if showPasswordDialog}
  <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50" role="dialog">
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="absolute inset-0" onclick={() => showPasswordDialog = false}></div>
    <div class="relative bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-2xl p-6 w-full max-w-sm shadow-2xl">
      <h3 class="text-lg font-bold text-slate-900 dark:text-white mb-1">{$t.saveTariffs}</h3>
      <p class="text-sm text-slate-500 mb-4">{$t.passwordWarning}</p>

      <div class="mb-4">
        <label class="block text-sm font-bold text-slate-700 dark:text-slate-300 mb-2" for="tariff-password">
          {$t.password}
        </label>
        <input
          id="tariff-password"
          type="password"
          maxlength={8}
          bind:value={password}
          onkeydown={(e) => { if (e.key === "Enter") handleSave(); }}
          placeholder="00000000"
          class="w-full px-4 py-3 bg-white dark:bg-[#1a2632] border border-slate-200 dark:border-[#334a5e] rounded-xl text-center font-mono text-lg tracking-[0.3em] focus:border-primary focus:ring-1 focus:ring-primary outline-none"
        />
        {#if passwordError}
          <p class="text-xs text-red-500 mt-2">{passwordError}</p>
        {/if}
      </div>

      <div class="flex gap-3">
        <button
          onclick={() => showPasswordDialog = false}
          class="flex-1 px-4 py-3 bg-slate-100 dark:bg-[#334a5e] hover:bg-slate-200 dark:hover:bg-[#455a6e] text-slate-700 dark:text-white font-bold rounded-xl transition-colors"
        >
          {$t.cancel}
        </button>
        <button
          onclick={handleSave}
          disabled={password.length !== 8}
          class="flex-1 px-4 py-3 bg-primary hover:bg-primary/90 text-white font-bold rounded-xl shadow-lg shadow-primary/20 transition-all disabled:opacity-50 disabled:cursor-not-allowed"
        >
          <Icon name="edit_note" size="sm" class="inline mr-1" />
          {$t.writeToMeter}
        </button>
      </div>
    </div>
  </div>
{/if}
