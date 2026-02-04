<script lang="ts">
  import Icon from "$lib/components/common/Icon.svelte";
  import { t, isConnected, meterStore } from "$lib/stores";

  // Mock data - will be replaced with real data from meter
  const hasAlarms = $derived($meterStore.shortReadData?.ffCode !== "0000000000000000");
  const alarmCount = $derived(hasAlarms ? 2 : 0);
  const timeCorrect = $derived(true);
  const recentEvent = $derived<string | null>(null);
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
