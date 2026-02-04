<script lang="ts">
  import Icon from "$lib/components/common/Icon.svelte";
  import { t, isConnected } from "$lib/stores";

  let currentPassword = $state("");
  let newPassword = $state("");
  let confirmPassword = $state("");

  let passwordsMatch = $derived(newPassword === confirmPassword);
  let isValidLength = $derived(newPassword.length === 8 && /^\d+$/.test(newPassword));
</script>

<div class="space-y-6">
  <div
    class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm"
  >
    <h3 class="text-xl font-bold text-slate-900 dark:text-white mb-2">{$t.passwordChange}</h3>
    <p class="text-sm text-slate-500 dark:text-slate-400">
      Change the meter programming password.
    </p>

    {#if !$isConnected}
      <div
        class="mt-4 p-4 bg-amber-500/10 border border-amber-500/20 rounded-xl text-amber-600 dark:text-amber-500 text-sm"
      >
        <div class="flex items-center gap-2">
          <Icon name="warning" />
          <span>Please connect to a meter first from the Dashboard.</span>
        </div>
      </div>
    {/if}
  </div>

  <!-- Warning -->
  <div
    class="bg-red-500/10 border border-red-500/20 rounded-xl p-6 text-red-600 dark:text-red-400"
  >
    <div class="flex items-start gap-3">
      <Icon name="warning" class="text-2xl flex-shrink-0" />
      <div>
        <h4 class="font-bold mb-1">Important Warning</h4>
        <p class="text-sm">{$t.passwordWarning}</p>
      </div>
    </div>
  </div>

  <!-- Password Form -->
  <div
    class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm"
  >
    <div class="max-w-md space-y-4">
      <div class="flex flex-col gap-1.5">
        <label class="text-sm font-bold text-slate-700 dark:text-slate-300">
          {$t.currentPassword}
        </label>
        <input
          type="password"
          bind:value={currentPassword}
          maxlength="8"
          placeholder="••••••••"
          disabled={!$isConnected}
          class="w-full bg-white dark:bg-[#1a2632] text-slate-700 dark:text-white border border-slate-300 dark:border-[#334a5e] rounded-lg px-4 py-3 focus:border-primary focus:ring-1 focus:ring-primary outline-none text-lg font-mono tracking-widest disabled:opacity-50 disabled:cursor-not-allowed"
        />
      </div>

      <div class="flex flex-col gap-1.5">
        <label class="text-sm font-bold text-slate-700 dark:text-slate-300">
          {$t.newPassword}
        </label>
        <input
          type="password"
          bind:value={newPassword}
          maxlength="8"
          placeholder="••••••••"
          disabled={!$isConnected}
          class="w-full bg-white dark:bg-[#1a2632] text-slate-700 dark:text-white border border-slate-300 dark:border-[#334a5e] rounded-lg px-4 py-3 focus:border-primary focus:ring-1 focus:ring-primary outline-none text-lg font-mono tracking-widest disabled:opacity-50 disabled:cursor-not-allowed
            {newPassword && !isValidLength ? 'border-red-500' : ''}"
        />
        {#if newPassword && !isValidLength}
          <span class="text-xs text-red-500">{$t.passwordMustBe8Digits}</span>
        {/if}
      </div>

      <div class="flex flex-col gap-1.5">
        <label class="text-sm font-bold text-slate-700 dark:text-slate-300">
          {$t.confirmPassword}
        </label>
        <input
          type="password"
          bind:value={confirmPassword}
          maxlength="8"
          placeholder="••••••••"
          disabled={!$isConnected}
          class="w-full bg-white dark:bg-[#1a2632] text-slate-700 dark:text-white border border-slate-300 dark:border-[#334a5e] rounded-lg px-4 py-3 focus:border-primary focus:ring-1 focus:ring-primary outline-none text-lg font-mono tracking-widest disabled:opacity-50 disabled:cursor-not-allowed
            {confirmPassword && !passwordsMatch ? 'border-red-500' : ''}"
        />
        {#if confirmPassword && !passwordsMatch}
          <span class="text-xs text-red-500">{$t.passwordsDontMatch}</span>
        {/if}
      </div>

      <button
        disabled={!$isConnected || !isValidLength || !passwordsMatch || !currentPassword}
        class="w-full flex items-center justify-center gap-2 px-6 py-4 bg-primary hover:bg-primary/90 text-white font-bold rounded-xl transition-all disabled:opacity-50 disabled:cursor-not-allowed mt-6"
      >
        <Icon name="lock_reset" />
        {$t.changePassword}
      </button>
    </div>
  </div>
</div>
