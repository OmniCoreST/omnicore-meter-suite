<script lang="ts">
  import Icon from "$lib/components/common/Icon.svelte";
  import { t, isConnected, addLog, errorToast, successToast } from "$lib/stores";
  import { changePassword } from "$lib/utils/tauri";

  let selectedLevel = $state<1 | 2 | 3>(3);
  let currentPassword = $state("");
  let newPassword = $state("");
  let confirmPassword = $state("");
  let isChanging = $state(false);

  let passwordsMatch = $derived(newPassword === confirmPassword);
  let isValidLength = $derived(newPassword.length === 8 && /^\d{8}$/.test(newPassword));
  let isCurrentValid = $derived(currentPassword.length === 8 && /^\d{8}$/.test(currentPassword));

  const levels = [
    { level: 1 as const, label: "P1", sublabel: "Okuyucu", color: "emerald", obis: "96.96.1" },
    { level: 2 as const, label: "P2", sublabel: "Operatör", color: "amber",   obis: "96.96.2" },
    { level: 3 as const, label: "P3", sublabel: "Master",   color: "red",     obis: "96.96.3" },
  ];

  let activeLevel = $derived(levels.find(l => l.level === selectedLevel)!);

  function selectLevel(level: 1 | 2 | 3) {
    selectedLevel = level;
    currentPassword = "";
    newPassword = "";
    confirmPassword = "";
  }

  async function handleChangePassword() {
    isChanging = true;
    try {
      addLog("info", `P${selectedLevel} şifre değiştirme başlatılıyor (OBIS: ${activeLevel.obis})...`);
      const result = await changePassword(currentPassword, newPassword, selectedLevel);
      addLog("success", result);
      successToast(`P${selectedLevel} şifresi başarıyla değiştirildi`);
      currentPassword = "";
      newPassword = "";
      confirmPassword = "";
    } catch (error) {
      addLog("error", `Şifre değiştirme hatası: ${error}`);
      errorToast(`Şifre değiştirme hatası: ${error}`);
    } finally {
      isChanging = false;
    }
  }
</script>

<div class="space-y-6">
  <!-- Header -->
  <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm">
    <h3 class="text-xl font-bold text-slate-900 dark:text-white mb-2">{$t.passwordChange}</h3>
    <p class="text-sm text-slate-500 dark:text-slate-400">Sayaç şifrelerini değiştirin. Tüm işlemler P3 (Master) kimlik doğrulaması gerektirir.</p>

    {#if !$isConnected}
      <div class="mt-4 p-4 bg-amber-500/10 border border-amber-500/20 rounded-xl text-amber-600 dark:text-amber-500 text-sm">
        <div class="flex items-center gap-2">
          <Icon name="warning" />
          <span>Lütfen önce Ana Sayfa'dan sayaca bağlanın.</span>
        </div>
      </div>
    {/if}
  </div>

  <!-- Warning -->
  <div class="bg-red-500/10 border border-red-500/20 rounded-xl p-6 text-red-600 dark:text-red-400">
    <div class="flex items-start gap-3">
      <Icon name="warning" class="text-2xl flex-shrink-0" />
      <div>
        <h4 class="font-bold mb-1">{$t.importantWarning}</h4>
        <p class="text-sm">{$t.passwordWarning}</p>
      </div>
    </div>
  </div>

  <!-- Level Tabs -->
  <div class="grid grid-cols-3 gap-3">
    {#each levels as lvl}
      {@const active = selectedLevel === lvl.level}
      <button
        onclick={() => selectLevel(lvl.level)}
        class="p-4 rounded-xl border-2 text-left transition-all
          {active
            ? `border-${lvl.color}-500 bg-${lvl.color}-500/10`
            : 'border-slate-200 dark:border-[#334a5e] bg-white dark:bg-surface-dark hover:border-slate-300 dark:hover:border-slate-500'}"
      >
        <div class="flex items-center gap-2 mb-1">
          <span class="text-lg font-mono font-bold {active ? `text-${lvl.color}-500` : 'text-slate-700 dark:text-slate-300'}">{lvl.label}</span>
          {#if active}
            <Icon name="check_circle" class="text-{lvl.color}-500 text-sm" size="sm" />
          {/if}
        </div>
        <div class="text-xs text-slate-500 dark:text-slate-400">{lvl.sublabel}</div>
        <div class="text-xs font-mono text-slate-400 mt-1">{lvl.obis}</div>
      </button>
    {/each}
  </div>

  <!-- Password Form -->
  <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm">
    <div class="flex items-center gap-2 mb-6">
      <span class="px-2 py-0.5 bg-{activeLevel.color}-500/10 text-{activeLevel.color}-600 dark:text-{activeLevel.color}-400 text-xs font-bold rounded">
        {activeLevel.label} - {activeLevel.sublabel}
      </span>
      <span class="text-xs text-slate-400">Auth: P3 | Hedef: {activeLevel.obis}</span>
    </div>

    <div class="max-w-md space-y-4">
      <div class="flex flex-col gap-1.5">
        <label class="text-sm font-bold text-slate-700 dark:text-slate-300">
          Mevcut P3 Şifresi (Master)
        </label>
        <input
          type="password"
          bind:value={currentPassword}
          maxlength="8"
          placeholder="••••••••"
          disabled={!$isConnected}
          oninput={(e) => { const t = e.currentTarget; t.value = t.value.replace(/\D/g, ''); currentPassword = t.value; }}
          class="w-full bg-white dark:bg-[#1a2632] text-slate-700 dark:text-white border border-slate-300 dark:border-[#334a5e] rounded-lg px-4 py-3 focus:border-primary focus:ring-1 focus:ring-primary outline-none text-lg font-mono tracking-widest disabled:opacity-50 disabled:cursor-not-allowed
            {currentPassword && !isCurrentValid ? 'border-red-500' : ''}"
        />
        {#if currentPassword && !isCurrentValid}
          <span class="text-xs text-red-500">Şifre tam olarak 8 rakam olmalıdır</span>
        {/if}
      </div>

      <div class="flex flex-col gap-1.5">
        <label class="text-sm font-bold text-slate-700 dark:text-slate-300">
          Yeni {activeLevel.label} Şifresi
        </label>
        <input
          type="password"
          bind:value={newPassword}
          maxlength="8"
          placeholder="••••••••"
          disabled={!$isConnected}
          oninput={(e) => { const t = e.currentTarget; t.value = t.value.replace(/\D/g, ''); newPassword = t.value; }}
          class="w-full bg-white dark:bg-[#1a2632] text-slate-700 dark:text-white border border-slate-300 dark:border-[#334a5e] rounded-lg px-4 py-3 focus:border-primary focus:ring-1 focus:ring-primary outline-none text-lg font-mono tracking-widest disabled:opacity-50 disabled:cursor-not-allowed
            {newPassword && !isValidLength ? 'border-red-500' : ''}"
        />
        {#if newPassword && !isValidLength}
          <span class="text-xs text-red-500">Şifre tam olarak 8 rakam olmalıdır</span>
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
          oninput={(e) => { const t = e.currentTarget; t.value = t.value.replace(/\D/g, ''); confirmPassword = t.value; }}
          class="w-full bg-white dark:bg-[#1a2632] text-slate-700 dark:text-white border border-slate-300 dark:border-[#334a5e] rounded-lg px-4 py-3 focus:border-primary focus:ring-1 focus:ring-primary outline-none text-lg font-mono tracking-widest disabled:opacity-50 disabled:cursor-not-allowed
            {confirmPassword && !passwordsMatch ? 'border-red-500' : ''}"
        />
        {#if confirmPassword && !passwordsMatch}
          <span class="text-xs text-red-500">{$t.passwordsDontMatch}</span>
        {/if}
      </div>

      <button
        onclick={handleChangePassword}
        disabled={!$isConnected || !isValidLength || !passwordsMatch || !isCurrentValid || isChanging}
        class="w-full flex items-center justify-center gap-2 px-6 py-4 bg-primary hover:bg-primary/90 text-white font-bold rounded-xl transition-all disabled:opacity-50 disabled:cursor-not-allowed mt-6"
      >
        {#if isChanging}
          <Icon name="sync" class="animate-spin" />
          {$t.saving}
        {:else}
          <Icon name="lock_reset" />
          {activeLevel.label} Şifresini Değiştir
        {/if}
      </button>
    </div>
  </div>
</div>
