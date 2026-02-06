<script lang="ts">
  import Icon from "$lib/components/common/Icon.svelte";
  import { t, isConnected, meterStore, isMeterReading, addLog } from "$lib/stores";
  import { readShort } from "$lib/utils/tauri";

  let isRefreshing = $state(false);

  async function refreshMeasurements() {
    if (isRefreshing || $isMeterReading) return;
    isRefreshing = true;
    meterStore.setReading(true);
    try {
      addLog("info", "Anlık ölçümler yenileniyor (Kısa okuma)...");
      const result = await readShort();
      // Detect meter type by OBIS code presence (not voltage value — unloaded phases read 0V)
      let meterType: "single-phase" | "three-phase" | "kombi" = "single-phase";
      const raw = result.rawData || "";
      if (raw.includes("52.7.0") || raw.includes("72.7.0")) {
        meterType = "three-phase";
      }
      meterStore.setShortReadData(result, meterType, false);
      addLog("success", "Anlık ölçümler güncellendi");
    } catch (e) {
      addLog("error", `Yenileme hatası: ${e}`);
    } finally {
      isRefreshing = false;
      meterStore.setReading(false);
    }
  }

  function fmt(value: number | undefined, decimals: number = 1): string {
    if (value === undefined || value === null) return "-";
    return value.toLocaleString("tr-TR", {
      minimumFractionDigits: decimals,
      maximumFractionDigits: decimals,
    });
  }

  function fmtTime(date: Date | null): string {
    if (!date) return "--:--:--";
    return date.toLocaleTimeString("tr-TR", { hour: "2-digit", minute: "2-digit", second: "2-digit" });
  }

  // --- Shared gauge geometry ---
  const CX = 100, CY = 108, R = 82;

  function valToAngle(val: number, min: number, max: number): number {
    return 180 - ((Math.max(min, Math.min(max, val)) - min) / (max - min)) * 180;
  }

  function toXY(deg: number, r = R) {
    const rad = (deg * Math.PI) / 180;
    return { x: CX + r * Math.cos(rad), y: CY - r * Math.sin(rad) };
  }

  function arcD(a1: number, a2: number, r = R): string {
    const s = toXY(a1, r), e = toXY(a2, r);
    return `M ${s.x.toFixed(1)} ${s.y.toFixed(1)} A ${r} ${r} 0 ${a1 - a2 > 180 ? 1 : 0} 1 ${e.x.toFixed(1)} ${e.y.toFixed(1)}`;
  }

  // --- Voltage gauge: 150–290V, allowed 207–253V ---
  const V_MIN = 150, V_MAX = 290;
  const vZones = [
    { v1: 150, v2: 195, color: "#ef4444" },
    { v1: 195, v2: 207, color: "#f59e0b" },
    { v1: 207, v2: 253, color: "#10b981" },
    { v1: 253, v2: 265, color: "#f59e0b" },
    { v1: 265, v2: 290, color: "#ef4444" },
  ];
  const vTicks = [
    { v: 150, label: "150" },
    { v: 207, label: "207" },
    { v: 230, label: "230" },
    { v: 253, label: "253" },
    { v: 290, label: "290" },
  ];

  function vColor(v: number | undefined): string {
    if (!v) return "#94a3b8";
    if (v < 195 || v > 265) return "#ef4444";
    if (v < 207 || v > 253) return "#f59e0b";
    return "#10b981";
  }

  function vQuality(v: number | undefined) {
    if (!v) return { label: "-", bg: "bg-slate-100 dark:bg-slate-800", text: "text-slate-400", dot: "bg-slate-400", pulse: false };
    if (v < 195 || v > 265) return { label: "Kritik", bg: "bg-red-500/10", text: "text-red-500", dot: "bg-red-500", pulse: true };
    if (v < 207 || v > 253) return { label: "Uyarı", bg: "bg-amber-500/10", text: "text-amber-500", dot: "bg-amber-500", pulse: false };
    return { label: "Normal", bg: "bg-emerald-500/10", text: "text-emerald-500", dot: "bg-emerald-500", pulse: false };
  }

  function cardRing(v: number | undefined): string {
    if (!v) return "";
    if (v < 195 || v > 265) return "ring-2 ring-red-500/40";
    if (v < 207 || v > 253) return "ring-1 ring-amber-500/30";
    return "";
  }

  // --- Current gauge: adaptive scale ---
  const C_COLOR = "#3b82f6";

  function cScale(c: number | undefined) {
    if (!c || c < 6) return {
      min: 0, max: 10,
      ticks: [{ v: 0, label: "0" }, { v: 5, label: "5" }, { v: 10, label: "10" }],
    };
    return {
      min: 0, max: 120,
      ticks: [{ v: 0, label: "0" }, { v: 30, label: "30" }, { v: 60, label: "60" }, { v: 90, label: "90" }, { v: 120, label: "120" }],
    };
  }
</script>

<div class="space-y-6">
  <!-- Header -->
  <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm">
    <div class="flex items-center justify-between">
      <div>
        <h3 class="text-xl font-bold text-slate-900 dark:text-white mb-1">{$t.liveMeasurements}</h3>
        <p class="text-sm text-slate-500 dark:text-slate-400">{$t.liveMeasurementsDescription}</p>
      </div>
      <div class="flex items-center gap-4">
        {#if $meterStore.lastReadTime}
          <div class="text-xs text-slate-400 dark:text-slate-500 text-right hidden sm:block">
            <div class="font-medium uppercase tracking-wider">Son Okuma</div>
            <div class="font-mono text-slate-500 dark:text-slate-400">{fmtTime($meterStore.lastReadTime)}</div>
          </div>
        {/if}
        {#if $isConnected}
          <button
            onclick={refreshMeasurements}
            disabled={isRefreshing}
            class="flex items-center gap-2 px-4 py-2.5 bg-primary hover:bg-primary/90 disabled:opacity-50 disabled:cursor-not-allowed text-white rounded-lg text-sm font-medium transition-colors"
          >
            <Icon name="refresh" size="sm" class={isRefreshing ? "animate-spin" : ""} />
            {$t.refreshMeasurements}
          </button>
        {/if}
      </div>
    </div>
  </div>

  {#if !$isConnected}
    <div class="bg-amber-500/10 border border-amber-500/20 rounded-xl p-6 text-center">
      <Icon name="warning" class="text-4xl text-amber-500 mb-3" />
      <p class="text-amber-600 dark:text-amber-400 font-medium">{$t.connectFirstWarning}</p>
    </div>
  {:else if $meterStore.shortReadData}
    {@const data = $meterStore.shortReadData}
    {@const isThreePhase = $meterStore.meterType === "three-phase"}

    <!-- ===== Voltage Gauge Snippet ===== -->
    {#snippet voltGauge(voltage: number | undefined, freq: number | undefined, label: string, dotColor: string)}
      {@const q = vQuality(voltage)}
      <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-5 shadow-sm transition-shadow {cardRing(voltage)}">
        <div class="text-center mb-1">
          <h4 class="font-semibold text-slate-900 dark:text-white flex items-center justify-center gap-2 text-sm">
            <span class="w-2.5 h-2.5 rounded-full {dotColor}"></span>
            {label}
          </h4>
        </div>

        <div class="mx-auto" style="max-width: 230px;">
          <svg viewBox="0 0 200 128" class="w-full">
            {#each vZones as z}
              <path d={arcD(valToAngle(z.v1, V_MIN, V_MAX), valToAngle(z.v2, V_MIN, V_MAX))} fill="none" stroke={z.color} stroke-width="10" opacity="0.18" stroke-linecap="butt" />
            {/each}
            {#if voltage && voltage > 0}
              <path d={arcD(180, valToAngle(voltage, V_MIN, V_MAX))} fill="none" stroke={vColor(voltage)} stroke-width="12" stroke-linecap="round" class="transition-all duration-700" />
              {@const dot = toXY(valToAngle(voltage, V_MIN, V_MAX))}
              <circle cx={dot.x} cy={dot.y} r="8" fill={vColor(voltage)} opacity="0.15" />
              <circle cx={dot.x} cy={dot.y} r="5" fill="white" stroke={vColor(voltage)} stroke-width="2.5" />
            {/if}
            {#each vTicks as tick}
              {@const inner = toXY(valToAngle(tick.v, V_MIN, V_MAX), R - 14)}
              {@const outer = toXY(valToAngle(tick.v, V_MIN, V_MAX), R + 3)}
              {@const lbl = toXY(valToAngle(tick.v, V_MIN, V_MAX), R - 27)}
              <line x1={inner.x} y1={inner.y} x2={outer.x} y2={outer.y} stroke="currentColor" class="text-slate-300 dark:text-slate-600" stroke-width="1.5" />
              <text x={lbl.x} y={lbl.y} text-anchor="middle" dominant-baseline="middle" font-size="8.5" fill="currentColor" class="text-slate-400 dark:text-slate-500">{tick.label}</text>
            {/each}
            <text x={CX} y={CY - 20} text-anchor="middle" dominant-baseline="middle" font-size="30" font-weight="bold" font-family="ui-monospace, monospace" fill="currentColor" class="text-slate-900 dark:text-white">
              {fmt(voltage)}
            </text>
            <text x={CX} y={CY + 2} text-anchor="middle" font-size="11" fill="currentColor" class="text-slate-400 dark:text-slate-500">Volt</text>
          </svg>
        </div>

        <!-- Quality badge -->
        <div class="text-center mb-2 -mt-1">
          <span class="inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-bold border {q.text} {q.bg} {q.pulse ? 'animate-pulse border-red-500/30' : q.label === 'Normal' ? 'border-emerald-500/20' : 'border-amber-500/20'}">
            <span class="w-1.5 h-1.5 rounded-full {q.dot}"></span>
            {q.label}
          </span>
        </div>

        <!-- Frequency -->
        <div class="flex items-center justify-center gap-2 p-2.5 bg-slate-50 dark:bg-[#0f1821] rounded-lg">
          <Icon name="speed" size="sm" class="text-slate-400" />
          <span class="text-[10px] uppercase tracking-wider text-slate-400">{$t.frequency}</span>
          <span class="text-sm font-mono font-bold text-primary">{fmt(freq, 2)} Hz</span>
        </div>
      </div>
    {/snippet}

    <!-- ===== Current Gauge Snippet ===== -->
    {#snippet currGauge(current: number | undefined, pf: number | undefined, label: string, dotColor: string)}
      {@const sc = cScale(current)}
      <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-5 shadow-sm">
        <div class="text-center mb-1">
          <h4 class="font-semibold text-slate-900 dark:text-white flex items-center justify-center gap-2 text-sm">
            <span class="w-2.5 h-2.5 rounded-full {dotColor}"></span>
            {label}
          </h4>
        </div>

        <div class="mx-auto" style="max-width: 230px;">
          <svg viewBox="0 0 200 128" class="w-full">
            <!-- Background arc -->
            <path d={arcD(180, 0)} fill="none" stroke={C_COLOR} stroke-width="10" opacity="0.12" stroke-linecap="butt" />
            <!-- Active arc -->
            {#if current && current > 0}
              {@const clamped = Math.min(current, sc.max)}
              <path d={arcD(180, valToAngle(clamped, sc.min, sc.max))} fill="none" stroke={C_COLOR} stroke-width="12" stroke-linecap="round" class="transition-all duration-700" />
              {@const dot = toXY(valToAngle(clamped, sc.min, sc.max))}
              <circle cx={dot.x} cy={dot.y} r="8" fill={C_COLOR} opacity="0.15" />
              <circle cx={dot.x} cy={dot.y} r="5" fill="white" stroke={C_COLOR} stroke-width="2.5" />
            {/if}
            <!-- Ticks -->
            {#each sc.ticks as tick}
              {@const inner = toXY(valToAngle(tick.v, sc.min, sc.max), R - 14)}
              {@const outer = toXY(valToAngle(tick.v, sc.min, sc.max), R + 3)}
              {@const lbl = toXY(valToAngle(tick.v, sc.min, sc.max), R - 27)}
              <line x1={inner.x} y1={inner.y} x2={outer.x} y2={outer.y} stroke="currentColor" class="text-slate-300 dark:text-slate-600" stroke-width="1.5" />
              <text x={lbl.x} y={lbl.y} text-anchor="middle" dominant-baseline="middle" font-size="8.5" fill="currentColor" class="text-slate-400 dark:text-slate-500">{tick.label}</text>
            {/each}
            <!-- Value -->
            <text x={CX} y={CY - 20} text-anchor="middle" dominant-baseline="middle" font-size="30" font-weight="bold" font-family="ui-monospace, monospace" fill="currentColor" class="text-slate-900 dark:text-white">
              {fmt(current, 2)}
            </text>
            <text x={CX} y={CY + 2} text-anchor="middle" font-size="11" fill="currentColor" class="text-slate-400 dark:text-slate-500">Amper</text>
          </svg>
        </div>

        <!-- Spacer to align with voltage card's quality badge -->
        <div class="mb-2 -mt-1 h-[26px]"></div>

        <!-- Cos φ -->
        <div class="flex items-center justify-center gap-2 p-2.5 bg-slate-50 dark:bg-[#0f1821] rounded-lg">
          <span class="text-[10px] uppercase tracking-wider text-slate-400">Cos &#966;</span>
          <span class="text-sm font-mono font-bold text-slate-900 dark:text-white">{fmt(pf, 3)}</span>
        </div>
      </div>
    {/snippet}

    <!-- Row 1: Voltage Gauges -->
    <div class="grid grid-cols-1 {isThreePhase ? 'md:grid-cols-3' : 'md:grid-cols-2'} gap-6">
      {@render voltGauge(data.voltageL1, data.frequency, isThreePhase ? `${$t.phaseR} (L1)` : $t.voltageL1, "bg-red-500")}
      {#if isThreePhase}
        {@render voltGauge(data.voltageL2, data.frequency, `${$t.phaseS} (L2)`, "bg-yellow-500")}
        {@render voltGauge(data.voltageL3, data.frequency, `${$t.phaseT} (L3)`, "bg-blue-500")}
      {:else}
        <!-- Single-phase: current gauge beside voltage -->
        {@render currGauge(data.currentL1, data.powerFactorL1, $t.current, "bg-red-500")}
      {/if}
    </div>

    <!-- Row 2: Current Gauges (3-phase only) -->
    {#if isThreePhase}
      <div class="grid grid-cols-1 md:grid-cols-3 gap-6">
        {@render currGauge(data.currentL1, data.powerFactorL1, `${$t.current} R (L1)`, "bg-red-500")}
        {@render currGauge(data.currentL2, data.powerFactorL2, `${$t.current} S (L2)`, "bg-yellow-500")}
        {@render currGauge(data.currentL3, data.powerFactorL3, `${$t.current} T (L3)`, "bg-blue-500")}
      </div>
    {/if}
  {:else}
    <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-12 shadow-sm text-center">
      <Icon name="speed" class="text-6xl text-slate-300 dark:text-slate-600 mb-4" />
      <p class="text-slate-500 dark:text-slate-400">Veri yok — Bağlantı sayfasından okuma yapın</p>
    </div>
  {/if}
</div>
