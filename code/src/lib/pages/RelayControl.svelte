<script lang="ts">
  import Icon from "$lib/components/common/Icon.svelte";
  import { t, isConnected } from "$lib/stores";
</script>

<div class="space-y-6">
  <!-- Header -->
  <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm">
    <div class="flex items-start justify-between">
      <div>
        <h3 class="text-xl font-bold text-slate-900 dark:text-white mb-2">{$t.relayControl}</h3>
        <p class="text-sm text-slate-500 dark:text-slate-400">{$t.relayControlDescription}</p>
      </div>
    </div>

    {#if !$isConnected}
      <div class="mt-4 p-4 bg-amber-500/10 border border-amber-500/20 rounded-xl text-amber-600 dark:text-amber-500 text-sm">
        <div class="flex items-center gap-2">
          <Icon name="warning" />
          <span>{$t.connectFirstWarning}</span>
        </div>
      </div>
    {/if}
  </div>

  <!-- Grayed out - not yet implemented -->
  <div class="relative">
    <div class="opacity-30 pointer-events-none select-none space-y-6">
      <!-- Large Status Indicator -->
      <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-8 shadow-sm">
        <div class="flex flex-col items-center">
          <div class="w-48 h-48 rounded-full flex items-center justify-center mb-6 bg-gradient-to-br from-slate-300 to-slate-400 shadow-2xl">
            <Icon name="power" class="text-white text-7xl" />
          </div>
          <div class="text-center">
            <h2 class="text-3xl font-bold text-slate-400 mb-2">{$t.relayState}</h2>
            <p class="text-slate-400">-</p>
          </div>
        </div>
      </div>

      <!-- Control Buttons -->
      <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm">
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <button disabled class="flex items-center justify-center gap-3 px-6 py-6 rounded-xl font-bold text-lg bg-slate-100 dark:bg-[#334a5e] text-slate-400 cursor-not-allowed">
            <Icon name="power" class="text-2xl" />
            {$t.connectRelay}
          </button>
          <button disabled class="flex items-center justify-center gap-3 px-6 py-6 rounded-xl font-bold text-lg bg-slate-100 dark:bg-[#334a5e] text-slate-400 cursor-not-allowed">
            <Icon name="power_off" class="text-2xl" />
            {$t.disconnectRelay}
          </button>
        </div>
      </div>

      <!-- Demand Button Status -->
      <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-4">
            <div class="p-3 rounded-xl bg-primary/10">
              <Icon name="touch_app" class="text-primary text-2xl" />
            </div>
            <div>
              <h4 class="font-bold text-slate-900 dark:text-white">{$t.demandButtonStatus}</h4>
              <p class="text-sm text-slate-500">96.91.0</p>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Not Supported Overlay -->
    <div class="absolute inset-0 flex items-start justify-center pt-8">
      <div class="bg-white/80 dark:bg-surface-dark/80 backdrop-blur-sm border border-slate-200 dark:border-[#334a5e] rounded-2xl px-8 py-6 text-center shadow-lg">
        <Icon name="block" class="text-4xl text-slate-400 mb-3" />
        <p class="text-lg font-bold text-slate-600 dark:text-slate-300">{$t.notSupported}</p>
        <p class="text-sm text-slate-400 mt-1">{$t.notSupportedDescription}</p>
      </div>
    </div>
  </div>
</div>

