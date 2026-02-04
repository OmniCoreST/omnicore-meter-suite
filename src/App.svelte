<script lang="ts">
  import Layout from "$lib/components/layout/Layout.svelte";
  import { themeStore } from "$lib/stores/theme";

  // Initialize theme from localStorage
  themeStore.init();
</script>

<div
  class="min-h-screen bg-background-light dark:bg-background-dark text-slate-900 dark:text-white font-display antialiased transition-colors duration-300"
  class:dark={$themeStore === "dark"}
>
  <Layout />
</div>
