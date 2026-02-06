<script lang="ts">
  import Sidebar from "./Sidebar.svelte";
  import Header from "./Header.svelte";
  import MeterStatusBar from "./MeterStatusBar.svelte";
  import CommLog from "./CommLog.svelte";
  import UpdateNotification from "$lib/components/common/UpdateNotification.svelte";
  import Toast from "$lib/components/common/Toast.svelte";
  import { navigationStore, localeStore, t } from "$lib/stores";
  import { onMount } from "svelte";

  // Pages - Dashboard
  import Home from "$lib/pages/Home.svelte";
  import Sessions from "$lib/pages/Sessions.svelte";

  // Pages - OKUMA (Reading) Section
  import Overview from "$lib/pages/Overview.svelte";
  import LiveMeasurements from "$lib/pages/LiveMeasurements.svelte";
  import Energy from "$lib/pages/Energy.svelte";
  import Demand from "$lib/pages/Demand.svelte";
  import LoadProfile from "$lib/pages/LoadProfile.svelte";
  import Warnings from "$lib/pages/Warnings.svelte";
  import Outages from "$lib/pages/Outages.svelte";
  import StatusCodes from "$lib/pages/StatusCodes.svelte";

  // Pages - AYARLAR (Settings) Section
  import TimeDate from "$lib/pages/TimeDate.svelte";
  import Password from "$lib/pages/Password.svelte";
  import DST from "$lib/pages/DST.svelte";
  import Tariffs from "$lib/pages/Tariffs.svelte";
  import Periods from "$lib/pages/Periods.svelte";
  import RelayControl from "$lib/pages/RelayControl.svelte";
  import ObisReader from "$lib/pages/ObisReader.svelte";

  onMount(() => {
    navigationStore.init();
    localeStore.init();
  });

  const pageComponents = {
    // Dashboard
    dashboard: Home,
    sessions: Sessions,
    // OKUMA Section
    overview: Overview,
    "live-measurements": LiveMeasurements,
    energy: Energy,
    demand: Demand,
    "load-profile": LoadProfile,
    warnings: Warnings,
    outages: Outages,
    "status-codes": StatusCodes,
    // AYARLAR Section
    "time-date": TimeDate,
    password: Password,
    dst: DST,
    tariffs: Tariffs,
    periods: Periods,
    "relay-control": RelayControl,
    "obis-reader": ObisReader,
  };

  let currentPage = $derived(pageComponents[$navigationStore] || Home);
</script>

<div class="flex overflow-hidden h-screen">
  <Sidebar />

  <main class="flex-1 flex flex-col h-screen overflow-hidden relative">
    <Header />
    <MeterStatusBar />

    <div class="flex-1 overflow-y-auto p-4 md:p-8 scroll-smooth">
      <div class="max-w-7xl mx-auto space-y-8">
        <svelte:component this={currentPage} />
      </div>

      <footer class="mt-12 py-6 border-t border-slate-200 dark:border-[#334a5e] text-center">
        <p class="text-slate-500 dark:text-slate-500 text-xs">{$t.copyright}</p>
      </footer>
    </div>

    <CommLog />
  </main>

  <!-- Auto-update notification -->
  <UpdateNotification />

  <!-- Toast Notifications -->
  <Toast />
</div>
