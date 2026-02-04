<script lang="ts">
  import Icon from "$lib/components/common/Icon.svelte";
  import { t, isConnected } from "$lib/stores";

  let demandPeriod = $state(15);
  let loadProfilePeriod = $state(15);
  let outageThreshold = $state(180);
</script>

<div class="space-y-6">
  <div
    class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm"
  >
    <h3 class="text-xl font-bold text-slate-900 dark:text-white mb-2">{$t.periodSettings}</h3>
    <p class="text-sm text-slate-500 dark:text-slate-400">
      Configure demand period, load profile period, and outage threshold settings.
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

  <!-- Period Settings -->
  <div class="grid grid-cols-1 md:grid-cols-3 gap-6">
    <!-- Demand Period -->
    <div
      class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm"
    >
      <h4 class="font-bold text-slate-900 dark:text-white mb-4 flex items-center gap-2">
        <Icon name="trending_up" class="text-primary" />
        {$t.demandPeriod}
      </h4>
      <div class="space-y-4">
        <select
          bind:value={demandPeriod}
          disabled={!$isConnected}
          class="w-full bg-white dark:bg-[#1a2632] text-slate-700 dark:text-white border border-slate-300 dark:border-[#334a5e] rounded-lg px-4 py-3 focus:border-primary focus:ring-1 focus:ring-primary outline-none disabled:opacity-50 disabled:cursor-not-allowed"
        >
          <option value={15}>15 {$t.minutes}</option>
          <option value={30}>30 {$t.minutes}</option>
          <option value={60}>60 {$t.minutes}</option>
        </select>
        <p class="text-xs text-slate-500">OBIS: 0.8.0</p>
      </div>
    </div>

    <!-- Load Profile Period -->
    <div
      class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm"
    >
      <h4 class="font-bold text-slate-900 dark:text-white mb-4 flex items-center gap-2">
        <Icon name="bar_chart" class="text-primary" />
        {$t.loadProfilePeriod}
      </h4>
      <div class="space-y-4">
        <select
          bind:value={loadProfilePeriod}
          disabled={!$isConnected}
          class="w-full bg-white dark:bg-[#1a2632] text-slate-700 dark:text-white border border-slate-300 dark:border-[#334a5e] rounded-lg px-4 py-3 focus:border-primary focus:ring-1 focus:ring-primary outline-none disabled:opacity-50 disabled:cursor-not-allowed"
        >
          <option value={15}>15 {$t.minutes}</option>
          <option value={30}>30 {$t.minutes}</option>
          <option value={60}>60 {$t.minutes}</option>
        </select>
        <p class="text-xs text-slate-500">OBIS: 0.8.4</p>
      </div>
    </div>

    <!-- Outage Threshold -->
    <div
      class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm"
    >
      <h4 class="font-bold text-slate-900 dark:text-white mb-4 flex items-center gap-2">
        <Icon name="power_off" class="text-primary" />
        {$t.outageThreshold}
      </h4>
      <div class="space-y-4">
        <div class="flex items-center gap-2">
          <input
            type="number"
            bind:value={outageThreshold}
            disabled={!$isConnected}
            class="flex-1 bg-white dark:bg-[#1a2632] text-slate-700 dark:text-white border border-slate-300 dark:border-[#334a5e] rounded-lg px-4 py-3 focus:border-primary focus:ring-1 focus:ring-primary outline-none disabled:opacity-50 disabled:cursor-not-allowed"
          />
          <span class="text-sm text-slate-500">{$t.seconds}</span>
        </div>
        <p class="text-xs text-slate-500">OBIS: 0.9.9</p>
      </div>
    </div>
  </div>

  <!-- Manual Demand Reset -->
  <div
    class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm"
  >
    <h4 class="font-bold text-slate-900 dark:text-white mb-4 flex items-center gap-2">
      <Icon name="restart_alt" class="text-primary" />
      {$t.manualDemandReset}
    </h4>
    <p class="text-sm text-slate-500 dark:text-slate-400 mb-4">
      Reset the maximum demand value. This action cannot be undone.
    </p>
    <button
      disabled={!$isConnected}
      class="flex items-center gap-2 px-6 py-3 bg-red-500 hover:bg-red-600 text-white font-bold rounded-lg transition-all disabled:opacity-50 disabled:cursor-not-allowed"
    >
      <Icon name="restart_alt" />
      {$t.resetDemand}
    </button>
  </div>

  <!-- Save Button -->
  <div class="flex justify-end">
    <button
      disabled={!$isConnected}
      class="flex items-center gap-2 px-6 py-3 bg-primary hover:bg-primary/90 text-white font-bold rounded-lg transition-all disabled:opacity-50 disabled:cursor-not-allowed"
    >
      <Icon name="save" />
      {$t.save}
    </button>
  </div>
</div>
