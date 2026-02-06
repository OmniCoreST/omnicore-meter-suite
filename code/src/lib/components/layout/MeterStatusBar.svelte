<script lang="ts">
  import Icon from "$lib/components/common/Icon.svelte";
  import { t, isConnected, meterStore } from "$lib/stores";

  // Parse FF code bits to count active alarm conditions
  let alarmCount = $derived.by(() => {
    const ffCode = $meterStore.shortReadData?.ffCode;
    if (!ffCode || ffCode === "0000000000000000") return 0;

    try {
      const ffValue = BigInt("0x" + ffCode);
      let count = 0;
      if ((ffValue & (1n << 37n)) !== 0n) count++; // System battery low
      if ((ffValue & (1n << 38n)) !== 0n) count++; // Clock battery low
      if ((ffValue & (1n << 6n)) !== 0n) count++;  // Top cover open
      if ((ffValue & (1n << 5n)) !== 0n) count++;  // Terminal cover open
      if ((ffValue & (1n << 11n)) !== 0n || (ffValue & (1n << 12n)) !== 0n || (ffValue & (1n << 13n)) !== 0n) count++; // Magnetic
      return count;
    } catch {
      return 0;
    }
  });

  // Calculate time drift from meter time vs computer time
  let timeDriftSeconds = $derived.by(() => {
    const data = $meterStore.shortReadData;
    if (!data?.meterDate || !data?.meterTime) return 0;

    try {
      const meterDateTime = new Date(`${data.meterDate}T${data.meterTime}`);
      return Math.round((Date.now() - meterDateTime.getTime()) / 1000);
    } catch {
      return 0;
    }
  });

  let timeCorrect = $derived(Math.abs(timeDriftSeconds) <= 30);

  // Derive most notable event from warning counters in raw data
  let recentEvent = $derived.by((): string | null => {
    const data = $meterStore.shortReadData;
    if (!data) return null;

    // @ts-ignore - rawData exists on full read results
    const raw: string = data.rawData || "";
    if (!raw) return null;

    const voltageCount = raw.match(/96\.7\.4\((\d+)\)/);
    if (voltageCount && parseInt(voltageCount[1]) > 0) {
      return `${voltageCount[1]}x ${$t.voltageWarnings}`;
    }

    const magneticCount = raw.match(/96\.7\.6\((\d+)\)/);
    if (magneticCount && parseInt(magneticCount[1]) > 0) {
      return `${magneticCount[1]}x ${$t.magneticField}`;
    }

    const currentCount = raw.match(/96\.7\.5\((\d+)\)/);
    if (currentCount && parseInt(currentCount[1]) > 0) {
      return `${currentCount[1]}x ${$t.currentWarnings}`;
    }

    return null;
  });
</script>

<div
  class="bg-slate-50 dark:bg-[#0f1821] border-b border-slate-200 dark:border-[#334a5e] px-6 py-2"
>
  <div class="max-w-7xl mx-auto flex items-center justify-between gap-6">
    <!-- Status Indicators -->
    <div class="flex items-center gap-6">
      {#if $isConnected}
        <!-- Alarm Status -->
        <div class="flex items-center gap-2">
          <Icon
            name="notifications"
            size="sm"
            class={alarmCount > 0 ? "text-red-500" : "text-emerald-500"}
          />
          <span class="text-xs font-medium text-slate-600 dark:text-slate-400">
            {$t.alarmStatus}:
          </span>
          <span
            class="text-xs font-bold {alarmCount > 0
              ? 'text-red-500'
              : 'text-emerald-500'}"
          >
            {alarmCount > 0 ? `${alarmCount} ${$t.activeAlarms}` : $t.noAlarms}
          </span>
        </div>

        <!-- Time Status -->
        <div class="flex items-center gap-2">
          <Icon
            name="schedule"
            size="sm"
            class={timeCorrect ? "text-emerald-500" : "text-amber-500"}
          />
          <span class="text-xs font-medium text-slate-600 dark:text-slate-400">
            {$t.timeStatus}:
          </span>
          <span
            class="text-xs font-bold {timeCorrect
              ? 'text-emerald-500'
              : 'text-amber-500'}"
          >
            {timeCorrect ? $t.timeCorrect : $t.timeDrift}
          </span>
        </div>

        <!-- Recent Event -->
        <div class="flex items-center gap-2">
          <Icon
            name="event_note"
            size="sm"
            class={recentEvent ? "text-blue-500" : "text-slate-400"}
          />
          <span class="text-xs font-medium text-slate-600 dark:text-slate-400">
            {$t.recentEvent}:
          </span>
          <span class="text-xs font-bold text-slate-700 dark:text-slate-300">
            {recentEvent || $t.noRecentEvents}
          </span>
        </div>
      {:else}
        <!-- Not Connected State -->
        <div class="flex items-center gap-2">
          <Icon name="link_off" size="sm" class="text-slate-400" />
          <span class="text-xs font-medium text-slate-500 dark:text-slate-500">
            {$t.waitingForConnection}
          </span>
        </div>

        <!-- Alarm Status Placeholder -->
        <div class="flex items-center gap-2 opacity-50">
          <Icon name="notifications" size="sm" class="text-slate-400" />
          <span class="text-xs font-medium text-slate-400">
            {$t.alarmStatus}: —
          </span>
        </div>

        <!-- Time Status Placeholder -->
        <div class="flex items-center gap-2 opacity-50">
          <Icon name="schedule" size="sm" class="text-slate-400" />
          <span class="text-xs font-medium text-slate-400">
            {$t.timeStatus}: —
          </span>
        </div>

        <!-- Recent Event Placeholder -->
        <div class="flex items-center gap-2 opacity-50">
          <Icon name="event_note" size="sm" class="text-slate-400" />
          <span class="text-xs font-medium text-slate-400">
            {$t.recentEvent}: —
          </span>
        </div>
      {/if}
    </div>
  </div>
</div>
