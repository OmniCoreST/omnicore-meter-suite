<script lang="ts">
  import { toastStore } from "$lib/stores";
  import Icon from "./Icon.svelte";

  function getIcon(type: string) {
    switch (type) {
      case "success": return "check_circle";
      case "error": return "error";
      case "warning": return "warning";
      case "info": return "info";
      default: return "info";
    }
  }

  function getColorClasses(type: string) {
    switch (type) {
      case "success":
        return "bg-emerald-500/95 text-white border-emerald-600";
      case "error":
        return "bg-red-500/95 text-white border-red-600";
      case "warning":
        return "bg-amber-500/95 text-white border-amber-600";
      case "info":
        return "bg-blue-500/95 text-white border-blue-600";
      default:
        return "bg-slate-500/95 text-white border-slate-600";
    }
  }
</script>

<div class="fixed top-4 right-4 z-[9999] flex flex-col gap-2 pointer-events-none">
  {#each $toastStore as toast (toast.id)}
    <div
      class="pointer-events-auto flex items-center gap-3 px-4 py-3 rounded-lg border shadow-lg backdrop-blur-sm animate-in slide-in-from-right duration-300 min-w-[300px] max-w-[400px] {getColorClasses(toast.type)}"
    >
      <Icon name={getIcon(toast.type)} size="sm" />
      <span class="flex-1 text-sm font-medium">{toast.message}</span>
      <button
        onclick={() => toastStore.dismiss(toast.id)}
        class="hover:bg-white/20 rounded p-1 transition-colors"
      >
        <Icon name="close" size="xs" />
      </button>
    </div>
  {/each}
</div>
