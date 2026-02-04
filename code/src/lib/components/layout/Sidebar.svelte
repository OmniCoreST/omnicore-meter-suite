<script lang="ts">
  import Icon from "$lib/components/common/Icon.svelte";
  import { navigationStore, type Page, t } from "$lib/stores";

  interface NavItem {
    id: Page;
    icon: string;
    labelKey: keyof typeof import("$lib/i18n/tr").tr;
  }

  const mainMenuItems: NavItem[] = [
    { id: "dashboard", icon: "dashboard", labelKey: "dashboard" },
    { id: "short-read", icon: "menu_book", labelKey: "shortRead" },
    { id: "full-read", icon: "assignment", labelKey: "fullRead" },
    { id: "load-profile", icon: "bar_chart", labelKey: "loadProfile" },
    { id: "events", icon: "event_note", labelKey: "events" },
    { id: "alarms", icon: "notifications", labelKey: "alarms" },
  ];

  const settingsMenuItems: NavItem[] = [
    { id: "time-sync", icon: "schedule", labelKey: "timeSync" },
    { id: "password", icon: "lock_reset", labelKey: "passwordChange" },
    { id: "dst", icon: "wb_sunny", labelKey: "dstSettings" },
    { id: "periods", icon: "timer", labelKey: "periodSettings" },
    { id: "tariffs", icon: "payments", labelKey: "tariffSettings" },
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
    <!-- Main Menu -->
    <div class="flex flex-col gap-1 mb-4">
      <p class="text-[10px] font-semibold text-slate-500 uppercase tracking-wider mb-1 px-2">
        {$t.mainMenu}
      </p>
      {#each mainMenuItems as item}
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

    <!-- Settings Menu -->
    <div class="flex flex-col gap-1">
      <p class="text-[10px] font-semibold text-slate-500 uppercase tracking-wider mb-1 px-2">
        {$t.meterSettings}
      </p>
      {#each settingsMenuItems as item}
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
      <Icon name="open_in_new" size="xs" class="text-slate-300 dark:text-slate-600 group-hover:text-primary transition-colors ml-auto" />
    </a>
  </div>
</nav>
