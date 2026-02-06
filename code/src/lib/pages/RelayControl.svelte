<script lang="ts">
  import Icon from "$lib/components/common/Icon.svelte";
  import { t, isConnected, meterStore, addLog } from "$lib/stores";

  let isToggling = $state(false);
  let showConfirmDialog = $state(false);
  let pendingAction = $state<"connect" | "disconnect" | null>(null);

  let relayActive = $derived($meterStore.shortReadData?.relayStatus === "active");

  async function toggleRelay(action: "connect" | "disconnect") {
    if (action === "disconnect") {
      // Show confirmation for disconnect
      pendingAction = action;
      showConfirmDialog = true;
      return;
    }

    await executeRelayAction(action);
  }

  async function executeRelayAction(action: "connect" | "disconnect") {
    if (!$isConnected || isToggling) return;

    isToggling = true;
    showConfirmDialog = false;
    pendingAction = null;

    try {
      addLog("info", `Role ${action === "connect" ? "baglaniyor" : "kesiliyor"}...`);

      // TODO: Call actual Tauri command
      // await setRelayState(action === "connect");

      // Simulate action
      await new Promise(r => setTimeout(r, 2000));

      addLog("success", `Role basariyla ${action === "connect" ? "baglandi" : "kesildi"}!`);
    } catch (error) {
      addLog("error", `Role islemi hatasi: ${error}`);
    } finally {
      isToggling = false;
    }
  }

  function cancelAction() {
    showConfirmDialog = false;
    pendingAction = null;
  }
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

  {#if $isConnected}
    <!-- Large Status Indicator -->
    <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-8 shadow-sm">
      <div class="flex flex-col items-center">
        <!-- Big Status Circle -->
        <div
          class="w-48 h-48 rounded-full flex items-center justify-center mb-6 transition-all duration-500
            {relayActive
              ? 'bg-gradient-to-br from-emerald-400 to-emerald-600 shadow-2xl shadow-emerald-500/50'
              : 'bg-gradient-to-br from-red-400 to-red-600 shadow-2xl shadow-red-500/50'}"
        >
          <Icon
            name={relayActive ? "power" : "power_off"}
            class="text-white text-7xl"
          />
        </div>

        <!-- Status Text -->
        <div class="text-center">
          <h2 class="text-3xl font-bold {relayActive ? 'text-emerald-500' : 'text-red-500'} mb-2">
            {relayActive ? $t.energyOn : $t.energyCut}
          </h2>
          <p class="text-slate-500">
            {relayActive ? $t.relayConnected : $t.relayDisconnected}
          </p>
        </div>
      </div>
    </div>

    <!-- Control Buttons -->
    <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm">
      <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
        <button
          onclick={() => toggleRelay("connect")}
          disabled={relayActive || isToggling}
          class="flex items-center justify-center gap-3 px-6 py-6 rounded-xl font-bold text-lg transition-all
            {relayActive || isToggling
              ? 'bg-slate-100 dark:bg-[#334a5e] text-slate-400 cursor-not-allowed'
              : 'bg-emerald-500 hover:bg-emerald-600 text-white shadow-lg shadow-emerald-500/30'}"
        >
          {#if isToggling && pendingAction !== "disconnect"}
            <Icon name="sync" class="animate-spin text-2xl" />
            Baglaniyor...
          {:else}
            <Icon name="power" class="text-2xl" />
            {$t.connectRelay}
          {/if}
        </button>

        <button
          onclick={() => toggleRelay("disconnect")}
          disabled={!relayActive || isToggling}
          class="flex items-center justify-center gap-3 px-6 py-6 rounded-xl font-bold text-lg transition-all
            {!relayActive || isToggling
              ? 'bg-slate-100 dark:bg-[#334a5e] text-slate-400 cursor-not-allowed'
              : 'bg-red-500 hover:bg-red-600 text-white shadow-lg shadow-red-500/30'}"
        >
          {#if isToggling && pendingAction === "disconnect"}
            <Icon name="sync" class="animate-spin text-2xl" />
            Kesiliyor...
          {:else}
            <Icon name="power_off" class="text-2xl" />
            {$t.disconnectRelay}
          {/if}
        </button>
      </div>

      <!-- Warning Message -->
      <div class="mt-4 p-4 bg-amber-500/10 border border-amber-500/20 rounded-xl">
        <div class="flex items-start gap-3">
          <Icon name="warning" class="text-amber-500 flex-shrink-0" />
          <p class="text-sm text-amber-600 dark:text-amber-400">{$t.relayWarning}</p>
        </div>
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
            <p class="text-sm text-slate-500">Sayac uzerindeki talep butonu durumu (96.91.0)</p>
          </div>
        </div>
        <div class="flex items-center gap-3">
          <span class="px-4 py-2 bg-emerald-500/10 border border-emerald-500/20 rounded-lg font-bold text-emerald-600">
            {$t.demandButtonEnabled}
          </span>
          <button
            disabled={!$isConnected}
            class="px-4 py-2 bg-slate-100 dark:bg-[#334a5e] hover:bg-slate-200 dark:hover:bg-[#455a6e] rounded-lg font-medium text-slate-600 dark:text-slate-400 transition-colors disabled:opacity-50"
          >
            Degistir
          </button>
        </div>
      </div>
    </div>
  {/if}
</div>

<!-- Confirmation Dialog -->
{#if showConfirmDialog}
  <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
    <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-2xl p-6 max-w-md mx-4 shadow-2xl">
      <div class="text-center mb-6">
        <div class="w-16 h-16 bg-red-500/10 rounded-full flex items-center justify-center mx-auto mb-4">
          <Icon name="warning" class="text-red-500 text-3xl" />
        </div>
        <h3 class="text-xl font-bold text-slate-900 dark:text-white mb-2">{$t.disconnectRelay}</h3>
        <p class="text-slate-500">{$t.confirmDisconnect}</p>
      </div>

      <div class="flex gap-3">
        <button
          onclick={cancelAction}
          class="flex-1 px-4 py-3 bg-slate-100 dark:bg-[#334a5e] hover:bg-slate-200 dark:hover:bg-[#455a6e] rounded-xl font-bold text-slate-700 dark:text-white transition-colors"
        >
          {$t.cancel}
        </button>
        <button
          onclick={() => executeRelayAction("disconnect")}
          class="flex-1 px-4 py-3 bg-red-500 hover:bg-red-600 rounded-xl font-bold text-white transition-colors"
        >
          {$t.confirm}
        </button>
      </div>
    </div>
  </div>
{/if}
