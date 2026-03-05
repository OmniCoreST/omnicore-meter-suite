<script lang="ts">
  import { onMount } from "svelte";
  import { t } from "$lib/stores";
  import { complianceStore } from "$lib/stores/compliance";
  import { meterStore } from "$lib/stores/meter";
  import { connectionStore } from "$lib/stores/connection";
  import {
    checkCompliance,
    readFull,
    readObisBatch,
    getComplianceRulesPath,
    reloadComplianceRules,
    updateComplianceRules,
    importComplianceRulesFromFile,
    addComplianceRule,
    listComplianceRules,
    updateComplianceRule,
    deleteComplianceRule,
    isTauri,
    type ComplianceRuleDef,
  } from "$lib/utils/tauri";
  import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
  import { warningToast, successToast, errorToast } from "$lib/stores/toast";

  let reloading = $state(false);
  let updating = $state(false);
  let showUpdateModal = $state(false);
  let fetchingData = $state(false);

  // OBIS yağmuru canvas animasyonu
  let rainCanvas = $state<HTMLCanvasElement | null>(null);
  let rainAnimFrame: number | null = null;

  const OBIS_CODES = [
    "1.8.0","1.8.1","1.8.2","1.8.3","1.8.4",
    "2.8.0","2.8.1","2.8.2","3.8.0","4.8.0",
    "5.8.0","6.8.0","7.8.0","8.8.0",
    "32.7.0","52.7.0","72.7.0",
    "31.7.0","51.7.0","71.7.0",
    "13.7.0","33.7.0","53.7.0","73.7.0",
    "14.7.0","0.9.1","0.9.2","0.0.0",
    "96.5.0","96.5.1","96.5.4","96.5.5",
    "0.8.0","0.8.1","0.8.2","0.8.3","0.8.4",
    "15.8.0","16.8.0","128.8.0",
  ];

  function startRain(canvas: HTMLCanvasElement) {
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const resize = () => {
      canvas.width = canvas.offsetWidth;
      canvas.height = canvas.offsetHeight;
    };
    resize();
    const ro = new ResizeObserver(resize);
    ro.observe(canvas);

    const rand = (a: number, b: number) => a + Math.random() * (b - a);
    const randCode = () => OBIS_CODES[Math.floor(Math.random() * OBIS_CODES.length)];

    const PARTICLE_COUNT = 38;
    const START_DELAY_MS = 1200; // ilk N ms'de hiç kod gösterme
    let startTime = performance.now();

    type Particle = {
      x: number; y: number;
      code: string;
      fontSize: number;
      alpha: number;
      phase: "in" | "hold" | "out";
      phaseT: number;
      inDur: number;
      holdDur: number;
      outDur: number;
    };

    const makeParticle = (fresh = true): Particle => ({
      x:        rand(40, canvas.width  - 40),
      y:        rand(30, canvas.height - 30),
      code:     randCode(),
      fontSize: rand(10, 16),
      alpha:    0,
      phase:    "in",
      phaseT:   0,
      inDur:    rand(20, 50),
      holdDur:  rand(40, 120),
      outDur:   rand(30, 70),
    });

    // hepsi gizli başlıyor — delay sonrası birer birer açılacak
    const particles: Particle[] = Array.from({ length: PARTICLE_COUNT }, () => makeParticle());

    const tick = () => {
      ctx.clearRect(0, 0, canvas.width, canvas.height);

      const elapsed = performance.now() - startTime;
      if (elapsed < START_DELAY_MS) {
        rainAnimFrame = requestAnimationFrame(tick);
        return;
      }

      ctx.textAlign = "center";

      for (const p of particles) {
        p.phaseT++;

        if (p.phase === "in") {
          p.alpha = p.phaseT / p.inDur;
          if (p.phaseT >= p.inDur) { p.phase = "hold"; p.phaseT = 0; }
        } else if (p.phase === "hold") {
          p.alpha = 1;
          if (p.phaseT >= p.holdDur) { p.phase = "out"; p.phaseT = 0; }
        } else {
          p.alpha = 1 - p.phaseT / p.outDur;
          if (p.phaseT >= p.outDur) {
            Object.assign(p, makeParticle());
          }
        }

        const a = Math.max(0, Math.min(1, p.alpha));
        ctx.shadowColor = `rgba(39,158,167,${a * 0.8})`;
        ctx.shadowBlur  = 8 + a * 10;
        const r = Math.round(39  + (200 - 39)  * a);
        const g = Math.round(158 + (255 - 158) * a);
        const bv= Math.round(167 + (255 - 167) * a);
        ctx.fillStyle = `rgba(${r},${g},${bv},${a * 0.9})`;
        ctx.font      = `bold ${p.fontSize}px "Oxanium", monospace`;
        ctx.fillText(p.code, p.x, p.y);
      }

      ctx.shadowBlur = 0;
      rainAnimFrame = requestAnimationFrame(tick);
    };
    tick();

    return () => { ro.disconnect(); };
  }

  $effect(() => {
    if (fetchingData && rainCanvas) {
      const cleanup = startRain(rainCanvas);
      return () => {
        if (rainAnimFrame !== null) { cancelAnimationFrame(rainAnimFrame); rainAnimFrame = null; }
        cleanup?.();
      };
    }
  });

  // Faz sayısını otomatik tespit et
  // Öncelik: meterStore.meterType → anlık veri alanları → model adı → localStorage fallback
  const meterPhases: 1 | 3 = $derived.by(() => {
    const mt = $meterStore.meterType;
    if (mt === "three-phase" || mt === "kombi") return 3;
    if (mt === "single-phase") return 1;

    const data = $meterStore.shortReadData ?? $meterStore.fullReadData;
    if (data?.voltageL2 !== undefined || data?.currentL2 !== undefined) return 3;

    const model = $connectionStore.meterIdentity?.model ?? "";
    if (/3[Pp]|TP|MT\d|3PH/i.test(model)) return 3;

    const stored = parseInt(localStorage.getItem("compliance_meter_phases") ?? "0");
    if (stored === 1 || stored === 3) return stored as 1 | 3;

    return 3;
  });

  const currentData = $derived(
    $meterStore.shortReadData ?? $meterStore.fullReadData
  );
  const result = $derived($complianceStore.result);
  const loading = $derived($complianceStore.loading);

  /**
   * readFull(), F.F.0'ı bir kez okuyarak latch'i temizler.
   * F.F.0 "read-and-clear" latch'tir: ikinci okuma gerçek anlık durumu verir.
   * Bu fonksiyon readFull sonrası bir kez daha F.F.0 ve F.F.1 okuyarak
   * raw verisini günceller — böylece klemens kapağı gibi anlık durumlar doğru tespit edilir.
   */
  async function readFullWithFreshFF() {
    const raw = await readFull() as any;
    try {
      const fresh = await readObisBatch(["F.F.0", "F.F.1"]);
      if (fresh["F.F.0"]) raw.ffCode = fresh["F.F.0"];
      if (fresh["F.F.1"]) raw.gfCode = fresh["F.F.1"];
    } catch {
      // Ek okuma başarısız olursa readFull sonucu kullanılmaya devam eder
    }
    return raw;
  }

  onMount(async () => {
    // İlk girişte (sonuç yoksa) tam okuma yap; sonrasında kullanıcı isteğe bağlı okur.
    if ($connectionStore.status === "connected" && isTauri() && !$complianceStore.result) {
      fetchingData = true;
      try {
        const raw = await readFullWithFreshFF();
        const rawStr = raw.rawData || "";
        let meterType: "single-phase" | "three-phase" | "kombi" = "single-phase";
        if (rawStr.includes("52.7.0") || rawStr.includes("72.7.0")) {
          meterType = "three-phase";
        }
        meterStore.setShortReadData(raw, meterType, false);
        complianceStore.setLoading();
        const res = await checkCompliance(raw as any, meterPhases);
        complianceStore.setResult(res);
      } catch (e) {
        errorToast(String(e));
      } finally {
        fetchingData = false;
      }
    }
  });

  const sortedIssues = $derived(
    result?.issues.slice().sort((a, b) => {
      const order = { error: 0, warning: 1, info: 2 };
      return (order[a.severity as keyof typeof order] ?? 3) -
             (order[b.severity as keyof typeof order] ?? 3);
    }) ?? []
  );

  async function refetchAndCheck() {
    if ($connectionStore.status !== "connected" || !isTauri()) return;
    fetchingData = true;
    try {
      const raw = await readFullWithFreshFF();
      const rawStr = raw.rawData || "";
      let meterType: "single-phase" | "three-phase" | "kombi" = "single-phase";
      if (rawStr.includes("52.7.0") || rawStr.includes("72.7.0")) {
        meterType = "three-phase";
      }
      meterStore.setShortReadData(raw, meterType, false);
      complianceStore.setLoading();
      const res = await checkCompliance(raw as any, meterPhases);
      complianceStore.setResult(res);
    } catch (e) {
      errorToast(String(e));
    } finally {
      fetchingData = false;
    }
  }

  async function runCheck() {
    if (!currentData) return;
    complianceStore.setLoading();
    try {
      const res = await checkCompliance(currentData as any, meterPhases);
      complianceStore.setResult(res);
      if (res.rulesStatus === "tooOld") {
        errorToast($t.complianceTooOld);
      } else if (res.errorCount === 0 && res.warningCount === 0) {
        successToast($t.complianceAllPassed);
      } else {
        const parts: string[] = [];
        if (res.errorCount > 0) parts.push(`${res.errorCount} ${$t.complianceErrors}`);
        if (res.warningCount > 0) parts.push(`${res.warningCount} ${$t.complianceWarnings}`);
        warningToast(parts.join(", "));
      }
    } catch (e) {
      complianceStore.setError(String(e));
      errorToast(String(e));
    }
  }

  async function openRulesFile() {
    if (!isTauri()) return;
    try {
      const path = await getComplianceRulesPath();
      const { openPath } = await import("@tauri-apps/plugin-opener");
      await openPath(path);
    } catch (e) {
      errorToast(String(e));
    }
  }

  async function reloadRules() {
    reloading = true;
    try {
      const msg = await reloadComplianceRules();
      successToast(msg);
      if (currentData) await runCheck();
    } catch (e) {
      errorToast(String(e));
    } finally {
      reloading = false;
    }
  }

  async function updateRulesFromInternet() {
    showUpdateModal = false;
    updating = true;
    try {
      const msg = await updateComplianceRules();
      successToast(msg);
      if (currentData) await runCheck();
    } catch (e) {
      errorToast(String(e));
    } finally {
      updating = false;
    }
  }

  async function updateRulesFromFile() {
    showUpdateModal = false;
    const selected = await openFileDialog({
      filters: [{ name: "TOML", extensions: ["toml"] }],
      multiple: false,
    });
    if (!selected) return;
    const path = typeof selected === "string" ? selected : selected.path;
    updating = true;
    try {
      const msg = await importComplianceRulesFromFile(path);
      successToast(msg);
      if (currentData) await runCheck();
    } catch (e) {
      errorToast(String(e));
    } finally {
      updating = false;
    }
  }

  function formatTime(iso: string): string {
    try {
      return new Date(iso).toLocaleTimeString("tr-TR", {
        hour: "2-digit", minute: "2-digit", second: "2-digit",
      });
    } catch { return iso; }
  }

  function severityIcon(s: string) {
    return s === "error" ? "cancel" : s === "warning" ? "warning" : "info";
  }
  function severityText(s: string) {
    return s === "error" ? "text-red-500" : s === "warning" ? "text-yellow-500" : "text-blue-400";
  }
  function severityBorder(s: string) {
    return s === "error"
      ? "bg-red-500/10 border-red-500/20"
      : s === "warning"
      ? "bg-yellow-500/10 border-yellow-500/20"
      : "bg-blue-500/10 border-blue-500/20";
  }

  /** "TEDAŞ §5.3 Ek-F / MASS §5.3 Ek-C" → ["TEDAŞ §5.3 Ek-F", "MASS §5.3 Ek-C"] */
  function specSources(specRef: string): string[] {
    return specRef.split(" / ").map(part =>
      part.trim().replace(/^TEDAS(\s|§)/, "TEDAŞ$1")
    );
  }

  /** FF/GF bit kontrolleri için true döner — bu kurallarda ölçülen/beklenen değer anlamlı değil */
  function isBitCheck(field: string): boolean {
    return field === "ff_code" || field === "gf_code";
  }

  // ─── Kural Ekleme Formu ────────────────────────────────────────────────────

  // "date"       → sadece not_empty + not_equals (geçersiz tarih kontrolü)
  // "datetime"   → sadece not_empty (meter_date/time gibi sürekli değişenler)
  // "enum"       → equals / not_equals ama hazır seçenekle + not_empty
  type FieldType = "numeric" | "text" | "date" | "datetime" | "enum" | "bitfield" | "tariff_sum" | "time_drift";

  interface FieldDef {
    value: string;
    label: string;
    type: FieldType;
    phases?: number;
    /** "enum" alanları için hazır seçenekler */
    options?: { value: string; label: string }[];
  }

  interface FieldGroup {
    label: string;
    fields: FieldDef[];
  }

  const FIELD_GROUPS: FieldGroup[] = [
    {
      label: "Elektriksel Parametreler",
      fields: [
        { value: "voltage_l1", label: "L1 (R Faz) Gerilimi", type: "numeric" },
        { value: "voltage_l2", label: "L2 (S Faz) Gerilimi", type: "numeric", phases: 3 },
        { value: "voltage_l3", label: "L3 (T Faz) Gerilimi", type: "numeric", phases: 3 },
        { value: "current_l1", label: "L1 Akımı", type: "numeric" },
        { value: "current_l2", label: "L2 Akımı", type: "numeric", phases: 3 },
        { value: "current_l3", label: "L3 Akımı", type: "numeric", phases: 3 },
        { value: "frequency", label: "Frekans", type: "numeric" },
        { value: "power_factor_l1", label: "L1 Güç Faktörü", type: "numeric" },
        { value: "power_factor_l2", label: "L2 Güç Faktörü", type: "numeric", phases: 3 },
        { value: "power_factor_l3", label: "L3 Güç Faktörü", type: "numeric", phases: 3 },
      ],
    },
    {
      label: "Kimlik Bilgileri",
      fields: [
        { value: "serial_number", label: "Seri Numarası", type: "text" },
        { value: "program_version", label: "Yazılım Versiyonu", type: "text" },
        { value: "production_date", label: "Üretim Tarihi", type: "date" },
        { value: "calibration_date", label: "Kalibrasyon Tarihi", type: "date" },
      ],
    },
    {
      label: "Enerji",
      fields: [
        { value: "active_energy_import_total", label: "Toplam Aktif Enerji", type: "numeric" },
        { value: "active_energy_import_t1", label: "T1 Enerjisi", type: "numeric" },
        { value: "active_energy_import_t2", label: "T2 Enerjisi", type: "numeric" },
        { value: "active_energy_import_t3", label: "T3 Enerjisi", type: "numeric" },
        { value: "active_energy_import_t4", label: "T4 Enerjisi", type: "numeric" },
      ],
    },
    {
      label: "Durum Bayrakları",
      fields: [
        { value: "ff_code", label: "FF Durum Kodu", type: "bitfield" },
        { value: "gf_code", label: "GF Durum Kodu", type: "bitfield" },
        {
          value: "battery_status", label: "Batarya Durumu", type: "enum",
          options: [
            { value: "full", label: "Dolu (full)" },
            { value: "low",  label: "Zayıf (low)" },
          ],
        },
        {
          value: "relay_status", label: "Röle Durumu", type: "enum",
          options: [
            { value: "active",  label: "Aktif / Bağlı (active)" },
            { value: "passive", label: "Pasif / Kesik (passive)" },
          ],
        },
      ],
    },
    {
      label: "Ayarlar",
      fields: [
        { value: "demand_period", label: "Talep Periyodu", type: "text" },
        { value: "lp_period", label: "Yük Profili Periyodu", type: "text" },
      ],
    },
    {
      label: "Zaman",
      fields: [
        { value: "meter_date", label: "Sayaç Tarihi", type: "datetime" },
        { value: "meter_time", label: "Sayaç Saati", type: "datetime" },
      ],
    },
    {
      label: "Özel",
      fields: [
        { value: "tariff_sum", label: "Tarife Dengesi (T1+T2+T3+T4)", type: "tariff_sum" },
        { value: "time_drift", label: "Saat Sapması", type: "time_drift" },
      ],
    },
  ];

  function getFieldDef(fieldValue: string): FieldDef | undefined {
    for (const group of FIELD_GROUPS) {
      const f = group.fields.find(fd => fd.value === fieldValue);
      if (f) return f;
    }
    return undefined;
  }

  function getChecksForField(fieldValue: string): { value: string; label: string }[] {
    const fd = getFieldDef(fieldValue);
    if (!fd) return [];
    switch (fd.type) {
      case "bitfield":
        return [
          { value: "bit_zero", label: $t.complianceCheckBitZero },
          { value: "bit_one", label: $t.complianceCheckBitOne },
        ];
      case "tariff_sum":
        return [{ value: "tariff_balance", label: $t.complianceCheckTariffBalance }];
      case "time_drift":
        return [{ value: "time_drift_minutes", label: $t.complianceCheckTimeDrift }];
      case "datetime":
        // Sürekli değişen alanlar — sadece "boş değil" mantıklı
        return [{ value: "not_empty", label: $t.complianceCheckNotEmpty }];
      case "date":
        // Üretim/kalibrasyon tarihi — boşluk + geçersiz tarih kontrolü
        return [
          { value: "not_empty", label: $t.complianceCheckNotEmpty },
          { value: "not_equals", label: `${$t.complianceCheckNotEquals} (ör. geçersiz tarih)` },
        ];
      case "enum":
        // Hazır değerli alanlar
        return [
          { value: "equals", label: $t.complianceCheckEquals },
          { value: "not_equals", label: $t.complianceCheckNotEquals },
          { value: "not_empty", label: $t.complianceCheckNotEmpty },
        ];
      case "numeric":
        return [
          { value: "range", label: $t.complianceCheckRange },
          { value: "equals", label: $t.complianceCheckEquals },
          { value: "not_equals", label: $t.complianceCheckNotEquals },
          { value: "not_empty", label: $t.complianceCheckNotEmpty },
        ];
      case "text":
      default:
        return [
          { value: "equals", label: $t.complianceCheckEquals },
          { value: "not_equals", label: $t.complianceCheckNotEquals },
          { value: "not_empty", label: $t.complianceCheckNotEmpty },
        ];
    }
  }

  // ─── Kural Ekle/Düzenle Modal ────────────────────────────────────────────────
  let showAddRule = $state(false);
  let editMode = $state(false); // true = mevcut kuralı düzenle
  let saving = $state(false);

  let ruleCode = $state("");
  let ruleField = $state("voltage_l1");
  let ruleCheck = $state("range");
  let ruleSeverity = $state<"error" | "warning" | "info">("warning");
  let rulePhases = $state<0 | 1 | 3>(0);
  let ruleDesc = $state("");
  let ruleSpecRef = $state("");
  let ruleMin = $state("");
  let ruleMax = $state("");
  let ruleValue = $state("");
  let ruleBit = $state("");
  let ruleTolerance = $state("");
  let ruleMaxDrift = $state("");

  const availableChecks = $derived(getChecksForField(ruleField));

  $effect(() => {
    const checks = getChecksForField(ruleField);
    if (checks.length > 0 && !checks.find(c => c.value === ruleCheck)) {
      ruleCheck = checks[0].value;
    }
  });

  function openAddRuleModal(prefill?: ComplianceRuleDef) {
    editMode = !!prefill;
    ruleCode = prefill?.code ?? "";
    ruleField = prefill?.field ?? "voltage_l1";
    ruleCheck = prefill?.check ?? "range";
    ruleSeverity = (prefill?.severity as "error"|"warning"|"info") ?? "warning";
    rulePhases = (prefill?.phases as 0|1|3) ?? 0;
    ruleDesc = prefill?.description ?? "";
    ruleSpecRef = prefill?.spec_ref ?? "";
    ruleMin = prefill?.min != null ? String(prefill.min) : "";
    ruleMax = prefill?.max != null ? String(prefill.max) : "";
    ruleValue = prefill?.value ?? "";
    ruleBit = prefill?.bit != null ? String(prefill.bit) : "";
    ruleTolerance = prefill?.tolerance != null ? String(prefill.tolerance) : "";
    ruleMaxDrift = prefill?.max_drift != null ? String(prefill.max_drift) : "";
    showAddRule = true;
  }

  async function saveRule() {
    if (!ruleCode.trim() || !ruleDesc.trim()) return;
    saving = true;
    try {
      if (editMode) {
        // Struct tabanlı güncelleme
        const rule: ComplianceRuleDef = {
          code: ruleCode.trim(),
          field: ruleField,
          check: ruleCheck,
          severity: ruleSeverity,
          description: ruleDesc.trim(),
          spec_ref: ruleSpecRef.trim() || null,
          phases: rulePhases !== 0 ? rulePhases : null,
          min: ruleCheck === "range" ? parseFloat(ruleMin) : null,
          max: ruleCheck === "range" ? parseFloat(ruleMax) : null,
          value: (ruleCheck === "equals" || ruleCheck === "not_equals") ? ruleValue : null,
          bit: (ruleCheck === "bit_zero" || ruleCheck === "bit_one") ? parseInt(ruleBit) : null,
          tolerance: ruleCheck === "tariff_balance" ? parseFloat(ruleTolerance) : null,
          max_drift: ruleCheck === "time_drift_minutes" ? parseInt(ruleMaxDrift) : null,
        };
        await updateComplianceRule(rule);
        successToast($t.complianceRuleUpdated);
        await loadManagedRules();
      } else {
        // TOML append (yeni kural)
        let toml = `[[rules]]\n`;
        toml += `code = "${ruleCode.trim()}"\n`;
        toml += `field = "${ruleField}"\n`;
        toml += `check = "${ruleCheck}"\n`;
        toml += `severity = "${ruleSeverity}"\n`;
        toml += `description = "${ruleDesc.trim()}"\n`;
        if (ruleSpecRef.trim()) toml += `spec_ref = "${ruleSpecRef.trim()}"\n`;
        if (rulePhases !== 0) toml += `phases = ${rulePhases}\n`;
        if (ruleCheck === "range") {
          toml += `min = ${ruleMin}\nmax = ${ruleMax}\n`;
        } else if (ruleCheck === "equals" || ruleCheck === "not_equals") {
          toml += `value = "${ruleValue}"\n`;
        } else if (ruleCheck === "bit_zero" || ruleCheck === "bit_one") {
          toml += `bit = ${ruleBit}\n`;
        } else if (ruleCheck === "tariff_balance") {
          toml += `tolerance = ${ruleTolerance}\n`;
        } else if (ruleCheck === "time_drift_minutes") {
          toml += `max_drift = ${ruleMaxDrift}\n`;
        }
        await addComplianceRule(toml);
        successToast($t.complianceRuleAdded);
        await loadManagedRules();
      }
      await reloadComplianceRules();
      showAddRule = false;
    } catch (e) {
      errorToast(String(e));
    } finally {
      saving = false;
    }
  }

  // ─── Kuralları Yönet Modal ────────────────────────────────────────────────────
  let showManageRules = $state(false);
  let managedRules = $state<ComplianceRuleDef[]>([]);
  let loadingRules = $state(false);
  let deletingCode = $state<string | null>(null);

  async function loadManagedRules() {
    loadingRules = true;
    try {
      managedRules = await listComplianceRules();
    } catch (e) {
      errorToast(String(e));
    } finally {
      loadingRules = false;
    }
  }

  async function openManageRules() {
    showManageRules = true;
    await loadManagedRules();
  }

  async function confirmDelete(code: string) {
    if (!confirm($t.complianceDeleteConfirm + ` (${code})`)) return;
    deletingCode = code;
    try {
      await deleteComplianceRule(code);
      successToast($t.complianceRuleDeleted);
      await loadManagedRules();
      await reloadComplianceRules();
    } catch (e) {
      errorToast(String(e));
    } finally {
      deletingCode = null;
    }
  }

  function severityDot(s: string) {
    return s === "error" ? "bg-red-500" : s === "warning" ? "bg-yellow-500" : "bg-blue-400";
  }

</script>

<div class="space-y-6">

  <!-- Tam okuma bekleme ekranı (tam sayfa overlay) -->
  {#if fetchingData}
    <div class="fixed inset-0 z-50 flex flex-col items-center justify-center bg-[#0a1520] overflow-hidden">

      <!-- Arka plan nabzı — radar/uyarı ışığı -->
      <div class="absolute inset-0 obis-bg-pulse pointer-events-none"
           style="background: radial-gradient(ellipse 70% 60% at 50% 50%, rgba(39,158,167,0.22) 0%, rgba(39,158,167,0.06) 50%, transparent 100%);"></div>

      <!-- OBIS parçacık canvas -->
      <canvas
        bind:this={rainCanvas}
        class="absolute inset-0 w-full h-full pointer-events-none"
      ></canvas>

      <!-- Merkez kart -->
      <div class="relative z-10 flex flex-col items-center gap-7 px-12 py-10 text-center
                  rounded-2xl border border-primary/25 bg-[#0a1520]/80 backdrop-blur-lg
                  shadow-[0_0_60px_rgba(39,158,167,0.15)]">

        <!-- Halka + ikon -->
        <div class="relative flex items-center justify-center w-28 h-28">
          <!-- dış halka ping -->
          <span class="animate-ping absolute inset-0 rounded-full bg-primary/10"></span>
          <!-- dönen yay (SVG) -->
          <svg class="absolute inset-0 w-full h-full animate-spin-reverse" style="animation-duration:2s" viewBox="0 0 112 112">
            <circle cx="56" cy="56" r="50" fill="none" stroke="rgba(39,158,167,0.15)" stroke-width="2"/>
            <circle cx="56" cy="56" r="50" fill="none" stroke="rgba(39,158,167,0.7)" stroke-width="2"
              stroke-dasharray="80 235" stroke-linecap="round"/>
          </svg>
          <!-- iç daire + ikon -->
          <span class="relative flex items-center justify-center w-16 h-16 rounded-full
                       bg-primary/10 border border-primary/30 shadow-[0_0_20px_rgba(39,158,167,0.3)]">
            <span class="material-symbols-outlined text-4xl text-primary">electric_meter</span>
          </span>
        </div>

        <!-- Yazılar -->
        <div class="space-y-1.5">
          <p class="text-xl font-bold text-white tracking-wider">Okuma yapılıyor</p>
          <div class="flex items-center justify-center gap-1">
            <span class="w-1.5 h-1.5 rounded-full bg-primary animate-bounce" style="animation-delay:0ms"></span>
            <span class="w-1.5 h-1.5 rounded-full bg-primary animate-bounce" style="animation-delay:150ms"></span>
            <span class="w-1.5 h-1.5 rounded-full bg-primary animate-bounce" style="animation-delay:300ms"></span>
          </div>
          <p class="text-xs text-primary/60 mt-1">OBIS verileri alınıyor</p>
        </div>

        <!-- Sayaç bilgisi -->
        {#if $connectionStore.meterIdentity}
          <div class="flex items-center gap-2 px-4 py-2 rounded-lg bg-primary/8 border border-primary/20 text-xs text-primary/70">
            <span class="material-symbols-outlined text-sm text-primary/80">tag</span>
            <span>{$connectionStore.meterIdentity.manufacturer} · {$connectionStore.meterIdentity.serialNumber}</span>
          </div>
        {/if}
      </div>
    </div>
  {:else}

  <!-- Başlık -->
  <div class="flex items-start justify-between gap-4 flex-wrap">
    <div>
      <h2 class="text-xl font-bold text-slate-900 dark:text-white">
        TEDAŞ Uyumluluk Kontrolü
      </h2>
      <p class="text-sm text-slate-500 dark:text-slate-400 mt-1">
        {$t.complianceDescription}
      </p>
    </div>

    <div class="flex items-center gap-3">
      <!-- Faz göstergesi (otomatik tespit) -->
      <div class="flex items-center gap-1.5 px-3 py-2 rounded-lg border border-slate-200 dark:border-slate-600 text-xs font-medium text-slate-600 dark:text-slate-300">
        <span class="material-symbols-outlined text-sm text-primary">electric_bolt</span>
        {meterPhases} Faz
        <span class="text-slate-400 dark:text-slate-500">(Otomatik)</span>
      </div>

      <button
        onclick={runCheck}
        disabled={!currentData || loading || fetchingData}
        class="flex items-center gap-2 px-4 py-2 rounded-lg font-medium text-sm transition-all
          {!currentData || loading || fetchingData
            ? 'bg-slate-200 dark:bg-slate-700 text-slate-400 cursor-not-allowed'
            : 'bg-primary text-white hover:bg-primary/90 active:scale-95'}"
      >
        {#if loading}
          <span class="material-symbols-outlined text-base animate-spin-reverse">autorenew</span>
          Kontrol ediliyor...
        {:else}
          <span class="material-symbols-outlined text-base">verified_user</span>
          {$t.complianceCheck}
        {/if}
      </button>

      {#if $connectionStore.status === "connected"}
        <button
          onclick={refetchAndCheck}
          disabled={fetchingData || loading}
          class="flex items-center gap-2 px-4 py-2 rounded-lg font-medium text-sm border transition-all
            {fetchingData || loading
              ? 'border-slate-200 dark:border-slate-700 text-slate-400 cursor-not-allowed'
              : 'border-slate-300 dark:border-slate-600 text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700 active:scale-95'}"
        >
          {#if fetchingData}
            <span class="material-symbols-outlined text-base animate-spin-reverse">autorenew</span>
            {$t.complianceRefetching}
          {:else}
            <span class="material-symbols-outlined text-base">sync</span>
            {$t.complianceRefetch}
          {/if}
        </button>
      {/if}

    </div>
  </div>

  <!-- Veri yok -->
  {#if !currentData}
    <div class="rounded-xl border border-dashed border-slate-300 dark:border-slate-600 p-8 text-center">
      <span class="material-symbols-outlined text-4xl text-slate-400 mb-3 block">sensors_off</span>
      <p class="text-slate-500 dark:text-slate-400 text-sm">{$t.complianceNoData}</p>
    </div>

  {:else if result}

    <!-- Kurallar çok eski → kilitli -->
    {#if result.rulesStatus === "tooOld"}
      <div class="rounded-xl border border-red-500/30 bg-red-500/10 p-6 text-center space-y-3">
        <span class="material-symbols-outlined text-4xl text-red-500 block">block</span>
        <p class="font-bold text-red-500">{$t.complianceTooOld}</p>
        <p class="text-sm text-slate-500 dark:text-slate-400">{$t.complianceTooOldDesc}</p>
        <p class="text-xs text-slate-400">
          Yerel: v{result.rulesVersion}
          {#if result.latestVersion} → Güncel: v{result.latestVersion}{/if}
        </p>
        <button
          onclick={() => showUpdateModal = true}
          disabled={updating}
          class="mx-auto flex items-center gap-2 px-4 py-2 rounded-lg bg-red-500 text-white text-sm font-medium hover:bg-red-600 transition-all disabled:opacity-50"
        >
          <span class="material-symbols-outlined text-base {updating ? 'animate-spin-reverse' : ''}">cloud_download</span>
          {updating ? $t.complianceUpdating : $t.complianceUpdate}
        </button>
      </div>

    {:else}
      <!-- Durum bildirimleri -->
      {#if result.rulesStatus === "offline"}
        <div class="rounded-lg border border-yellow-500/30 bg-yellow-500/10 px-4 py-2 flex items-center gap-2 text-sm text-yellow-600 dark:text-yellow-400">
          <span class="material-symbols-outlined text-base">wifi_off</span>
          {$t.complianceOffline}
        </div>
      {:else if result.latestVersion && result.latestVersion !== result.rulesVersion}
        <div class="rounded-lg border border-blue-500/30 bg-blue-500/10 px-4 py-2 flex items-center gap-2 text-sm text-blue-600 dark:text-blue-400">
          <span class="material-symbols-outlined text-base">update</span>
          {$t.complianceOutdated}: v{result.latestVersion}
          <button onclick={() => showUpdateModal = true} disabled={updating} class="ml-auto text-xs underline disabled:opacity-50">
            {updating ? $t.complianceUpdating : $t.complianceUpdate}
          </button>
        </div>
      {/if}

      <!-- Meta + araçlar -->
      <div class="flex items-center gap-3 flex-wrap">
        <span class="text-xs text-slate-500 dark:text-slate-400">
          <span class="font-medium">{$t.complianceRulesVersion}:</span> {result.rulesVersion}
        </span>
        <span class="text-slate-300 dark:text-slate-600">•</span>
        <span class="text-xs text-slate-500 dark:text-slate-400">
          <span class="font-medium">{$t.complianceLastCheck}:</span> {formatTime(result.checkedAt)}
        </span>
        <div class="ml-auto flex items-center gap-2">
          <button onclick={openRulesFile}
            class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium border border-slate-200 dark:border-slate-600 text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700 transition-all">
            <span class="material-symbols-outlined text-sm">open_in_new</span>
            {$t.complianceOpenRules}
          </button>
          <button onclick={reloadRules} disabled={reloading}
            class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium border border-slate-200 dark:border-slate-600 text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700 transition-all disabled:opacity-50">
            <span class="material-symbols-outlined text-sm {reloading ? 'animate-spin-reverse' : ''}">refresh</span>
            {reloading ? $t.complianceReloading : $t.complianceReload}
          </button>
          <button onclick={() => showUpdateModal = true} disabled={updating}
            class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium border border-slate-200 dark:border-slate-600 text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700 transition-all disabled:opacity-50">
            <span class="material-symbols-outlined text-sm {updating ? 'animate-spin-reverse' : ''}">cloud_download</span>
            {updating ? $t.complianceUpdating : $t.complianceUpdate}
          </button>
        </div>
      </div>

      <!-- Özet -->
      <div class="grid grid-cols-2 gap-3">
        <div class="rounded-xl p-4 border {result.errorCount > 0 ? 'bg-red-500/10 border-red-500/20' : 'bg-slate-50 dark:bg-slate-800/50 border-slate-200 dark:border-slate-700'}">
          <div class="flex items-center gap-3">
            <span class="material-symbols-outlined text-2xl {result.errorCount > 0 ? 'text-red-500' : 'text-slate-300 dark:text-slate-600'}">
              {result.errorCount > 0 ? 'cancel' : 'check_circle'}
            </span>
            <div>
              <p class="text-2xl font-bold {result.errorCount > 0 ? 'text-red-500' : 'text-slate-400'}">
                {result.errorCount}
              </p>
              <p class="text-xs text-slate-500">{$t.complianceErrors}</p>
            </div>
          </div>
        </div>
        <div class="rounded-xl p-4 border {result.warningCount > 0 ? 'bg-yellow-500/10 border-yellow-500/20' : 'bg-slate-50 dark:bg-slate-800/50 border-slate-200 dark:border-slate-700'}">
          <div class="flex items-center gap-3">
            <span class="material-symbols-outlined text-2xl {result.warningCount > 0 ? 'text-yellow-500' : 'text-slate-300 dark:text-slate-600'}">
              {result.warningCount > 0 ? 'warning' : 'check_circle'}
            </span>
            <div>
              <p class="text-2xl font-bold {result.warningCount > 0 ? 'text-yellow-500' : 'text-slate-400'}">
                {result.warningCount}
              </p>
              <p class="text-xs text-slate-500">{$t.complianceWarnings}</p>
            </div>
          </div>
        </div>
      </div>

      <!-- Tüm kurallar geçti -->
      {#if result.issues.length === 0}
        <div class="rounded-xl border border-emerald-500/20 bg-emerald-500/10 p-6 text-center">
          <span class="material-symbols-outlined text-4xl text-emerald-500 mb-2 block">verified</span>
          <p class="font-bold text-emerald-600 dark:text-emerald-400">{$t.complianceAllPassed}</p>
          <p class="text-sm text-slate-500 mt-1">{$t.complianceAllPassedDesc}</p>
        </div>
      {:else}
        <!-- İhlaller listesi -->
        <div class="space-y-2">
          {#each sortedIssues as issue (issue.code)}
            <div class="rounded-xl border overflow-hidden {severityBorder(issue.severity)}">
              <div class="grid grid-cols-2 divide-x divide-slate-200/60 dark:divide-slate-700/60">

                <!-- SOL: kural bilgisi -->
                <div class="p-4 flex items-start gap-3">
                  <span class="material-symbols-outlined text-xl mt-0.5 {severityText(issue.severity)} flex-shrink-0">
                    {severityIcon(issue.severity)}
                  </span>
                  <div class="min-w-0">
                    <!-- Kod + şartname rozetleri -->
                    <div class="flex items-center gap-1.5 mb-1.5 flex-wrap">
                      <span class="font-mono text-xs font-bold {severityText(issue.severity)}">{issue.code}</span>
                      {#if issue.specRef}
                        {#each specSources(issue.specRef) as src}
                          <span class="text-[10px] bg-slate-100 dark:bg-slate-700/60 text-slate-500 dark:text-slate-400 px-1.5 py-0.5 rounded-full font-medium">
                            {src}
                          </span>
                        {/each}
                      {/if}
                    </div>
                    {#if issue.specRef}
                      <p class="text-[10px] text-slate-400 dark:text-slate-500 mb-1">
                        <span class="font-semibold">{issue.specRef}'e göre:</span>
                      </p>
                    {/if}
                    <p class="text-sm font-medium text-slate-700 dark:text-slate-200 leading-relaxed">
                      {issue.description}
                    </p>
                  </div>
                </div>

                <!-- SAĞ: neden & düzeltme -->
                <div class="p-4 flex flex-col gap-3 bg-black/[0.02] dark:bg-white/[0.02]">
                  {#if issue.cause}
                    <div>
                      <p class="text-[10px] font-semibold text-slate-400 dark:text-slate-500 uppercase tracking-wider mb-1 flex items-center gap-1">
                        <span class="material-symbols-outlined text-xs">help</span>
                        Neden
                      </p>
                      <p class="text-xs text-slate-600 dark:text-slate-300 leading-relaxed">{issue.cause}</p>
                    </div>
                  {/if}
                  {#if issue.remedy}
                    <div class="border-t border-slate-200/60 dark:border-slate-700/60 pt-3">
                      <p class="text-[10px] font-semibold text-slate-400 dark:text-slate-500 uppercase tracking-wider mb-1 flex items-center gap-1">
                        <span class="material-symbols-outlined text-xs">build</span>
                        Düzeltme
                      </p>
                      <p class="text-xs text-slate-600 dark:text-slate-300 leading-relaxed">{issue.remedy}</p>
                    </div>
                  {:else if !issue.cause}
                    <p class="text-xs text-slate-400 dark:text-slate-500 italic">Bu kural için neden/düzeltme bilgisi tanımlanmamış.</p>
                  {/if}
                </div>

              </div>
            </div>
          {/each}
        </div>

        <!-- Rapor notu (Excel çıktısında da yer alacak) -->
        <p class="text-xs text-slate-400 dark:text-slate-500 text-right">
          {$t.complianceReportNote} v{result.rulesVersion}
          {#if result.latestVersion && result.latestVersion !== result.rulesVersion}
            · {$t.complianceOutdated}: v{result.latestVersion}
          {/if}
        </p>
      {/if}
    {/if}

  {:else if !loading}
    <!-- Henüz kontrol yapılmadı -->
    <div class="rounded-xl border border-dashed border-slate-300 dark:border-slate-600 p-8 text-center space-y-3">
      <span class="material-symbols-outlined text-4xl text-slate-400 block">policy</span>
      <p class="text-slate-500 dark:text-slate-400 text-sm">
        Uyumluluk kontrolü için "Kontrol Et" butonuna tıklayın.
      </p>
      <div class="flex items-center justify-center gap-2">
        <button onclick={openRulesFile}
          class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium border border-slate-200 dark:border-slate-600 text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700 transition-all">
          <span class="material-symbols-outlined text-sm">open_in_new</span>
          {$t.complianceOpenRules}
        </button>
      </div>
    </div>
  {/if}

  {/if}
</div>

<!-- ─── Yeni Kural Ekle Modalı ─────────────────────────────────────────────── -->
{#if showAddRule}
  <!-- Backdrop -->
  <div
    role="presentation"
    class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm"
    onclick={(e) => { if (e.target === e.currentTarget) showAddRule = false; }}
    onkeydown={(e) => { if (e.key === 'Escape') showAddRule = false; }}
  >
    <!-- Modal panel -->
    <div class="w-full max-w-md bg-white dark:bg-slate-800 rounded-2xl shadow-2xl border border-slate-200 dark:border-slate-700 overflow-hidden">
      <!-- Header -->
      <div class="flex items-center justify-between px-5 py-4 border-b border-slate-200 dark:border-slate-700">
        <div class="flex items-center gap-2">
          <span class="material-symbols-outlined text-primary text-xl">{editMode ? 'edit' : 'add_circle'}</span>
          <h3 class="font-bold text-slate-900 dark:text-white">
            {editMode ? $t.complianceEditRule + ': ' + ruleCode : $t.complianceAddRuleTitle}
          </h3>
        </div>
        <button
          onclick={() => showAddRule = false}
          class="text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 transition-colors"
        >
          <span class="material-symbols-outlined">close</span>
        </button>
      </div>

      <!-- Body -->
      <div class="px-5 py-4 space-y-4 max-h-[70vh] overflow-y-auto">
        <!-- Kural Kodu + Şiddet -->
        <div class="grid grid-cols-2 gap-3">
          <div>
            <label for="rule-code" class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1">
              {$t.complianceRuleCode} *
            </label>
            <input
              id="rule-code"
              type="text"
              bind:value={ruleCode}
              placeholder="EL-005"
              class="w-full px-3 py-2 text-sm rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-700 text-slate-900 dark:text-white placeholder-slate-400 focus:outline-none focus:ring-2 focus:ring-primary/50"
            />
          </div>
          <div>
            <label for="rule-severity" class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1">
              {$t.complianceRuleSeverity}
            </label>
            <select
              id="rule-severity"
              bind:value={ruleSeverity}
              class="w-full px-3 py-2 text-sm rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-700 text-slate-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-primary/50"
            >
              <option value="error">⛔ {$t.complianceSeverityError}</option>
              <option value="warning">⚠️ {$t.complianceSeverityWarning}</option>
              <option value="info">ℹ️ {$t.complianceSeverityInfo}</option>
            </select>
          </div>
        </div>

        <!-- Faz -->
        <fieldset class="border-0 p-0 m-0">
          <legend class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1">
            {$t.complianceRulePhases}
          </legend>
          <div class="flex items-center rounded-lg border border-slate-200 dark:border-slate-600 overflow-hidden text-xs font-medium w-fit">
            <button
              type="button"
              onclick={() => rulePhases = 0}
              class="px-3 py-2 transition-colors {rulePhases === 0
                ? 'bg-primary text-white'
                : 'text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700'}"
            >
              {$t.compliancePhasesAll}
            </button>
            <button
              type="button"
              onclick={() => rulePhases = 1}
              class="px-3 py-2 transition-colors {rulePhases === 1
                ? 'bg-primary text-white'
                : 'text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700'}"
            >
              1 Faz
            </button>
            <button
              type="button"
              onclick={() => rulePhases = 3}
              class="px-3 py-2 transition-colors {rulePhases === 3
                ? 'bg-primary text-white'
                : 'text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700'}"
            >
              3 Faz
            </button>
          </div>
        </fieldset>

        <!-- Ayırıcı -->
        <div class="border-t border-slate-100 dark:border-slate-700 pt-1">
          <p class="text-xs font-semibold text-slate-400 dark:text-slate-500 uppercase tracking-wide mb-3">Kontrol</p>

          <!-- Alan -->
          <div class="mb-3">
            <label for="rule-field" class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1">
              {$t.complianceRuleField}
            </label>
            <select
              id="rule-field"
              bind:value={ruleField}
              class="w-full px-3 py-2 text-sm rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-700 text-slate-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-primary/50"
            >
              {#each FIELD_GROUPS as group}
                <optgroup label={group.label}>
                  {#each group.fields as field}
                    <option value={field.value}>
                      {field.label}{field.phases ? ` (${field.phases} faz)` : ""}
                    </option>
                  {/each}
                </optgroup>
              {/each}
            </select>
          </div>

          <!-- Kontrol Tipi -->
          <div class="mb-3">
            <label for="rule-check" class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1">
              {$t.complianceRuleCheck}
            </label>
            <select
              id="rule-check"
              bind:value={ruleCheck}
              class="w-full px-3 py-2 text-sm rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-700 text-slate-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-primary/50"
            >
              {#each availableChecks as chk}
                <option value={chk.value}>{chk.label}</option>
              {/each}
            </select>
          </div>

          <!-- Dinamik parametreler -->
          {#if ruleCheck === "range"}
            <div class="grid grid-cols-2 gap-3">
              <div>
                <label for="rule-min" class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1">{$t.complianceRuleMin}</label>
                <input id="rule-min" type="number" bind:value={ruleMin} placeholder="90.0"
                  class="w-full px-3 py-2 text-sm rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-700 text-slate-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-primary/50" />
              </div>
              <div>
                <label for="rule-max" class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1">{$t.complianceRuleMax}</label>
                <input id="rule-max" type="number" bind:value={ruleMax} placeholder="265.0"
                  class="w-full px-3 py-2 text-sm rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-700 text-slate-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-primary/50" />
              </div>
            </div>
          {:else if ruleCheck === "equals" || ruleCheck === "not_equals"}
            <div>
              <label for="rule-value" class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1">{$t.complianceRuleValue}</label>
              {#if getFieldDef(ruleField)?.type === "enum" && getFieldDef(ruleField)?.options}
                <!-- Hazır seçenekler — battery_status, relay_status gibi alanlar -->
                <select
                  id="rule-value"
                  bind:value={ruleValue}
                  class="w-full px-3 py-2 text-sm rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-700 text-slate-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-primary/50"
                >
                  {#each getFieldDef(ruleField)!.options! as opt}
                    <option value={opt.value}>{opt.label}</option>
                  {/each}
                </select>
              {:else if getFieldDef(ruleField)?.type === "date"}
                <!-- Tarih alanı — geçersiz tarih formatı ipucu -->
                <input id="rule-value" type="text" bind:value={ruleValue} placeholder="00.00.0000"
                  class="w-full px-3 py-2 text-sm rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-700 text-slate-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-primary/50" />
                <p class="text-xs text-slate-400 mt-1">İpucu: Geçersiz/boş tarihi kontrol etmek için <span class="font-mono">00.00.0000</span> girin</p>
              {:else}
                <input id="rule-value" type="text" bind:value={ruleValue} placeholder="15"
                  class="w-full px-3 py-2 text-sm rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-700 text-slate-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-primary/50" />
              {/if}
            </div>
          {:else if ruleCheck === "bit_zero" || ruleCheck === "bit_one"}
            <div>
              <label for="rule-bit" class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1">{$t.complianceRuleBit} (0–63)</label>
              <input id="rule-bit" type="number" bind:value={ruleBit} placeholder="0" min="0" max="63"
                class="w-full px-3 py-2 text-sm rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-700 text-slate-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-primary/50" />
            </div>
          {:else if ruleCheck === "tariff_balance"}
            <div>
              <label for="rule-tolerance" class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1">{$t.complianceRuleTolerance}</label>
              <input id="rule-tolerance" type="number" bind:value={ruleTolerance} placeholder="0.01" step="0.001"
                class="w-full px-3 py-2 text-sm rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-700 text-slate-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-primary/50" />
            </div>
          {:else if ruleCheck === "time_drift_minutes"}
            <div>
              <label for="rule-maxdrift" class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1">{$t.complianceRuleMaxDrift}</label>
              <input id="rule-maxdrift" type="number" bind:value={ruleMaxDrift} placeholder="5" min="1"
                class="w-full px-3 py-2 text-sm rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-700 text-slate-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-primary/50" />
            </div>
          {/if}
        </div>

        <!-- Ayırıcı -->
        <div class="border-t border-slate-100 dark:border-slate-700 pt-1">
          <p class="text-xs font-semibold text-slate-400 dark:text-slate-500 uppercase tracking-wide mb-3">Açıklama</p>

          <div class="space-y-3">
            <div>
              <label for="rule-desc" class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1">
                {$t.complianceRuleDesc} *
              </label>
              <input id="rule-desc" type="text" bind:value={ruleDesc}
                placeholder="Kural açıklaması..."
                class="w-full px-3 py-2 text-sm rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-700 text-slate-900 dark:text-white placeholder-slate-400 focus:outline-none focus:ring-2 focus:ring-primary/50" />
            </div>
            <div>
              <label for="rule-specref" class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1">
                {$t.complianceRuleSpecRef}
              </label>
              <input id="rule-specref" type="text" bind:value={ruleSpecRef}
                placeholder="TEDAŞ Şartname 2.2.2 / MASS Şartname 2.2.2"
                class="w-full px-3 py-2 text-sm rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-700 text-slate-900 dark:text-white placeholder-slate-400 focus:outline-none focus:ring-2 focus:ring-primary/50" />
            </div>
          </div>
        </div>
      </div>

      <!-- Footer -->
      <div class="flex items-center justify-end gap-3 px-5 py-4 border-t border-slate-200 dark:border-slate-700">
        <button
          onclick={() => showAddRule = false}
          class="px-4 py-2 rounded-lg text-sm font-medium text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700 transition-all"
        >
          {$t.complianceRuleCancel}
        </button>
        <button
          onclick={saveRule}
          disabled={saving || !ruleCode.trim() || !ruleDesc.trim()}
          class="flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-all
            {saving || !ruleCode.trim() || !ruleDesc.trim()
              ? 'bg-slate-200 dark:bg-slate-700 text-slate-400 cursor-not-allowed'
              : 'bg-primary text-white hover:bg-primary/90 active:scale-95'}"
        >
          {#if saving}
            <span class="material-symbols-outlined text-base animate-spin-reverse">autorenew</span>
            Kaydediliyor...
          {:else}
            <span class="material-symbols-outlined text-base">save</span>
            {$t.complianceRuleSave}
          {/if}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- ─── Kuralları Yönet Modalı ─────────────────────────────────────────────── -->
{#if showManageRules}
  <div
    role="presentation"
    class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm"
    onclick={(e) => { if (e.target === e.currentTarget) showManageRules = false; }}
    onkeydown={(e) => { if (e.key === 'Escape') showManageRules = false; }}
  >
    <div class="w-full max-w-2xl bg-white dark:bg-slate-800 rounded-2xl shadow-2xl border border-slate-200 dark:border-slate-700 overflow-hidden flex flex-col max-h-[85vh]">
      <!-- Header -->
      <div class="flex items-center justify-between px-5 py-4 border-b border-slate-200 dark:border-slate-700 flex-shrink-0">
        <div class="flex items-center gap-2">
          <span class="material-symbols-outlined text-primary text-xl">manage_accounts</span>
          <h3 class="font-bold text-slate-900 dark:text-white">{$t.complianceManageRulesTitle}</h3>
          {#if !loadingRules}
            <span class="text-xs text-slate-400 ml-1">({managedRules.length} kural)</span>
          {/if}
        </div>
        <div class="flex items-center gap-2">
          <button
            onclick={() => { showManageRules = false; openAddRuleModal(); }}
            class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium border border-primary/40 text-primary hover:bg-primary/10 transition-all"
          >
            <span class="material-symbols-outlined text-sm">add_circle</span>
            {$t.complianceAddNew}
          </button>
          <button
            onclick={() => showManageRules = false}
            class="text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 transition-colors"
          >
            <span class="material-symbols-outlined">close</span>
          </button>
        </div>
      </div>

      <!-- Body -->
      <div class="overflow-y-auto flex-1 divide-y divide-slate-100 dark:divide-slate-700">
        {#if loadingRules}
          <div class="py-12 text-center">
            <span class="material-symbols-outlined text-3xl text-slate-400 animate-spin-reverse block">autorenew</span>
          </div>
        {:else if managedRules.length === 0}
          <div class="py-12 text-center text-slate-400 text-sm">{$t.complianceNoRules}</div>
        {:else}
          {#each managedRules as rule (rule.code)}
            <div class="flex items-center gap-3 px-5 py-3 hover:bg-slate-50 dark:hover:bg-slate-700/40 transition-colors group">
              <span class="w-2 h-2 rounded-full flex-shrink-0 {severityDot(rule.severity)}"></span>
              <div class="flex-1 min-w-0">
                <div class="flex items-center gap-2 flex-wrap">
                  <span class="text-xs font-mono font-bold text-slate-700 dark:text-slate-200">{rule.code}</span>
                  <span class="text-xs text-slate-400">{rule.field}</span>
                  <span class="text-xs bg-slate-100 dark:bg-slate-700 text-slate-500 dark:text-slate-400 px-1.5 py-0.5 rounded font-mono">{rule.check}</span>
                </div>
                <p class="text-xs text-slate-500 dark:text-slate-400 truncate mt-0.5">{rule.description}</p>
              </div>
              <div class="flex items-center gap-1 flex-shrink-0 opacity-0 group-hover:opacity-100 transition-opacity">
                <button
                  onclick={() => { showManageRules = false; openAddRuleModal(rule); }}
                  class="p-1.5 rounded-lg text-slate-400 hover:text-primary hover:bg-primary/10 transition-all"
                  title={$t.complianceEditRule}
                >
                  <span class="material-symbols-outlined text-sm">edit</span>
                </button>
                <button
                  onclick={() => confirmDelete(rule.code)}
                  disabled={deletingCode === rule.code}
                  class="p-1.5 rounded-lg text-slate-400 hover:text-red-500 hover:bg-red-500/10 transition-all disabled:opacity-50"
                  title={$t.complianceDeleteRule}
                >
                  <span class="material-symbols-outlined text-sm {deletingCode === rule.code ? 'animate-spin-reverse' : ''}">
                    {deletingCode === rule.code ? 'autorenew' : 'delete'}
                  </span>
                </button>
              </div>
            </div>
          {/each}
        {/if}
      </div>
    </div>
  </div>
{/if}

<!-- ─── Güncelleme Seçim Modalı ──────────────────────────────────────────────── -->
{#if showUpdateModal}
  <div
    role="presentation"
    class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm"
    onclick={(e) => { if (e.target === e.currentTarget) showUpdateModal = false; }}
    onkeydown={(e) => { if (e.key === 'Escape') showUpdateModal = false; }}
  >
    <div class="w-full max-w-sm bg-white dark:bg-slate-800 rounded-2xl shadow-2xl border border-slate-200 dark:border-slate-700 overflow-hidden">
      <div class="flex items-center justify-between px-5 py-4 border-b border-slate-200 dark:border-slate-700">
        <div class="flex items-center gap-2">
          <span class="material-symbols-outlined text-primary text-xl">cloud_download</span>
          <h3 class="font-bold text-slate-900 dark:text-white">{$t.complianceUpdateModalTitle}</h3>
        </div>
        <button
          onclick={() => showUpdateModal = false}
          class="text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 transition-colors"
        >
          <span class="material-symbols-outlined">close</span>
        </button>
      </div>
      <div class="p-4 flex flex-col gap-3">
        <button
          onclick={updateRulesFromInternet}
          class="flex items-center gap-4 p-4 rounded-xl border border-slate-200 dark:border-slate-600 hover:border-primary hover:bg-primary/5 transition-all text-left"
        >
          <span class="material-symbols-outlined text-primary text-3xl">cloud_download</span>
          <div>
            <p class="font-semibold text-slate-900 dark:text-white text-sm">{$t.complianceUpdateFromInternet}</p>
            <p class="text-xs text-slate-500 dark:text-slate-400 mt-0.5">{$t.complianceUpdateFromInternetDesc}</p>
          </div>
        </button>
        <button
          onclick={updateRulesFromFile}
          class="flex items-center gap-4 p-4 rounded-xl border border-slate-200 dark:border-slate-600 hover:border-primary hover:bg-primary/5 transition-all text-left"
        >
          <span class="material-symbols-outlined text-primary text-3xl">folder_open</span>
          <div>
            <p class="font-semibold text-slate-900 dark:text-white text-sm">{$t.complianceUpdateFromFile}</p>
            <p class="text-xs text-slate-500 dark:text-slate-400 mt-0.5">{$t.complianceUpdateFromFileDesc}</p>
          </div>
        </button>
      </div>
    </div>
  </div>
{/if}
