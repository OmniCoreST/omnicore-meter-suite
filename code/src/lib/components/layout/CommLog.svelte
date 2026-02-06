<script lang="ts">
  import Icon from "$lib/components/common/Icon.svelte";
  import { logsStore, addLog, t } from "$lib/stores";
  import type { LogType } from "$lib/stores";
  import { onMount, onDestroy } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { onCommLog } from "$lib/utils/tauri";

  let txActive = $state(false);
  let rxActive = $state(false);
  let txTimer: ReturnType<typeof setTimeout> | null = null;
  let rxTimer: ReturnType<typeof setTimeout> | null = null;
  let unlistenActivity: (() => void) | null = null;
  let unlistenCommLog: (() => void) | null = null;

  onMount(async () => {
    // Listen to communication activity events for LED blinking
    unlistenActivity = await listen("comm-activity", (event: any) => {
      const activityType = event.payload.type;

      if (activityType === 'tx') {
        txActive = true;
        if (txTimer) clearTimeout(txTimer);
        txTimer = setTimeout(() => txActive = false, 200);
      } else if (activityType === 'rx') {
        rxActive = true;
        if (rxTimer) clearTimeout(rxTimer);
        rxTimer = setTimeout(() => rxActive = false, 200);
      }
    });

    // Global listener for all backend comm-log events
    unlistenCommLog = await onCommLog((event) => {
      const logType = event.logType.toLowerCase() as LogType;
      addLog(logType, event.message);
    });
  });

  onDestroy(() => {
    if (unlistenActivity) unlistenActivity();
    if (unlistenCommLog) unlistenCommLog();
    if (txTimer) clearTimeout(txTimer);
    if (rxTimer) clearTimeout(rxTimer);
  });

  let logContainer: HTMLDivElement | undefined = $state();

  // Auto-scroll to bottom when new log entries arrive
  $effect(() => {
    // Track the logs array to detect changes
    if ($logsStore.length && logContainer && isOpen) {
      requestAnimationFrame(() => {
        if (logContainer) {
          logContainer.scrollTop = logContainer.scrollHeight;
        }
      });
    }
  });

  let isOpen = $state(false); // Collapsed by default
  let panelHeight = $state(192); // 48 * 4 = 192px (h-48)
  let isResizing = $state(false);
  let startY = 0;
  let startHeight = 0;

  const MIN_HEIGHT = 100;
  const MAX_HEIGHT = 500;
  const STORAGE_KEY = "commLogHeight";

  onMount(() => {
    // Load saved height from localStorage
    const savedHeight = localStorage.getItem(STORAGE_KEY);
    if (savedHeight) {
      const parsed = parseInt(savedHeight, 10);
      if (!isNaN(parsed) && parsed >= MIN_HEIGHT && parsed <= MAX_HEIGHT) {
        panelHeight = parsed;
      }
    }
  });

  function saveHeight() {
    localStorage.setItem(STORAGE_KEY, String(panelHeight));
  }

  function startResize(e: MouseEvent) {
    e.preventDefault();
    isResizing = true;
    startY = e.clientY;
    startHeight = panelHeight;
    document.body.classList.add("no-select");

    window.addEventListener("mousemove", handleResize);
    window.addEventListener("mouseup", stopResize);
  }

  function handleResize(e: MouseEvent) {
    if (!isResizing) return;

    const deltaY = startY - e.clientY;
    const newHeight = Math.min(MAX_HEIGHT, Math.max(MIN_HEIGHT, startHeight + deltaY));
    panelHeight = newHeight;
  }

  function stopResize() {
    if (isResizing) {
      isResizing = false;
      document.body.classList.remove("no-select");
      saveHeight();

      window.removeEventListener("mousemove", handleResize);
      window.removeEventListener("mouseup", stopResize);
    }
  }

  function formatTime(date: Date): string {
    return date.toLocaleTimeString("en-GB", {
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });
  }

  function getTypeLabel(type: LogType): string {
    const labels: Record<LogType, keyof typeof import("$lib/i18n/tr").tr> = {
      info: "logInfo",
      warn: "logWarn",
      success: "logSuccess",
      error: "logError",
      tx: "logTx",
      rx: "logRx",
    };
    return $t[labels[type]];
  }

  function getTypeClass(type: LogType): string {
    const classes: Record<LogType, string> = {
      info: "text-blue-600 dark:text-blue-400",
      warn: "text-amber-600 dark:text-amber-500",
      success: "text-emerald-600 dark:text-emerald-500",
      error: "text-red-600 dark:text-red-500",
      tx: "text-violet-600 dark:text-violet-400",
      rx: "text-emerald-600 dark:text-emerald-500",
    };
    return classes[type];
  }

  function clearLogs() {
    logsStore.clear();
  }

  function exportLogs() {
    const logs = $logsStore;
    const content = logs
      .map((log) => `[${formatTime(log.timestamp)}] ${getTypeLabel(log.type)} - ${log.message}`)
      .join("\n");

    const blob = new Blob([content], { type: "text/plain" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `comm-log-${new Date().toISOString().slice(0, 10)}.txt`;
    a.click();
    URL.revokeObjectURL(url);
  }
</script>

<div
  class="flex-none border-t border-slate-200 dark:border-[#334a5e] bg-slate-100 dark:bg-[#0d151c] z-30 shadow-[0_-5px_15px_rgba(0,0,0,0.05)] dark:shadow-[0_-5px_15px_rgba(0,0,0,0.3)]"
>
  <!-- Resize handle (only shown when panel is open) -->
  {#if isOpen}
    <div
      role="separator"
      tabindex="0"
      aria-label="Resize communication log panel"
      class="resize-handle h-2 flex items-center justify-center text-slate-400 dark:text-slate-600 hover:text-primary transition-colors"
      onmousedown={startResize}
      onkeydown={(e) => {
        if (e.key === "ArrowUp") {
          panelHeight = Math.min(MAX_HEIGHT, panelHeight + 20);
          saveHeight();
        } else if (e.key === "ArrowDown") {
          panelHeight = Math.max(MIN_HEIGHT, panelHeight - 20);
          saveHeight();
        }
      }}
    ></div>
  {/if}

  <details bind:open={isOpen} class="group">
    <summary
      class="flex items-center justify-between px-4 py-2 bg-slate-200 dark:bg-[#111c26] cursor-pointer hover:bg-slate-300 dark:hover:bg-[#1f3244] select-none text-xs text-slate-700 dark:text-slate-300 font-medium uppercase tracking-wider transition-colors border-b border-transparent group-open:border-slate-300 dark:group-open:border-[#334a5e]"
    >
      <div class="flex items-center gap-3">
        <div class="flex items-center gap-2">
          <Icon name="terminal" size="sm" />
          <span>{$t.communicationLog}</span>
        </div>
        <span class="h-3 w-[1px] bg-slate-400 dark:bg-slate-600"></span>
        <div class="flex items-center gap-3">
          <!-- TX LED -->
          <div class="flex items-center gap-1.5">
            <div class="relative">
              <div class="size-2 rounded-full bg-amber-500/30 dark:bg-amber-500/20"></div>
              {#if txActive}
                <div class="absolute inset-0 size-2 rounded-full bg-amber-500 animate-ping"></div>
                <div class="absolute inset-0 size-2 rounded-full bg-amber-500"></div>
              {/if}
            </div>
            <span class="text-[10px] font-bold text-amber-600 dark:text-amber-400">TX</span>
          </div>
          <!-- RX LED -->
          <div class="flex items-center gap-1.5">
            <div class="relative">
              <div class="size-2 rounded-full bg-emerald-500/30 dark:bg-emerald-500/20"></div>
              {#if rxActive}
                <div class="absolute inset-0 size-2 rounded-full bg-emerald-500 animate-ping"></div>
                <div class="absolute inset-0 size-2 rounded-full bg-emerald-500"></div>
              {/if}
            </div>
            <span class="text-[10px] font-bold text-emerald-600 dark:text-emerald-400">RX</span>
          </div>
        </div>
      </div>
      <div class="flex items-center gap-4">
        <div class="flex gap-2">
          <button
            onclick={clearLogs}
            class="hover:text-primary dark:hover:text-white"
            title={$t.clearConsole}
          >
            <Icon name="block" size="sm" />
          </button>
          <button
            onclick={exportLogs}
            class="hover:text-primary dark:hover:text-white"
            title={$t.exportLog}
          >
            <Icon name="download" size="sm" />
          </button>
        </div>
        <Icon
          name="expand_less"
          size="sm"
          class="group-open:rotate-180 transition-transform"
        />
      </div>
    </summary>

    <div
      bind:this={logContainer}
      style="height: {panelHeight}px"
      class="overflow-y-auto p-0 font-mono text-xs text-slate-600 dark:text-slate-400 bg-slate-50 dark:bg-[#0f1821] scroll-smooth transition-none"
      class:transition-none={isResizing}
    >
      <div class="min-w-full inline-block align-middle">
        <!-- Header -->
        <div
          class="grid grid-cols-[100px_100px_1fr] gap-0 border-b border-slate-200 dark:border-[#334a5e]/50 bg-slate-100 dark:bg-[#111c26] sticky top-0 z-10 text-slate-500 font-bold uppercase text-[10px] tracking-wider"
        >
          <div class="px-4 py-2">{$t.time}</div>
          <div class="px-4 py-2">{$t.type}</div>
          <div class="px-4 py-2">{$t.details}</div>
        </div>

        <!-- Log Entries -->
        <div class="flex flex-col">
          {#each $logsStore as log (log.id)}
            <div
              class="grid grid-cols-[100px_100px_1fr] gap-0 hover:bg-white dark:hover:bg-white/5 transition-colors border-b border-slate-200 dark:border-[#334a5e]/20 last:border-0 group"
            >
              <div class="px-4 py-1.5 text-primary group-hover:text-slate-900 dark:group-hover:text-white">
                [{formatTime(log.timestamp)}]
              </div>
              <div class="px-4 py-1.5 {getTypeClass(log.type)} font-bold">
                {getTypeLabel(log.type)}
              </div>
              <div
                class="px-4 py-1.5 text-slate-600 dark:text-slate-400 group-hover:text-slate-900 dark:group-hover:text-slate-300 {log.type ===
                  'tx' || log.type === 'rx'
                  ? 'font-mono text-[11px] whitespace-pre-wrap'
                  : ''}"
              >
                {log.message}
                {#if log.raw}
                  <span class="text-slate-400 dark:text-slate-600 ml-2">({log.raw})</span>
                {/if}
              </div>
            </div>
          {:else}
            <div class="px-4 py-4 text-center text-slate-400 dark:text-slate-600">
              No log entries
            </div>
          {/each}
        </div>
      </div>
    </div>
  </details>
</div>
