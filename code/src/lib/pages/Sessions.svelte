<script lang="ts">
  import Icon from "$lib/components/common/Icon.svelte";
  import { t, sessionsStore, connectionStore, meterStore, errorToast, successToast, navigationStore, type SessionInfo } from "$lib/stores";
  import { onMount } from "svelte";
  import { loadSessionFile } from "$lib/utils/tauri";

  // Search query
  let searchQuery = $state("");

  // Filtered sessions based on search
  let filteredSessions = $derived(
    $sessionsStore.filter(session => {
      if (!searchQuery.trim()) return true;
      const query = searchQuery.toLowerCase();
      return (
        session.serialNumber.toLowerCase().includes(query) ||
        session.flag.toLowerCase().includes(query) ||
        session.model.toLowerCase().includes(query) ||
        (session.note && session.note.toLowerCase().includes(query))
      );
    })
  );

  // Session load confirmation dialog
  let showLoadConfirmDialog = $state(false);
  let pendingSessionToLoad = $state<SessionInfo | null>(null);
  let isLoadingSession = $state(false);

  // Session delete confirmation dialog
  let showDeleteConfirmDialog = $state(false);
  let pendingSessionToDelete = $state<SessionInfo | null>(null);
  let isDeletingSession = $state(false);

  onMount(() => {
    // Refresh sessions list on mount
    sessionsStore.refresh();
  });

  function loadSession(session: SessionInfo) {
    // Check if there's an active session with data
    if ($meterStore.shortReadData) {
      // Show confirmation dialog
      pendingSessionToLoad = session;
      showLoadConfirmDialog = true;
    } else {
      // No active session, load directly
      performSessionLoad(session);
    }
  }

  function cancelLoadSession() {
    showLoadConfirmDialog = false;
    pendingSessionToLoad = null;
  }

  function confirmLoadSession() {
    if (pendingSessionToLoad) {
      performSessionLoad(pendingSessionToLoad);
    }
    showLoadConfirmDialog = false;
    pendingSessionToLoad = null;
  }

  async function performSessionLoad(session: SessionInfo) {
    if (!session.fileName) {
      errorToast($t.sessionLoadError || "Session file not found");
      return;
    }

    isLoadingSession = true;

    try {
      const sessionData = await loadSessionFile(session.fileName);

      // Extract meter data from session
      const meterData = sessionData.meterData as {
        shortReadData?: Record<string, unknown>;
        fullReadData?: Record<string, unknown>;
        loadProfileData?: Record<string, unknown>;
        meterType?: "single-phase" | "three-phase" | "kombi";
        isBidirectional?: boolean;
      };

      if (meterData?.shortReadData) {
        // Load the data into meterStore
        meterStore.setShortReadData(
          meterData.shortReadData as unknown as Parameters<typeof meterStore.setShortReadData>[0],
          meterData.meterType || "single-phase",
          meterData.isBidirectional || false
        );

        // Load full read data if available
        if (meterData.fullReadData) {
          meterStore.setFullReadData(
            meterData.fullReadData as unknown as Parameters<typeof meterStore.setFullReadData>[0]
          );
        }

        // Load profile data if available
        if (meterData.loadProfileData) {
          meterStore.setLoadProfileData(
            meterData.loadProfileData as unknown as Parameters<typeof meterStore.setLoadProfileData>[0]
          );
        }

        // Set connection status to "connected" so pages display the data
        connectionStore.setConnected({
          flag: sessionData.flag || session.flag,
          manufacturer: sessionData.flag || session.flag,
          edasId: "",
          model: sessionData.model || "",
          baudChar: "",
          serialNumber: sessionData.serialNumber || session.serialNumber,
        });

        successToast($t.sessionLoaded || `Session loaded: ${session.flag} — ${session.serialNumber}`);

        // Navigate to overview page after loading
        navigationStore.navigate("overview");
      } else {
        errorToast($t.sessionLoadError || "Invalid session data");
      }
    } catch (e) {
      console.error("Failed to load session:", e);
      errorToast(`${$t.sessionLoadError || "Failed to load session"}: ${e}`);
    } finally {
      isLoadingSession = false;
    }
  }

  function promptDeleteSession(session: SessionInfo, event: MouseEvent) {
    event.stopPropagation();
    pendingSessionToDelete = session;
    showDeleteConfirmDialog = true;
  }

  function cancelDeleteSession() {
    showDeleteConfirmDialog = false;
    pendingSessionToDelete = null;
  }

  async function confirmDeleteSession() {
    if (!pendingSessionToDelete) return;

    isDeletingSession = true;
    try {
      await sessionsStore.delete(pendingSessionToDelete.fileName);
      successToast($t.sessionDeleted || "Session deleted");
    } catch (e) {
      console.error("Failed to delete session:", e);
      errorToast(`${$t.sessionDeleteError || "Failed to delete session"}: ${e}`);
    } finally {
      isDeletingSession = false;
      showDeleteConfirmDialog = false;
      pendingSessionToDelete = null;
    }
  }

  function goBack() {
    navigationStore.navigate("dashboard");
  }

  function formatDate(dateStr: string): string {
    if (!dateStr) return "";
    try {
      const date = new Date(dateStr);
      return date.toLocaleDateString("tr-TR", {
        year: "numeric",
        month: "short",
        day: "numeric",
        hour: "2-digit",
        minute: "2-digit"
      });
    } catch {
      return dateStr;
    }
  }
</script>

<!-- Page Header -->
<div class="flex items-center justify-between mb-6">
  <div class="flex items-center gap-4">
    <button
      onclick={goBack}
      class="p-2 rounded-lg text-slate-500 hover:text-slate-700 dark:text-slate-400 dark:hover:text-slate-200 hover:bg-slate-100 dark:hover:bg-[#334a5e] transition-colors"
    >
      <Icon name="arrow_back" />
    </button>
    <div>
      <h1 class="text-2xl font-bold text-slate-900 dark:text-white">{$t.allSessions || "All Sessions"}</h1>
      <p class="text-sm text-slate-500 dark:text-slate-400">
        {$t.sessionsDescription || "View and manage all saved meter reading sessions"}
      </p>
    </div>
  </div>
  <div class="text-sm text-slate-500 dark:text-slate-400">
    {filteredSessions.length} / {$sessionsStore.length} {$t.sessions || "sessions"}
  </div>
</div>

<!-- Search Bar -->
<div class="mb-6">
  <div class="relative max-w-md">
    <Icon
      name="search"
      size="sm"
      class="absolute left-3 top-1/2 -translate-y-1/2 text-slate-400"
    />
    <input
      type="text"
      bind:value={searchQuery}
      placeholder={$t.searchSessionsPlaceholder || "Search by serial number, flag, or note..."}
      class="w-full pl-10 pr-4 py-2.5 bg-white dark:bg-[#1a2632] border border-slate-300 dark:border-[#334a5e] rounded-lg text-sm text-slate-900 dark:text-white placeholder-slate-400 dark:placeholder-slate-500 focus:border-primary focus:ring-1 focus:ring-primary transition-colors"
    />
    {#if searchQuery}
      <button
        onclick={() => searchQuery = ""}
        class="absolute right-3 top-1/2 -translate-y-1/2 text-slate-400 hover:text-slate-600 dark:hover:text-slate-300"
      >
        <Icon name="close" size="sm" />
      </button>
    {/if}
  </div>
</div>

<!-- Sessions List -->
<div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl shadow-sm overflow-hidden">
  {#if $sessionsStore.length === 0}
    <!-- No Sessions At All -->
    <div class="p-12 text-center">
      <Icon name="folder_off" class="text-slate-300 dark:text-slate-600 text-5xl mb-3" />
      <p class="text-lg font-bold text-slate-500 dark:text-slate-400">{$t.noPreviousSessions}</p>
      <p class="text-sm text-slate-400 dark:text-slate-500 mt-1">
        {$t.noSessionsHint || "Sessions will appear here after you read and save meter data."}
      </p>
    </div>
  {:else if filteredSessions.length === 0}
    <!-- No Search Results -->
    <div class="p-12 text-center">
      <Icon name="search_off" class="text-slate-300 dark:text-slate-600 text-5xl mb-3" />
      <p class="text-lg font-bold text-slate-500 dark:text-slate-400">{$t.noSearchResults || "No results found"}</p>
      <p class="text-sm text-slate-400 dark:text-slate-500 mt-1">
        {$t.tryDifferentSearch || "Try a different search term."}
      </p>
    </div>
  {:else}
    <!-- Sessions Table -->
    <div class="overflow-x-auto">
      <table class="w-full">
        <thead>
          <tr class="bg-slate-50 dark:bg-[#0f1821] border-b border-slate-200 dark:border-[#334a5e]">
            <th class="px-4 py-3 text-left text-xs font-bold text-slate-500 dark:text-slate-400 uppercase tracking-wider">
              {$t.flag || "Flag"}
            </th>
            <th class="px-4 py-3 text-left text-xs font-bold text-slate-500 dark:text-slate-400 uppercase tracking-wider">
              {$t.serialNumber}
            </th>
            <th class="px-4 py-3 text-left text-xs font-bold text-slate-500 dark:text-slate-400 uppercase tracking-wider hidden sm:table-cell">
              {$t.model || "Model"}
            </th>
            <th class="px-4 py-3 text-left text-xs font-bold text-slate-500 dark:text-slate-400 uppercase tracking-wider hidden md:table-cell">
              {$t.dateTime}
            </th>
            <th class="px-4 py-3 text-left text-xs font-bold text-slate-500 dark:text-slate-400 uppercase tracking-wider hidden lg:table-cell">
              {$t.note || "Note"}
            </th>
            <th class="px-4 py-3 text-right text-xs font-bold text-slate-500 dark:text-slate-400 uppercase tracking-wider">
              {$t.actions || "Actions"}
            </th>
          </tr>
        </thead>
        <tbody class="divide-y divide-slate-200 dark:divide-[#334a5e]">
          {#each filteredSessions as session}
            <tr class="hover:bg-slate-50 dark:hover:bg-[#1a2632] transition-colors group">
              <!-- Flag -->
              <td class="px-4 py-3">
                <div
                  class="inline-flex items-center justify-center w-10 h-8 rounded font-mono font-bold text-xs
                    {session.success
                    ? 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400'
                    : 'bg-red-500/10 text-red-600 dark:text-red-400'}"
                >
                  {session.flag}
                </div>
              </td>

              <!-- Serial Number -->
              <td class="px-4 py-3">
                <span class="font-mono font-bold text-slate-900 dark:text-white">
                  {session.serialNumber}
                </span>
              </td>

              <!-- Model -->
              <td class="px-4 py-3 hidden sm:table-cell">
                <span class="text-sm text-slate-600 dark:text-slate-400">
                  {session.model || "-"}
                </span>
              </td>

              <!-- Date/Time -->
              <td class="px-4 py-3 hidden md:table-cell">
                <span class="text-sm text-slate-600 dark:text-slate-400">
                  {formatDate(session.dateTime)}
                </span>
              </td>

              <!-- Note -->
              <td class="px-4 py-3 hidden lg:table-cell">
                <span class="text-sm text-slate-500 dark:text-slate-500 truncate max-w-[200px] block">
                  {session.note || "-"}
                </span>
              </td>

              <!-- Actions -->
              <td class="px-4 py-3">
                <div class="flex items-center justify-end gap-2">
                  <button
                    onclick={() => loadSession(session)}
                    class="flex items-center gap-1.5 px-3 py-1.5 text-xs font-bold text-primary hover:bg-primary/10 rounded-lg transition-colors"
                  >
                    <Icon name="folder_open" size="sm" />
                    <span class="hidden sm:inline">{$t.load || "Load"}</span>
                  </button>
                  <button
                    onclick={(e) => promptDeleteSession(session, e)}
                    class="p-1.5 rounded-lg text-slate-400 hover:text-red-500 hover:bg-red-50 dark:hover:bg-red-500/10 transition-colors opacity-0 group-hover:opacity-100"
                    title={$t.delete}
                  >
                    <Icon name="delete" size="sm" />
                  </button>
                </div>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>

<!-- Session Load Confirmation Dialog -->
{#if showLoadConfirmDialog && pendingSessionToLoad}
  <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50" onclick={cancelLoadSession}>
    <div
      class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl shadow-xl w-full max-w-md mx-4"
      onclick={(e) => e.stopPropagation()}
    >
      <!-- Dialog Header -->
      <div class="flex items-center gap-3 px-5 py-4 border-b border-slate-200 dark:border-[#334a5e]">
        <div class="p-2 bg-amber-500/10 rounded-lg">
          <Icon name="warning" class="text-amber-500" />
        </div>
        <h3 class="text-lg font-bold text-slate-900 dark:text-white">{$t.loadSessionConfirmTitle || "Load Session?"}</h3>
      </div>

      <!-- Dialog Body -->
      <div class="p-5 space-y-4">
        <p class="text-sm text-slate-600 dark:text-slate-400">
          {$t.loadSessionConfirmMessage || "There is existing meter data. Loading this session will replace the current data."}
        </p>

        <!-- Session to load info -->
        <div class="p-3 bg-slate-50 dark:bg-[#0f1821] rounded-lg">
          <p class="text-xs text-slate-500 mb-1">{$t.sessionToLoad || "Session to load"}</p>
          <p class="text-sm font-bold text-slate-900 dark:text-white">
            {pendingSessionToLoad.flag} — {pendingSessionToLoad.serialNumber}
          </p>
          <p class="text-xs text-slate-500">{formatDate(pendingSessionToLoad.dateTime)}</p>
        </div>
      </div>

      <!-- Dialog Footer -->
      <div class="flex justify-end gap-3 px-5 py-4 border-t border-slate-200 dark:border-[#334a5e]">
        <button
          onclick={cancelLoadSession}
          class="px-4 py-2 text-sm font-bold text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-[#334a5e] rounded-lg transition-colors"
        >
          {$t.cancel}
        </button>
        <button
          onclick={confirmLoadSession}
          class="flex items-center gap-2 px-4 py-2 bg-primary hover:bg-primary/90 text-white text-sm font-bold rounded-lg transition-colors"
        >
          <Icon name="folder_open" size="sm" />
          {$t.loadSession || "Load Session"}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Loading Overlay -->
{#if isLoadingSession}
  <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
    <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl shadow-xl p-8 text-center">
      <Icon name="sync" class="text-primary text-4xl mb-3 animate-spin" />
      <p class="text-sm font-bold text-slate-900 dark:text-white">{$t.loadingSession || "Loading session..."}</p>
    </div>
  </div>
{/if}

<!-- Delete Confirmation Dialog -->
{#if showDeleteConfirmDialog && pendingSessionToDelete}
  <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50" onclick={cancelDeleteSession}>
    <div
      class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl shadow-xl w-full max-w-md mx-4"
      onclick={(e) => e.stopPropagation()}
    >
      <!-- Dialog Header -->
      <div class="flex items-center gap-3 px-5 py-4 border-b border-slate-200 dark:border-[#334a5e]">
        <div class="p-2 bg-red-500/10 rounded-lg">
          <Icon name="delete" class="text-red-500" />
        </div>
        <h3 class="text-lg font-bold text-slate-900 dark:text-white">{$t.deleteSessionConfirmTitle || "Delete Session?"}</h3>
      </div>

      <!-- Dialog Body -->
      <div class="p-5 space-y-4">
        <p class="text-sm text-slate-600 dark:text-slate-400">
          {$t.deleteSessionConfirmMessage || "This action cannot be undone. The session file will be permanently deleted."}
        </p>

        <!-- Session to delete info -->
        <div class="p-3 bg-red-50 dark:bg-red-500/10 border border-red-200 dark:border-red-500/20 rounded-lg">
          <p class="text-xs text-red-500 mb-1">{$t.sessionToDelete || "Session to delete"}</p>
          <p class="text-sm font-bold text-slate-900 dark:text-white">
            {pendingSessionToDelete.flag} — {pendingSessionToDelete.serialNumber}
          </p>
          <p class="text-xs text-slate-500">{formatDate(pendingSessionToDelete.dateTime)}</p>
        </div>
      </div>

      <!-- Dialog Footer -->
      <div class="flex justify-end gap-3 px-5 py-4 border-t border-slate-200 dark:border-[#334a5e]">
        <button
          onclick={cancelDeleteSession}
          class="px-4 py-2 text-sm font-bold text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-[#334a5e] rounded-lg transition-colors"
        >
          {$t.cancel}
        </button>
        <button
          onclick={confirmDeleteSession}
          disabled={isDeletingSession}
          class="flex items-center gap-2 px-4 py-2 bg-red-500 hover:bg-red-600 text-white text-sm font-bold rounded-lg transition-colors disabled:opacity-50"
        >
          {#if isDeletingSession}
            <Icon name="sync" size="sm" class="animate-spin" />
          {:else}
            <Icon name="delete" size="sm" />
          {/if}
          {$t.delete}
        </button>
      </div>
    </div>
  </div>
{/if}
