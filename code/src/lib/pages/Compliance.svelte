<script lang="ts">
  import { onMount } from "svelte";
  import { t, isConnected, navigationStore } from "$lib/stores";
  import { complianceStore, type ComplianceResult } from "$lib/stores/compliance";
  import { meterStore } from "$lib/stores/meter";
  import { connectionStore } from "$lib/stores/connection";
  import {
    checkCompliance,
    readShort,
    readFull,
    readObisBatch,
    readLoadProfile,
    readPacket,
    getComplianceRulesPath,
    reloadComplianceRules,
    updateComplianceRules,
    importComplianceRulesFromFile,
    addComplianceRule,
    listComplianceRules,
    updateComplianceRule,
    deleteComplianceRule,
    getComplianceProfiles,
    getComplianceTestPlan,
    isTauri,
    type ComplianceRuleDef,
    type ShortReadResult,
  } from "$lib/utils/tauri";
  import type { ComplianceProfile, TestStep } from "$lib/stores/compliance";
  import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
  import { warningToast, successToast, errorToast } from "$lib/stores/toast";

  // ─── State ───────────────────────────────────────────────────────────────────

  let reloading = $state(false);
  let updating = $state(false);
  let showUpdateModal = $state(false);

  // Profile & test plan
  let profiles = $state<ComplianceProfile[]>([]);
  let selectedProfileId = $state("");
  let testSteps = $state<TestStep[]>([]);

  // Test runner
  type StepStatus = "pending" | "running" | "done" | "failed" | "skipped";
  interface StepState {
    step: TestStep;
    status: StepStatus;
    message: string;
    durationMs: number;
    issueCount: number;
  }

  let running = $state(false);
  let stepStates = $state<StepState[]>([]);
  let currentStepIndex = $state(-1);
  let runLog = $state<string[]>([]);

  // Collected data from readings
  let shortReadData = $state<ShortReadResult | null>(null);
  let fullReadData = $state<ShortReadResult | null>(null);

  // Results
  const result = $derived($complianceStore.result);

  // Phase detection
  const meterPhases: 1 | 3 = $derived.by(() => {
    const mt = $meterStore.meterType;
    if (mt === "three-phase" || mt === "kombi") return 3;
    if (mt === "single-phase") return 1;
    const data = $meterStore.shortReadData ?? $meterStore.fullReadData;
    if (data?.voltageL2 !== undefined || data?.currentL2 !== undefined) return 3;
    const model = $connectionStore.meterIdentity?.model ?? "";
    if (/3[Pp]|TP|MT\d|3PH/i.test(model)) return 3;
    return 3;
  });

  // Auto-select profile based on phases
  $effect(() => {
    if (profiles.length > 0 && !selectedProfileId) {
      const defaultId = meterPhases === 1 ? "single_phase" : "three_phase_direct";
      const match = profiles.find(p => p.id === defaultId);
      selectedProfileId = match?.id ?? profiles[0].id;
    }
  });

  // ─── Init ────────────────────────────────────────────────────────────────────

  onMount(async () => {
    if (!isTauri()) return;
    try {
      const [profs, plan] = await Promise.all([
        getComplianceProfiles(),
        getComplianceTestPlan(),
      ]);
      profiles = profs;
      if (plan) testSteps = plan.steps.filter(s => s.enabled);
      complianceStore.setProfiles(profs);
      complianceStore.setTestPlan(plan);
    } catch (e) {
      console.error("Failed to load compliance config:", e);
    }
  });

  // ─── Test Runner ─────────────────────────────────────────────────────────────

  function addLog(msg: string) {
    const ts = new Date().toLocaleTimeString("tr-TR", { hour: "2-digit", minute: "2-digit", second: "2-digit" });
    runLog = [...runLog, `[${ts}] ${msg}`];
  }

  async function runFullTest() {
    if (!$isConnected || running) return;

    running = true;
    runLog = [];
    shortReadData = null;
    fullReadData = null;
    complianceStore.setLoading();

    // Initialize step states
    stepStates = testSteps.map(step => ({
      step,
      status: "pending" as StepStatus,
      message: "",
      durationMs: 0,
      issueCount: 0,
    }));

    addLog(`Test başlatıldı — ${profiles.find(p => p.id === selectedProfileId)?.name ?? selectedProfileId}`);

    let latestData: ShortReadResult | null = null;

    for (let i = 0; i < stepStates.length; i++) {
      currentStepIndex = i;
      const ss = stepStates[i];
      ss.status = "running";
      ss.message = "Okuma yapılıyor...";
      stepStates = [...stepStates]; // trigger reactivity

      const t0 = performance.now();
      addLog(`Adım ${i + 1}/${stepStates.length}: ${ss.step.name}`);

      try {
        switch (ss.step.mode) {
          case "short_read": {
            const data = await readShort();
            shortReadData = data;
            latestData = data;

            // Detect meter type
            const raw = data.rawData || "";
            let meterType: "single-phase" | "three-phase" | "kombi" = "single-phase";
            if (raw.includes("52.7.0") || raw.includes("72.7.0")) meterType = "three-phase";
            meterStore.setShortReadData(data, meterType, false);

            ss.message = `${Object.keys(data).length} alan okundu`;
            addLog(`  Kısa okuma tamamlandı — Seri: ${data.serialNumber}`);
            break;
          }

          case "full_read": {
            const data = await readFull();
            fullReadData = data;
            latestData = data;

            // Re-read FF codes for fresh latch data
            try {
              const fresh = await readObisBatch(["F.F.0", "F.F.1"]);
              if (fresh["F.F.0"]) (data as any).ffCode = fresh["F.F.0"];
              if (fresh["F.F.1"]) (data as any).gfCode = fresh["F.F.1"];
            } catch { /* fallback to readFull result */ }

            const raw = data.rawData || "";
            let meterType: "single-phase" | "three-phase" | "kombi" = "single-phase";
            if (raw.includes("52.7.0") || raw.includes("72.7.0")) meterType = "three-phase";
            meterStore.setShortReadData(data, meterType, false);

            const lineCount = raw.split("\n").filter(l => l.trim()).length;
            ss.message = `${lineCount} OBIS satırı okundu`;
            addLog(`  Tam okuma tamamlandı — ${lineCount} satır`);
            break;
          }

          case "load_profile": {
            try {
              const lp = await readLoadProfile(1, null, null);
              const entryCount = lp.entries?.length ?? 0;
              ss.message = `${entryCount} kayıt okundu`;
              addLog(`  Yük profili tamamlandı — ${entryCount} kayıt`);
              if (lp.entries && lp.entries.length > 0) {
                meterStore.setLoadProfileData(lp as any);
              }
            } catch (e) {
              // Load profile may not be available on all meters
              ss.message = `Okunamadı: ${e}`;
              ss.status = "failed";
              addLog(`  Yük profili başarısız: ${e}`);
            }
            break;
          }

          case "packet_read": {
            const pMode = ss.step.packetMode ?? 7;
            const pResult = await readPacket(pMode);
            ss.message = `Mode ${pMode}: ${pResult.bytesRead} bayt okundu`;
            addLog(`  Paket okuma (Mode ${pMode}) tamamlandı — ${pResult.bytesRead} bayt, ${pResult.readDurationMs}ms, BCC: ${pResult.bccValid ? 'OK' : 'FAIL'}`);
            break;
          }

          case "obis_read": {
            const codes = ss.step.obisCodes ?? ["0.9.1", "0.9.2"];
            const values = await readObisBatch(codes);
            const readCount = Object.keys(values).length;
            ss.message = `${readCount}/${codes.length} kod okundu`;
            addLog(`  OBIS okuma tamamlandı — ${readCount} kod: ${JSON.stringify(values)}`);
            break;
          }

          default:
            ss.message = `Bilinmeyen mod: ${ss.step.mode}`;
            ss.status = "skipped";
            addLog(`  Atlandı — bilinmeyen mod: ${ss.step.mode}`);
        }

        if (ss.status === "running") ss.status = "done";
      } catch (e) {
        ss.status = "failed";
        ss.message = String(e);
        addLog(`  HATA: ${e}`);
      }

      ss.durationMs = Math.round(performance.now() - t0);
      stepStates = [...stepStates];
    }

    // Run compliance check with the best available data
    const checkData = latestData ?? fullReadData ?? shortReadData;
    if (checkData) {
      addLog("Uyumluluk kontrolü yapılıyor...");
      try {
        const res = await checkCompliance(checkData as any, meterPhases);
        complianceStore.setResult(res);

        // Update step issue counts based on categories
        // (simplified: distribute total across steps for visual feedback)
        const totalIssues = res.errorCount + res.warningCount;
        addLog(`Kontrol tamamlandı — ${res.errorCount} hata, ${res.warningCount} uyarı, ${res.infoCount} bilgi`);

        if (res.errorCount === 0 && res.warningCount === 0) {
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
        addLog(`Kontrol hatası: ${e}`);
      }
    } else {
      complianceStore.setError("Hiçbir okuma başarılı olmadı");
      addLog("HATA: Hiçbir okuma başarılı olmadı — kontrol yapılamıyor");
    }

    currentStepIndex = -1;
    running = false;
    addLog("Test tamamlandı.");
  }

  // ─── Quick check on existing data ────────────────────────────────────────────

  async function runCheckOnly() {
    const data = fullReadData ?? shortReadData ?? $meterStore.shortReadData ?? $meterStore.fullReadData;
    if (!data) return;
    complianceStore.setLoading();
    try {
      const res = await checkCompliance(data as any, meterPhases);
      complianceStore.setResult(res);
      if (res.errorCount === 0 && res.warningCount === 0) {
        successToast($t.complianceAllPassed);
      }
    } catch (e) {
      complianceStore.setError(String(e));
      errorToast(String(e));
    }
  }

  // ─── Helpers ─────────────────────────────────────────────────────────────────

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

  function specSources(specRef: string): string[] {
    return specRef.split(" / ").map(part =>
      part.trim().replace(/^TEDAS(\s|§)/, "TEDAŞ$1")
    );
  }

  function stepIcon(mode: string): string {
    switch (mode) {
      case "short_read": return "electric_meter";
      case "full_read": return "description";
      case "load_profile": return "bar_chart";
      case "packet_read": return "inventory_2";
      case "obis_read": return "schedule";
      default: return "help";
    }
  }

  function statusIcon(s: StepStatus): string {
    switch (s) {
      case "pending": return "radio_button_unchecked";
      case "running": return "autorenew";
      case "done": return "check_circle";
      case "failed": return "error";
      case "skipped": return "skip_next";
    }
  }

  function statusColor(s: StepStatus): string {
    switch (s) {
      case "pending": return "text-slate-400";
      case "running": return "text-primary";
      case "done": return "text-emerald-500";
      case "failed": return "text-red-500";
      case "skipped": return "text-yellow-500";
    }
  }

  // ─── Rules management ────────────────────────────────────────────────────────

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
    updating = true;
    try {
      const msg = await importComplianceRulesFromFile(selected);
      successToast(msg);
    } catch (e) {
      errorToast(String(e));
    } finally {
      updating = false;
    }
  }

  // ─── Rule Add/Edit Modal ────────────────────────────────────────────────────

  type FieldType = "numeric" | "text" | "date" | "datetime" | "enum" | "bitfield" | "tariff_sum" | "time_drift";

  interface FieldDef {
    value: string;
    label: string;
    type: FieldType;
    phases?: number;
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
        return [{ value: "not_empty", label: $t.complianceCheckNotEmpty }];
      case "date":
        return [
          { value: "not_empty", label: $t.complianceCheckNotEmpty },
          { value: "not_equals", label: `${$t.complianceCheckNotEquals} (ör. geçersiz tarih)` },
        ];
      case "enum":
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

  let showAddRule = $state(false);
  let editMode = $state(false);
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
    ruleSpecRef = prefill?.specRef ?? "";
    ruleMin = prefill?.min != null ? String(prefill.min) : "";
    ruleMax = prefill?.max != null ? String(prefill.max) : "";
    ruleValue = prefill?.value ?? "";
    ruleBit = prefill?.bit != null ? String(prefill.bit) : "";
    ruleTolerance = prefill?.tolerance != null ? String(prefill.tolerance) : "";
    ruleMaxDrift = prefill?.maxDrift != null ? String(prefill.maxDrift) : "";
    showAddRule = true;
  }

  async function saveRule() {
    if (!ruleCode.trim() || !ruleDesc.trim()) return;
    saving = true;
    try {
      if (editMode) {
        const rule: ComplianceRuleDef = {
          code: ruleCode.trim(),
          category: "obis_value",
          check: ruleCheck,
          severity: ruleSeverity,
          description: ruleDesc.trim(),
          obisCode: null,
          obisCodes: [],
          specRef: ruleSpecRef.trim() || null,
          profile: rulePhases === 3 ? ["three_phase_direct", "three_phase_ct"] : rulePhases === 1 ? ["single_phase"] : [],
          sessionType: null,
          min: ruleCheck === "range" ? parseFloat(ruleMin) : null,
          max: ruleCheck === "range" ? parseFloat(ruleMax) : null,
          value: (ruleCheck === "equals" || ruleCheck === "not_equals") ? ruleValue : null,
          bit: (ruleCheck === "bit_zero" || ruleCheck === "bit_one") ? parseInt(ruleBit) : null,
          tolerance: ruleCheck === "tariff_balance" ? parseFloat(ruleTolerance) : null,
          maxDrift: ruleCheck === "time_drift_minutes" ? parseInt(ruleMaxDrift) : null,
          enabled: true,
          cause: null,
          remedy: null,
          field: ruleField,
          phases: rulePhases !== 0 ? rulePhases : null,
        };
        await updateComplianceRule(rule);
        successToast($t.complianceRuleUpdated);
        await loadManagedRules();
      } else {
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

  // ─── Manage Rules Modal ────────────────────────────────────────────────────

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

  // Computed: total progress
  const completedSteps = $derived(stepStates.filter(s => s.status === "done" || s.status === "failed" || s.status === "skipped").length);
  const progressPct = $derived(stepStates.length > 0 ? Math.round((completedSteps / stepStates.length) * 100) : 0);

  // Sorted issues for display
  const sortedIssues = $derived(
    result?.issues.slice().sort((a, b) => {
      const order = { error: 0, warning: 1, info: 2 };
      return (order[a.severity as keyof typeof order] ?? 3) -
             (order[b.severity as keyof typeof order] ?? 3);
    }) ?? []
  );
</script>

<div class="space-y-6">

  <!-- Header -->
  <div class="flex items-start justify-between gap-4 flex-wrap">
    <div>
      <h2 class="text-xl font-bold text-slate-900 dark:text-white">
        TEDAŞ Uyumluluk Kontrolü
      </h2>
      <p class="text-sm text-slate-500 dark:text-slate-400 mt-1">
        {$t.complianceDescription}
      </p>
    </div>

    <div class="flex items-center gap-2">
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
      <button onclick={openManageRules}
        class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium border border-slate-200 dark:border-slate-600 text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700 transition-all">
        <span class="material-symbols-outlined text-sm">rule_settings</span>
        {$t.complianceManageRules ?? "Kuralları Yönet"}
      </button>
    </div>
  </div>

  <!-- Not connected warning -->
  {#if !$isConnected}
    <div class="rounded-xl border border-dashed border-slate-300 dark:border-slate-600 p-8 text-center space-y-4">
      <span class="material-symbols-outlined text-5xl text-slate-300 dark:text-slate-600 block">sensors_off</span>
      <div>
        <p class="text-slate-600 dark:text-slate-400 font-medium">Sayaca bağlı değilsiniz</p>
        <p class="text-sm text-slate-500 dark:text-slate-500 mt-1">Uyumluluk testi için önce bir sayaca bağlanın.</p>
      </div>
      <button
        onclick={() => navigationStore.navigate("dashboard")}
        class="inline-flex items-center gap-2 px-5 py-2.5 bg-primary hover:bg-primary/90 text-white font-bold rounded-lg transition-all"
      >
        <span class="material-symbols-outlined text-base">cable</span>
        Bağlantı Sayfasına Git
      </button>
    </div>

  {:else}

    <!-- Test Runner Card -->
    <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl shadow-sm overflow-hidden">
      <!-- Runner Header -->
      <div class="bg-gradient-to-r from-primary/10 to-emerald-500/10 dark:from-primary/20 dark:to-emerald-500/20 px-6 py-4 border-b border-slate-200 dark:border-[#334a5e]">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-3">
            <div class="p-2 bg-primary/10 rounded-lg">
              <span class="material-symbols-outlined text-primary">play_circle</span>
            </div>
            <div>
              <h3 class="text-base font-bold text-slate-900 dark:text-white">Test Çalıştır</h3>
              <p class="text-xs text-slate-500 dark:text-slate-400">
                {testSteps.length} adımlı uyumluluk testi
              </p>
            </div>
          </div>

          <div class="flex items-center gap-3">
            <!-- Profile selector -->
            <div class="flex items-center gap-2">
              <span class="text-xs text-slate-500">Profil:</span>
              <select
                bind:value={selectedProfileId}
                disabled={running}
                class="px-3 py-1.5 text-xs font-medium rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-[#1a2632] text-slate-900 dark:text-white focus:border-primary focus:ring-1 focus:ring-primary transition-colors disabled:opacity-50"
              >
                {#each profiles as p}
                  <option value={p.id}>{p.name}</option>
                {/each}
              </select>
            </div>

            <!-- Phase indicator -->
            <div class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg border border-slate-200 dark:border-slate-600 text-xs font-medium text-slate-600 dark:text-slate-300">
              <span class="material-symbols-outlined text-sm text-primary">electric_bolt</span>
              {meterPhases} Faz
            </div>

            <!-- Run button -->
            <button
              onclick={runFullTest}
              disabled={running || !$isConnected}
              class="flex items-center gap-2 px-5 py-2.5 rounded-lg font-bold text-sm transition-all
                {running
                  ? 'bg-primary/20 text-primary cursor-wait'
                  : 'bg-primary text-white hover:bg-primary/90 active:scale-95 shadow-lg shadow-primary/20'}"
            >
              {#if running}
                <span class="material-symbols-outlined text-base animate-spin-reverse">autorenew</span>
                Test Devam Ediyor...
              {:else}
                <span class="material-symbols-outlined text-base">play_arrow</span>
                Testi Başlat
              {/if}
            </button>
          </div>
        </div>
      </div>

      <!-- Test Steps -->
      <div class="p-6">
        {#if stepStates.length === 0 && !running}
          <!-- Before first run: show plan -->
          <div class="space-y-3">
            {#each testSteps as step, i}
              <div class="flex items-center gap-4 p-3 rounded-lg bg-slate-50 dark:bg-[#0f1821] border border-slate-200 dark:border-[#334a5e]">
                <div class="flex items-center justify-center w-8 h-8 rounded-full bg-slate-100 dark:bg-slate-700">
                  <span class="text-xs font-bold text-slate-500">{i + 1}</span>
                </div>
                <span class="material-symbols-outlined text-slate-400">{stepIcon(step.mode)}</span>
                <div class="flex-1">
                  <p class="text-sm font-medium text-slate-700 dark:text-slate-200">{step.name}</p>
                  <p class="text-xs text-slate-400">Zaman aşımı: {step.timeoutSeconds}s</p>
                </div>
              </div>
            {/each}
          </div>

        {:else}
          <!-- During / after run: show step status -->

          <!-- Progress bar -->
          {#if running}
            <div class="mb-5">
              <div class="flex items-center justify-between mb-1.5">
                <span class="text-xs font-medium text-slate-500">İlerleme</span>
                <span class="text-xs font-bold text-primary">{progressPct}%</span>
              </div>
              <div class="w-full h-2 bg-slate-100 dark:bg-slate-700 rounded-full overflow-hidden">
                <div
                  class="h-full bg-gradient-to-r from-primary to-emerald-400 rounded-full transition-all duration-500"
                  style="width: {progressPct}%"
                ></div>
              </div>
            </div>
          {/if}

          <div class="space-y-2">
            {#each stepStates as ss, i}
              <div class="flex items-center gap-4 p-3 rounded-lg border transition-all
                {ss.status === 'running'
                  ? 'bg-primary/5 border-primary/20 ring-1 ring-primary/10'
                  : ss.status === 'done'
                  ? 'bg-emerald-500/5 border-emerald-500/15'
                  : ss.status === 'failed'
                  ? 'bg-red-500/5 border-red-500/15'
                  : 'bg-slate-50 dark:bg-[#0f1821] border-slate-200 dark:border-[#334a5e]'}">
                <!-- Step number -->
                <div class="flex items-center justify-center w-8 h-8 rounded-full flex-shrink-0
                  {ss.status === 'done' ? 'bg-emerald-500/10' : ss.status === 'failed' ? 'bg-red-500/10' : ss.status === 'running' ? 'bg-primary/10' : 'bg-slate-100 dark:bg-slate-700'}">
                  <span class="material-symbols-outlined text-base {statusColor(ss.status)} {ss.status === 'running' ? 'animate-spin-reverse' : ''}">
                    {statusIcon(ss.status)}
                  </span>
                </div>

                <!-- Step icon -->
                <span class="material-symbols-outlined {ss.status === 'running' ? 'text-primary' : 'text-slate-400'}">
                  {stepIcon(ss.step.mode)}
                </span>

                <!-- Step info -->
                <div class="flex-1 min-w-0">
                  <p class="text-sm font-medium {ss.status === 'running' ? 'text-primary' : 'text-slate-700 dark:text-slate-200'}">
                    {ss.step.name}
                  </p>
                  {#if ss.message}
                    <p class="text-xs {ss.status === 'failed' ? 'text-red-500' : 'text-slate-400'} truncate">
                      {ss.message}
                    </p>
                  {/if}
                </div>

                <!-- Duration -->
                {#if ss.durationMs > 0}
                  <span class="text-xs font-mono text-slate-400 flex-shrink-0">
                    {(ss.durationMs / 1000).toFixed(1)}s
                  </span>
                {/if}
              </div>
            {/each}
          </div>

          <!-- Run log (collapsible) -->
          {#if runLog.length > 0}
            <details class="mt-4">
              <summary class="text-xs font-medium text-slate-500 cursor-pointer hover:text-primary transition-colors">
                Test Günlüğü ({runLog.length} satır)
              </summary>
              <div class="mt-2 p-3 rounded-lg bg-slate-50 dark:bg-[#0f1821] border border-slate-200 dark:border-[#334a5e] max-h-48 overflow-y-auto font-mono text-[11px] text-slate-500 dark:text-slate-400 space-y-0.5">
                {#each runLog as line}
                  <div class="{line.includes('HATA') ? 'text-red-500' : line.includes('tamamlandı') ? 'text-emerald-500' : ''}">{line}</div>
                {/each}
              </div>
            </details>
          {/if}
        {/if}
      </div>
    </div>

    <!-- Results Section (shown after test completes) -->
    {#if result && !running}
      <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl shadow-sm overflow-hidden">
        <!-- Results Header -->
        <div class="px-6 py-4 border-b border-slate-200 dark:border-[#334a5e]">
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-3">
              <span class="material-symbols-outlined text-xl {result.errorCount > 0 ? 'text-red-500' : result.warningCount > 0 ? 'text-yellow-500' : 'text-emerald-500'}">
                {result.errorCount > 0 ? 'gpp_bad' : result.warningCount > 0 ? 'gpp_maybe' : 'verified_user'}
              </span>
              <div>
                <h3 class="text-base font-bold text-slate-900 dark:text-white">Kontrol Sonuçları</h3>
                <span class="text-xs text-slate-500">
                  v{result.configVersion} · {formatTime(result.checkedAt)} · {result.totalRulesChecked} kural kontrol edildi
                </span>
              </div>
            </div>

            <!-- Re-check button -->
            <button
              onclick={runCheckOnly}
              class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium border border-slate-200 dark:border-slate-600 text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700 transition-all"
            >
              <span class="material-symbols-outlined text-sm">verified_user</span>
              Tekrar Kontrol
            </button>
          </div>
        </div>

        <div class="p-6 space-y-5">
          <!-- Rules status warnings -->
          {#if result.rulesStatus === "tooOld"}
            <div class="rounded-lg border border-red-500/30 bg-red-500/10 px-4 py-3 flex items-center gap-2 text-sm text-red-500">
              <span class="material-symbols-outlined text-base">block</span>
              {$t.complianceTooOld}
              <button onclick={() => showUpdateModal = true} disabled={updating} class="ml-auto text-xs underline disabled:opacity-50">
                {$t.complianceUpdate}
              </button>
            </div>
          {:else if result.rulesStatus === "offline"}
            <div class="rounded-lg border border-yellow-500/30 bg-yellow-500/10 px-4 py-2 flex items-center gap-2 text-sm text-yellow-600 dark:text-yellow-400">
              <span class="material-symbols-outlined text-base">wifi_off</span>
              {$t.complianceOffline}
            </div>
          {:else if result.latestVersion && result.latestVersion !== result.configVersion}
            <div class="rounded-lg border border-blue-500/30 bg-blue-500/10 px-4 py-2 flex items-center gap-2 text-sm text-blue-600 dark:text-blue-400">
              <span class="material-symbols-outlined text-base">update</span>
              {$t.complianceOutdated}: v{result.latestVersion}
              <button onclick={() => showUpdateModal = true} disabled={updating} class="ml-auto text-xs underline disabled:opacity-50">
                {$t.complianceUpdate}
              </button>
            </div>
          {/if}

          <!-- Summary -->
          <div class="grid grid-cols-3 gap-3">
            <div class="rounded-xl p-4 border {result.errorCount > 0 ? 'bg-red-500/10 border-red-500/20' : 'bg-slate-50 dark:bg-slate-800/50 border-slate-200 dark:border-slate-700'}">
              <div class="flex items-center gap-3">
                <span class="material-symbols-outlined text-2xl {result.errorCount > 0 ? 'text-red-500' : 'text-slate-300 dark:text-slate-600'}">
                  {result.errorCount > 0 ? 'cancel' : 'check_circle'}
                </span>
                <div>
                  <p class="text-2xl font-bold {result.errorCount > 0 ? 'text-red-500' : 'text-slate-400'}">{result.errorCount}</p>
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
                  <p class="text-2xl font-bold {result.warningCount > 0 ? 'text-yellow-500' : 'text-slate-400'}">{result.warningCount}</p>
                  <p class="text-xs text-slate-500">{$t.complianceWarnings}</p>
                </div>
              </div>
            </div>
            <div class="rounded-xl p-4 border bg-slate-50 dark:bg-slate-800/50 border-slate-200 dark:border-slate-700">
              <div class="flex items-center gap-3">
                <span class="material-symbols-outlined text-2xl text-blue-400">info</span>
                <div>
                  <p class="text-2xl font-bold text-slate-400">{result.infoCount}</p>
                  <p class="text-xs text-slate-500">Bilgi</p>
                </div>
              </div>
            </div>
          </div>

          <!-- All passed -->
          {#if result.issues.length === 0}
            <div class="rounded-xl border border-emerald-500/20 bg-emerald-500/10 p-6 text-center">
              <span class="material-symbols-outlined text-4xl text-emerald-500 mb-2 block">verified</span>
              <p class="font-bold text-emerald-600 dark:text-emerald-400">{$t.complianceAllPassed}</p>
              <p class="text-sm text-slate-500 mt-1">{$t.complianceAllPassedDesc}</p>
            </div>
          {:else}
            <!-- Issues list -->
            <div class="space-y-2">
              {#each sortedIssues as issue (issue.code)}
                <div class="rounded-xl border overflow-hidden {severityBorder(issue.severity)}">
                  <div class="grid grid-cols-2 divide-x divide-slate-200/60 dark:divide-slate-700/60">

                    <!-- LEFT: rule info -->
                    <div class="p-4 flex items-start gap-3">
                      <span class="material-symbols-outlined text-xl mt-0.5 {severityText(issue.severity)} flex-shrink-0">
                        {severityIcon(issue.severity)}
                      </span>
                      <div class="min-w-0">
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

                    <!-- RIGHT: cause & remedy -->
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

            <p class="text-xs text-slate-400 dark:text-slate-500 text-right">
              {$t.complianceReportNote} v{result.configVersion}
              {#if result.latestVersion && result.latestVersion !== result.configVersion}
                · {$t.complianceOutdated}: v{result.latestVersion}
              {/if}
            </p>
          {/if}
        </div>
      </div>
    {/if}

  {/if}
</div>

<!-- ─── Yeni Kural Ekle Modalı ─────────────────────────────────────────────── -->
{#if showAddRule}
  <div
    role="presentation"
    class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm"
    onclick={(e) => { if (e.target === e.currentTarget) showAddRule = false; }}
    onkeydown={(e) => { if (e.key === 'Escape') showAddRule = false; }}
  >
    <div class="w-full max-w-md bg-white dark:bg-slate-800 rounded-2xl shadow-2xl border border-slate-200 dark:border-slate-700 overflow-hidden">
      <div class="flex items-center justify-between px-5 py-4 border-b border-slate-200 dark:border-slate-700">
        <div class="flex items-center gap-2">
          <span class="material-symbols-outlined text-primary text-xl">{editMode ? 'edit' : 'add_circle'}</span>
          <h3 class="font-bold text-slate-900 dark:text-white">
            {editMode ? $t.complianceEditRule + ': ' + ruleCode : $t.complianceAddRuleTitle}
          </h3>
        </div>
        <button onclick={() => showAddRule = false} class="text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 transition-colors">
          <span class="material-symbols-outlined">close</span>
        </button>
      </div>
      <div class="px-5 py-4 space-y-4 max-h-[70vh] overflow-y-auto">
        <div class="grid grid-cols-2 gap-3">
          <div>
            <label for="rule-code" class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1">{$t.complianceRuleCode} *</label>
            <input id="rule-code" type="text" bind:value={ruleCode} placeholder="EL-005"
              class="w-full px-3 py-2 text-sm rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-700 text-slate-900 dark:text-white placeholder-slate-400 focus:outline-none focus:ring-2 focus:ring-primary/50" />
          </div>
          <div>
            <label for="rule-severity" class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1">{$t.complianceRuleSeverity}</label>
            <select id="rule-severity" bind:value={ruleSeverity}
              class="w-full px-3 py-2 text-sm rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-700 text-slate-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-primary/50">
              <option value="error">{$t.complianceSeverityError}</option>
              <option value="warning">{$t.complianceSeverityWarning}</option>
              <option value="info">{$t.complianceSeverityInfo}</option>
            </select>
          </div>
        </div>
        <fieldset class="border-0 p-0 m-0">
          <legend class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1">{$t.complianceRulePhases}</legend>
          <div class="flex items-center rounded-lg border border-slate-200 dark:border-slate-600 overflow-hidden text-xs font-medium w-fit">
            <button type="button" onclick={() => rulePhases = 0}
              class="px-3 py-2 transition-colors {rulePhases === 0 ? 'bg-primary text-white' : 'text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700'}">{$t.compliancePhasesAll}</button>
            <button type="button" onclick={() => rulePhases = 1}
              class="px-3 py-2 transition-colors {rulePhases === 1 ? 'bg-primary text-white' : 'text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700'}">1 Faz</button>
            <button type="button" onclick={() => rulePhases = 3}
              class="px-3 py-2 transition-colors {rulePhases === 3 ? 'bg-primary text-white' : 'text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700'}">3 Faz</button>
          </div>
        </fieldset>
        <div class="border-t border-slate-100 dark:border-slate-700 pt-1">
          <p class="text-xs font-semibold text-slate-400 dark:text-slate-500 uppercase tracking-wide mb-3">Kontrol</p>
          <div class="mb-3">
            <label for="rule-field" class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1">{$t.complianceRuleField}</label>
            <select id="rule-field" bind:value={ruleField}
              class="w-full px-3 py-2 text-sm rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-700 text-slate-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-primary/50">
              {#each FIELD_GROUPS as group}
                <optgroup label={group.label}>
                  {#each group.fields as field}
                    <option value={field.value}>{field.label}{field.phases ? ` (${field.phases} faz)` : ""}</option>
                  {/each}
                </optgroup>
              {/each}
            </select>
          </div>
          <div class="mb-3">
            <label for="rule-check" class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1">{$t.complianceRuleCheck}</label>
            <select id="rule-check" bind:value={ruleCheck}
              class="w-full px-3 py-2 text-sm rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-700 text-slate-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-primary/50">
              {#each availableChecks as chk}
                <option value={chk.value}>{chk.label}</option>
              {/each}
            </select>
          </div>
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
                <select id="rule-value" bind:value={ruleValue}
                  class="w-full px-3 py-2 text-sm rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-700 text-slate-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-primary/50">
                  {#each getFieldDef(ruleField)!.options! as opt}
                    <option value={opt.value}>{opt.label}</option>
                  {/each}
                </select>
              {:else if getFieldDef(ruleField)?.type === "date"}
                <input id="rule-value" type="text" bind:value={ruleValue} placeholder="00.00.0000"
                  class="w-full px-3 py-2 text-sm rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-700 text-slate-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-primary/50" />
              {:else}
                <input id="rule-value" type="text" bind:value={ruleValue} placeholder="15"
                  class="w-full px-3 py-2 text-sm rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-700 text-slate-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-primary/50" />
              {/if}
            </div>
          {:else if ruleCheck === "bit_zero" || ruleCheck === "bit_one"}
            <div>
              <label for="rule-bit" class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1">{$t.complianceRuleBit} (0-63)</label>
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
        <div class="border-t border-slate-100 dark:border-slate-700 pt-1">
          <p class="text-xs font-semibold text-slate-400 dark:text-slate-500 uppercase tracking-wide mb-3">Açıklama</p>
          <div class="space-y-3">
            <div>
              <label for="rule-desc" class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1">{$t.complianceRuleDesc} *</label>
              <input id="rule-desc" type="text" bind:value={ruleDesc} placeholder="Kural açıklaması..."
                class="w-full px-3 py-2 text-sm rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-700 text-slate-900 dark:text-white placeholder-slate-400 focus:outline-none focus:ring-2 focus:ring-primary/50" />
            </div>
            <div>
              <label for="rule-specref" class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1">{$t.complianceRuleSpecRef}</label>
              <input id="rule-specref" type="text" bind:value={ruleSpecRef} placeholder="TEDAŞ Şartname 2.2.2 / MASS Şartname 2.2.2"
                class="w-full px-3 py-2 text-sm rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-700 text-slate-900 dark:text-white placeholder-slate-400 focus:outline-none focus:ring-2 focus:ring-primary/50" />
            </div>
          </div>
        </div>
      </div>
      <div class="flex items-center justify-end gap-3 px-5 py-4 border-t border-slate-200 dark:border-slate-700">
        <button onclick={() => showAddRule = false}
          class="px-4 py-2 rounded-lg text-sm font-medium text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700 transition-all">{$t.complianceRuleCancel}</button>
        <button onclick={saveRule} disabled={saving || !ruleCode.trim() || !ruleDesc.trim()}
          class="flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-all
            {saving || !ruleCode.trim() || !ruleDesc.trim()
              ? 'bg-slate-200 dark:bg-slate-700 text-slate-400 cursor-not-allowed'
              : 'bg-primary text-white hover:bg-primary/90 active:scale-95'}">
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
      <div class="flex items-center justify-between px-5 py-4 border-b border-slate-200 dark:border-slate-700 flex-shrink-0">
        <div class="flex items-center gap-2">
          <span class="material-symbols-outlined text-primary text-xl">manage_accounts</span>
          <h3 class="font-bold text-slate-900 dark:text-white">{$t.complianceManageRulesTitle}</h3>
          {#if !loadingRules}
            <span class="text-xs text-slate-400 ml-1">({managedRules.length} kural)</span>
          {/if}
        </div>
        <div class="flex items-center gap-2">
          <button onclick={() => { showManageRules = false; openAddRuleModal(); }}
            class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium border border-primary/40 text-primary hover:bg-primary/10 transition-all">
            <span class="material-symbols-outlined text-sm">add_circle</span>
            {$t.complianceAddNew}
          </button>
          <button onclick={() => showManageRules = false} class="text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 transition-colors">
            <span class="material-symbols-outlined">close</span>
          </button>
        </div>
      </div>
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
                <button onclick={() => { showManageRules = false; openAddRuleModal(rule); }}
                  class="p-1.5 rounded-lg text-slate-400 hover:text-primary hover:bg-primary/10 transition-all" title={$t.complianceEditRule}>
                  <span class="material-symbols-outlined text-sm">edit</span>
                </button>
                <button onclick={() => confirmDelete(rule.code)} disabled={deletingCode === rule.code}
                  class="p-1.5 rounded-lg text-slate-400 hover:text-red-500 hover:bg-red-500/10 transition-all disabled:opacity-50" title={$t.complianceDeleteRule}>
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
        <button onclick={() => showUpdateModal = false} class="text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 transition-colors">
          <span class="material-symbols-outlined">close</span>
        </button>
      </div>
      <div class="p-4 flex flex-col gap-3">
        <button onclick={updateRulesFromInternet}
          class="flex items-center gap-4 p-4 rounded-xl border border-slate-200 dark:border-slate-600 hover:border-primary hover:bg-primary/5 transition-all text-left">
          <span class="material-symbols-outlined text-primary text-3xl">cloud_download</span>
          <div>
            <p class="font-semibold text-slate-900 dark:text-white text-sm">{$t.complianceUpdateFromInternet}</p>
            <p class="text-xs text-slate-500 dark:text-slate-400 mt-0.5">{$t.complianceUpdateFromInternetDesc}</p>
          </div>
        </button>
        <button onclick={updateRulesFromFile}
          class="flex items-center gap-4 p-4 rounded-xl border border-slate-200 dark:border-slate-600 hover:border-primary hover:bg-primary/5 transition-all text-left">
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
