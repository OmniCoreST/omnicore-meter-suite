<script lang="ts">
  import Sidebar from "./Sidebar.svelte";
  import Header from "./Header.svelte";
  import MeterStatusBar from "./MeterStatusBar.svelte";
  import CommLog from "./CommLog.svelte";
  import UpdateNotification from "$lib/components/common/UpdateNotification.svelte";
  import { navigationStore, localeStore, t } from "$lib/stores";
  import { onMount } from "svelte";

  // Pages
  import Home from "$lib/pages/Home.svelte";
  import ShortRead from "$lib/pages/ShortRead.svelte";
  import FullRead from "$lib/pages/FullRead.svelte";
  import LoadProfile from "$lib/pages/LoadProfile.svelte";
  import Events from "$lib/pages/Events.svelte";
  import Alarms from "$lib/pages/Alarms.svelte";
  import TimeSync from "$lib/pages/TimeSync.svelte";
  import Password from "$lib/pages/Password.svelte";
  import DST from "$lib/pages/DST.svelte";
  import Periods from "$lib/pages/Periods.svelte";
  import Tariffs from "$lib/pages/Tariffs.svelte";

  onMount(() => {
    navigationStore.init();
    localeStore.init();
  });

  const pageComponents = {
    dashboard: Home,
    "short-read": ShortRead,
    "full-read": FullRead,
    "load-profile": LoadProfile,
    events: Events,
    alarms: Alarms,
    "time-sync": TimeSync,
    password: Password,
    dst: DST,
    periods: Periods,
    tariffs: Tariffs,
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
</div>
