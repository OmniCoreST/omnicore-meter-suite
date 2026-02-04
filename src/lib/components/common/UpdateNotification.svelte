<script lang="ts">
  import Icon from "./Icon.svelte";
  import { t } from "$lib/stores";
  import { checkForUpdates, downloadAndInstall, relaunchApp, type UpdateInfo } from "$lib/utils/updater";
  import { onMount } from "svelte";

  let updateInfo = $state<UpdateInfo | null>(null);
  let isChecking = $state(false);
  let isDownloading = $state(false);
  let downloadProgress = $state(0);
  let showModal = $state(false);
  let error = $state<string | null>(null);
  let downloadComplete = $state(false);

  onMount(() => {
    // Check for updates on app start (after a short delay)
    setTimeout(() => {
      checkUpdates();
    }, 5000);
  });

  async function checkUpdates() {
    if (isChecking) return;

    isChecking = true;
    error = null;

    try {
      updateInfo = await checkForUpdates();
      if (updateInfo) {
        showModal = true;
      }
    } catch (e) {
      error = e instanceof Error ? e.message : "Failed to check for updates";
    } finally {
      isChecking = false;
    }
  }

  async function handleDownload() {
    if (isDownloading || !updateInfo) return;

    isDownloading = true;
    error = null;
    downloadProgress = 0;

    try {
      await downloadAndInstall((progress) => {
        downloadProgress = progress.downloaded;
      });
      downloadComplete = true;
    } catch (e) {
      error = e instanceof Error ? e.message : "Failed to download update";
    } finally {
      isDownloading = false;
    }
  }

  async function handleRelaunch() {
    try {
      await relaunchApp();
    } catch (e) {
      error = e instanceof Error ? e.message : "Failed to restart application";
    }
  }

  function closeModal() {
    showModal = false;
  }
</script>

{#if showModal && updateInfo}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm">
    <div class="bg-white dark:bg-[#1F3244] rounded-2xl shadow-2xl max-w-md w-full mx-4 overflow-hidden">
      <!-- Header -->
      <div class="px-6 py-4 border-b border-slate-200 dark:border-[#334a5e] bg-gradient-to-r from-primary/10 to-emerald-500/10">
        <div class="flex items-center gap-3">
          <div class="p-2 rounded-xl bg-primary/20 text-primary">
            <Icon name="system_update" size="md" />
          </div>
          <div>
            <h2 class="text-lg font-bold text-slate-900 dark:text-white">
              {$t.updateAvailable}
            </h2>
            <p class="text-sm text-slate-500 dark:text-slate-400">
              v{updateInfo.version}
            </p>
          </div>
        </div>
      </div>

      <!-- Content -->
      <div class="px-6 py-4 space-y-4">
        {#if updateInfo.body}
          <div class="text-sm text-slate-600 dark:text-slate-400 max-h-40 overflow-y-auto">
            <p class="font-medium text-slate-900 dark:text-white mb-2">{$t.whatsNew}</p>
            <div class="whitespace-pre-wrap">{updateInfo.body}</div>
          </div>
        {/if}

        {#if isDownloading}
          <div class="space-y-2">
            <div class="flex justify-between text-xs text-slate-500 dark:text-slate-400">
              <span>{$t.downloading}</span>
              <span>{downloadProgress}%</span>
            </div>
            <div class="h-2 bg-slate-200 dark:bg-[#0f1821] rounded-full overflow-hidden">
              <div
                class="h-full bg-gradient-to-r from-primary to-emerald-500 transition-all duration-300"
                style="width: {downloadProgress}%"
              ></div>
            </div>
          </div>
        {/if}

        {#if error}
          <div class="flex items-center gap-2 p-3 rounded-lg bg-red-500/10 text-red-500 text-sm">
            <Icon name="error" size="sm" />
            <span>{error}</span>
          </div>
        {/if}

        {#if downloadComplete}
          <div class="flex items-center gap-2 p-3 rounded-lg bg-emerald-500/10 text-emerald-500 text-sm">
            <Icon name="check_circle" size="sm" />
            <span>{$t.updateReady}</span>
          </div>
        {/if}
      </div>

      <!-- Footer -->
      <div class="px-6 py-4 border-t border-slate-200 dark:border-[#334a5e] flex justify-end gap-3">
        {#if !downloadComplete}
          <button
            onclick={closeModal}
            disabled={isDownloading}
            class="px-4 py-2 text-sm font-medium text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white disabled:opacity-50 transition-colors"
          >
            {$t.later}
          </button>
          <button
            onclick={handleDownload}
            disabled={isDownloading}
            class="flex items-center gap-2 px-4 py-2 text-sm font-bold text-white bg-primary rounded-lg hover:bg-primary/90 disabled:opacity-50 transition-colors shadow-lg shadow-primary/20"
          >
            {#if isDownloading}
              <div class="size-4 border-2 border-white/30 border-t-white rounded-full animate-spin"></div>
              <span>{$t.downloading}</span>
            {:else}
              <Icon name="download" size="sm" />
              <span>{$t.downloadUpdate}</span>
            {/if}
          </button>
        {:else}
          <button
            onclick={closeModal}
            class="px-4 py-2 text-sm font-medium text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white transition-colors"
          >
            {$t.restartLater}
          </button>
          <button
            onclick={handleRelaunch}
            class="flex items-center gap-2 px-4 py-2 text-sm font-bold text-white bg-emerald-500 rounded-lg hover:bg-emerald-600 transition-colors shadow-lg shadow-emerald-500/20"
          >
            <Icon name="restart_alt" size="sm" />
            <span>{$t.restartNow}</span>
          </button>
        {/if}
      </div>
    </div>
  </div>
{/if}
