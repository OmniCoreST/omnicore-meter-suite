<script lang="ts">
  import Icon from "$lib/components/common/Icon.svelte";
  import { navigationStore, type Page, t } from "$lib/stores";

  interface NavItem {
    id: Page;
    icon: string;
    labelKey: keyof typeof import("$lib/i18n/tr").tr;
  }

  // Dashboard at top (no section header)
  const dashboardItem: NavItem = { id: "dashboard", icon: "home", labelKey: "dashboard" };

  // OKUMA (Reading) Section - 8 pages
  const okumaItems: NavItem[] = [
    { id: "overview", icon: "info", labelKey: "overview" },
    { id: "live-measurements", icon: "speed", labelKey: "liveMeasurements" },
    { id: "energy", icon: "bolt", labelKey: "energy" },
    { id: "demand", icon: "trending_up", labelKey: "demand" },
    { id: "load-profile", icon: "bar_chart", labelKey: "loadProfile" },
    { id: "warnings", icon: "warning", labelKey: "warnings" },
    { id: "outages", icon: "power_off", labelKey: "outages" },
    { id: "status-codes", icon: "qr_code_2", labelKey: "statusCodes" },
  ];

  // AYARLAR (Settings) Section - 7 pages
  const ayarlarItems: NavItem[] = [
    { id: "time-date", icon: "schedule", labelKey: "timeDate" },
    { id: "password", icon: "lock_reset", labelKey: "passwordChange" },
    { id: "dst", icon: "wb_sunny", labelKey: "dstSettings" },
    { id: "tariffs", icon: "payments", labelKey: "tariffSettings" },
    { id: "periods", icon: "timer", labelKey: "periodSettings" },
    { id: "relay-control", icon: "power_settings_new", labelKey: "relayControl" },
    { id: "obis-reader", icon: "terminal", labelKey: "obisReader" },
  ];

  function navigate(page: Page) {
    navigationStore.navigate(page);
  }
</script>

<nav
  class="flex w-56 flex-col border-r border-slate-200 dark:border-[#334a5e] bg-white dark:bg-[#111c26] h-screen sticky top-0 z-50 transition-colors duration-300 flex-shrink-0"
>
  <!-- Logo - Fixed at top -->
  <div class="flex items-center gap-2 p-4">
    <div
      class="bg-center bg-no-repeat bg-contain rounded-lg size-8 shadow-lg shadow-primary/10"
      style="background-image: url('/logo.svg');"
    ></div>
    <div class="flex flex-col">
      <h1 class="text-slate-900 dark:text-white text-base font-bold tracking-tight">
        {$t.appName}
      </h1>
      <p class="text-slate-500 dark:text-slate-400 text-[10px] font-medium tracking-wide">
        {$t.appSubtitle}
      </p>
    </div>
  </div>

  <!-- Scrollable Menu Area -->
  <div class="flex-1 overflow-y-auto p-3">
    <!-- Dashboard (no section header) -->
    <div class="mb-3">
      <button
        onclick={() => navigate(dashboardItem.id)}
        class="flex items-center gap-2 px-3 py-2 rounded-lg transition-all text-left w-full
          {$navigationStore === dashboardItem.id
          ? 'bg-primary/10 text-primary'
          : 'hover:bg-slate-100 dark:hover:bg-[#334a5e] text-slate-600 dark:text-slate-300 group'}"
      >
        <Icon
          name={dashboardItem.icon}
          size="sm"
          filled={$navigationStore === dashboardItem.id}
          class={$navigationStore === dashboardItem.id
            ? ""
            : "group-hover:text-primary dark:group-hover:text-white transition-colors"}
        />
        <span
          class="text-xs {$navigationStore === dashboardItem.id
            ? 'font-bold'
            : 'font-medium group-hover:text-slate-900 dark:group-hover:text-white transition-colors'}"
        >
          {$t[dashboardItem.labelKey]}
        </span>
      </button>
    </div>

    <!-- OKUMA Section -->
    <div class="flex flex-col gap-1 mb-4">
      <p class="text-[10px] font-semibold text-slate-500 uppercase tracking-wider mb-1 px-2">
        {$t.okumaSection}
      </p>
      {#each okumaItems as item}
        <button
          onclick={() => navigate(item.id)}
          class="flex items-center gap-2 px-3 py-2 rounded-lg transition-all text-left w-full
            {$navigationStore === item.id
            ? 'bg-primary/10 text-primary'
            : 'hover:bg-slate-100 dark:hover:bg-[#334a5e] text-slate-600 dark:text-slate-300 group'}"
        >
          <Icon
            name={item.icon}
            size="sm"
            filled={$navigationStore === item.id}
            class={$navigationStore === item.id
              ? ""
              : "group-hover:text-primary dark:group-hover:text-white transition-colors"}
          />
          <span
            class="text-xs {$navigationStore === item.id
              ? 'font-bold'
              : 'font-medium group-hover:text-slate-900 dark:group-hover:text-white transition-colors'}"
          >
            {$t[item.labelKey]}
          </span>
        </button>
      {/each}
    </div>

    <!-- AYARLAR Section -->
    <div class="flex flex-col gap-1">
      <p class="text-[10px] font-semibold text-slate-500 uppercase tracking-wider mb-1 px-2">
        {$t.ayarlarSection}
      </p>
      {#each ayarlarItems as item}
        <button
          onclick={() => navigate(item.id)}
          class="flex items-center gap-2 px-3 py-2 rounded-lg transition-all text-left w-full
            {$navigationStore === item.id
            ? 'bg-primary/10 text-primary'
            : 'hover:bg-slate-100 dark:hover:bg-[#334a5e] text-slate-600 dark:text-slate-300 group'}"
        >
          <Icon
            name={item.icon}
            size="sm"
            filled={$navigationStore === item.id}
            class={$navigationStore === item.id
              ? ""
              : "group-hover:text-primary dark:group-hover:text-white transition-colors"}
          />
          <span
            class="text-xs {$navigationStore === item.id
              ? 'font-bold'
              : 'font-medium group-hover:text-slate-900 dark:group-hover:text-white transition-colors'}"
          >
            {$t[item.labelKey]}
          </span>
        </button>
      {/each}
    </div>
  </div>

  <!-- Website Link - Fixed at bottom -->
  <div class="p-4 border-t border-slate-200 dark:border-[#334a5e]">
    <a
      href="https://omnicore.com.tr"
      target="_blank"
      rel="noopener noreferrer"
      class="flex items-center justify-center gap-2 px-3 py-2 rounded-lg bg-slate-50 dark:bg-[#0f1821] hover:bg-primary/10 dark:hover:bg-primary/10 border border-slate-200 dark:border-[#334a5e] hover:border-primary/30 transition-all group"
    >
      <Icon name="language" size="sm" class="text-slate-400 group-hover:text-primary transition-colors" />
      <span class="text-xs font-medium text-slate-600 dark:text-slate-400 group-hover:text-primary transition-colors">
        omnicore.com.tr
      </span>
      <Icon name="open_in_new" size="sm" class="text-slate-300 dark:text-slate-600 group-hover:text-primary transition-colors ml-auto" />
    </a>
  </div>
</nav>
