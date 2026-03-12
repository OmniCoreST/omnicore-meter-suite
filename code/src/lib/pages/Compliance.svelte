<script lang="ts">
  import { onMount, tick } from "svelte";
  import { t, isConnected, isMeterReading, navigationStore, addLog as addCommLog } from "$lib/stores";
  import { exportToPdf } from "$lib/utils/export";
  import { openPath } from "@tauri-apps/plugin-opener";
  import { complianceStore } from "$lib/stores/compliance";
  import type { ComplianceProfile, TestStep, CommunicationLog, SessionLog, ObisLine } from "$lib/stores/compliance";
  import { meterStore } from "$lib/stores/meter";
  import { connectionStore } from "$lib/stores/connection";
  import {
    checkComplianceV3,
    readShort,
    readFull,
    readLoadProfile,
    readPacket,
    readObisBatch,
    openComplianceRulesFile,
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
  } from "$lib/utils/tauri";
  import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
  import { warningToast, successToast, errorToast } from "$lib/stores/toast";

  import type { StepState, StepStatus } from "$lib/stores/compliance";

  // ─── State ───────────────────────────────────────────────────────────────────

  let reloading = $state(false);
  let updating = $state(false);
  let showUpdateModal = $state(false);
  let showLpWarning = $state(false);

  let profiles = $state<ComplianceProfile[]>([]);
  let selectedProfileId = $state("");
  let testSteps = $state<TestStep[]>([]);

  // running, stepStates, runLog are stored in complianceStore to survive navigation
  const running = $derived($complianceStore.running);
  const stepStates = $derived($complianceStore.stepStates);
  const runLog = $derived($complianceStore.runLog);

  // Yük profili şifre dialog'u
  let showLpPasswordDialog = $state(false);
  let lpPassword = $state("");
  let lpPasswordLevel = $state(1);
  let lpPasswordPromise: Promise<void> | null = null;
  let lpPasswordResolve: (() => void) | null = null;
  let pendingLpStepIndex = $state<number | null>(null);

  const result = $derived($complianceStore.result);

  // Faz tespiti
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

  // Profil otomatik seçimi
  $effect(() => {
    if (profiles.length > 0 && !selectedProfileId) {
      const defaultId = meterPhases === 1 ? "single_phase" : "three_phase_direct";
      selectedProfileId = profiles.find(p => p.id === defaultId)?.id ?? profiles[0].id;
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
      console.error("Compliance config yüklenemedi:", e);
    }
  });

  // ─── OBIS Satır Ayrıştırıcı ──────────────────────────────────────────────────

  function parseObisLines(raw: string): ObisLine[] {
    const lines: ObisLine[] = [];
    for (const line of raw.split("\n")) {
      const m = line.trim().match(/^([0-9A-Fa-f.]+(?:\*\d+)?)\(([^)]*)\)/);
      if (!m) continue;
      const code = m[1];
      const rawValue = m[2];
      const unitMatch = rawValue.match(/^(.*)\*([a-zA-ZkKMW%µ]+(?:Wh|Wh|Ah|Hz|var|VA)?)$/);
      if (unitMatch) {
        lines.push({ code, rawValue, value: unitMatch[1], unit: unitMatch[2] });
      } else {
        lines.push({ code, rawValue, value: rawValue, unit: null });
      }
    }
    return lines;
  }

  function makeSession(
    sessionType: "short_read" | "full_read" | "load_profile" | "obis_read",
    obisLines: ObisLine[],
    success: boolean,
    bccValid: boolean | null,
    modesUsed: string[],
    durationMs: number,
    error?: string,
  ): SessionLog {
    return {
      sessionType,
      timestamp: new Date().toISOString(),
      handshake: {
        requestSent: true,
        identificationReceived: success,
        identificationRaw: "",
        identificationFormatValid: success,
        ackSent: true,
        ackMode: null,
        ackBaudChar: null,
        baudNegotiationSuccess: success,
        initialBaud: 300,
        targetBaud: 9600,
        responseTimeMs: 0,
      },
      obisLines,
      protocolEvents: [],
      bccValid,
      etxFound: success,
      durationMs,
      success,
      error: error ?? null,
      modesUsed,
    };
  }

  // ─── Test Çalıştırıcı ─────────────────────────────────────────────────────────

  function addLog(msg: string) {
    const ts = new Date().toLocaleTimeString("tr-TR", { hour: "2-digit", minute: "2-digit", second: "2-digit" });
    complianceStore.addRunLog(`[${ts}] ${msg}`);
  }

  async function runFullTest() {
    if (!$isConnected || running) return;
    complianceStore.setRunning(true);
    meterStore.setReading(true);
    await tick();
    complianceStore.clearRunLog();
    complianceStore.setLoading();

    complianceStore.setStepStates(testSteps.map(step => ({
      step, status: "pending" as StepStatus, message: "", durationMs: 0,
    })));

    const identity = $connectionStore.meterIdentity;
    const sessions: SessionLog[] = [];
    let meterSerial = "";
    // flag set during short_read when we need to inject a serial from the
    // handshake because 0.0.0 was missing from the packet. Used to add a
    // non‑error info item after compliance check.
    let injectedSerialFallback = false;

    addLog(`Test başlatıldı — ${profiles.find(p => p.id === selectedProfileId)?.name ?? selectedProfileId}`);

    for (let i = 0; i < stepStates.length; i++) {
      const ss = stepStates[i];
      complianceStore.updateStepState(i, { status: "running", message: "Okuma yapılıyor..." });

      const t0 = performance.now();
      addLog(`Adım ${i + 1}/${stepStates.length}: ${ss.step.name}`);

      const upd = (patch: Partial<StepState>) => complianceStore.updateStepState(i, patch);
      let curStatus: StepStatus = "running";

      try {
        addCommLog("section", ss.step.name);
        switch (ss.step.mode) {
          case "short_read": {
            const data = await readShort();
            meterSerial = data.serialNumber;
            let obisLines = parseObisLines(data.rawData ?? "");
            if (obisLines.length === 0) {
              if (data.serialNumber) obisLines.push({ code: "0.0.0", rawValue: data.serialNumber, value: data.serialNumber, unit: null });
              if (data.meterDate) obisLines.push({ code: "0.9.2", rawValue: data.meterDate, value: data.meterDate, unit: null });
              if (data.meterTime) obisLines.push({ code: "0.9.1", rawValue: data.meterTime, value: data.meterTime, unit: null });
              obisLines.push({ code: "32.7.0", rawValue: `${data.voltageL1}*V`, value: String(data.voltageL1), unit: "V" });
              obisLines.push({ code: "14.7.0", rawValue: `${data.frequency}*Hz`, value: String(data.frequency), unit: "Hz" });
              obisLines.push({ code: "1.8.0", rawValue: `${data.activeEnergyImportTotal}*kWh`, value: String(data.activeEnergyImportTotal), unit: "kWh" });
              if (data.ffCode) obisLines.push({ code: "F.F.0", rawValue: data.ffCode, value: data.ffCode, unit: null });
            }
            if (!obisLines.find(o => o.code === "0.0.0") && data.serialNumber) {
              obisLines.push({ code: "0.0.0", rawValue: data.serialNumber, value: data.serialNumber, unit: null });
              injectedSerialFallback = true;
            }

            console.log("🔍 Short Read Debug:");
            console.log("  data.serialNumber:", data.serialNumber);
            console.log("  obisLines 0.0.0:", obisLines.find(o => o.code === "0.0.0"));
            console.log("  obisLines 96.1.0:", obisLines.find(o => o.code === "96.1.0"));
            console.log("  injectedSerialFallback:", injectedSerialFallback);
            console.log("  Total obisLines:", obisLines.length);
            const dur = Math.round(performance.now() - t0);
            sessions.push(makeSession("short_read", obisLines, true, null, ["6"], dur));
            const raw = data.rawData || "";
            const mType: "single-phase" | "three-phase" = (raw.includes("52.7.0") || raw.includes("72.7.0")) ? "three-phase" : "single-phase";
            meterStore.setShortReadData(data, mType, false);
            upd({ message: `${obisLines.length} OBIS satırı` });
            addLog(`  Kısa okuma tamamlandı — Seri: ${data.serialNumber}, ${obisLines.length} satır`);
            break;
          }

          case "full_read": {
            const data = await readFull();
            meterSerial = meterSerial || data.serialNumber;
            const obisLines = parseObisLines(data.rawData ?? "");
            const dur = Math.round(performance.now() - t0);
            sessions.push(makeSession("full_read", obisLines, true, null, ["0"], dur));
            const raw = data.rawData || "";
            const mType: "single-phase" | "three-phase" = (raw.includes("52.7.0") || raw.includes("72.7.0")) ? "three-phase" : "single-phase";
            meterStore.setShortReadData(data, mType, false);
            upd({ message: `${obisLines.length} OBIS satırı` });
            addLog(`  Tam okuma tamamlandı — ${obisLines.length} satır`);
            break;
          }

          case "load_profile": {
            try {
              const lp = await readLoadProfile(1, null, null, undefined, 1);
              const obisLines = parseObisLines(lp.rawData ?? "");
              const dur = Math.round(performance.now() - t0);
              sessions.push(makeSession("load_profile", obisLines, true, null, ["1"], dur));
              if (lp.entries?.length > 0) meterStore.setLoadProfileData(lp as any);
              upd({ message: `${lp.entries?.length ?? 0} kayıt` });
              addLog(`  Yük profili tamamlandı — ${lp.entries?.length ?? 0} kayıt`);
            } catch (e) {
              const errorStr = String(e);
              if (errorStr.includes("şifre") || errorStr.includes("password") || errorStr.includes("authentication")) {
                pendingLpStepIndex = i;
                showLpPasswordDialog = true;
                createLpPasswordPromise();
                upd({ status: "pending", message: "Şifre bekleniyor..." });
                curStatus = "pending";
                addLog(`  Yük profili şifre gerektiriyor — kullanıcı girişi bekleniyor`);
                await lpPasswordPromise;
                break;
              } else {
                const dur = Math.round(performance.now() - t0);
                sessions.push(makeSession("load_profile", [], false, null, ["1"], dur, errorStr));
                upd({ status: "failed", message: errorStr });
                curStatus = "failed";
                addLog(`  Yük profili başarısız: ${e}`);
                if (errorStr.includes("tüm stratejiler başarısız")) {
                  pendingLpStepIndex = i;
                  showLpPasswordDialog = true;
                  createLpPasswordPromise();
                  upd({ status: "pending", message: "Şifre bekleniyor..." });
                  curStatus = "pending";
                  addLog(`  Şifre isteneceği için şifre dialog'u açılıyor`);
                  await lpPasswordPromise;
                }
              }
            }
            break;
          }

          case "packet_read": {
            const pMode = ss.step.packetMode ?? 7;
            const pResult = await readPacket(pMode);
            const obisLines = parseObisLines(pResult.rawData ?? "");
            const dur = Math.round(performance.now() - t0);
            sessions.push(makeSession("obis_read", obisLines, true, pResult.bccValid, [String(pMode)], dur));
            upd({ message: `Mode ${pMode}: ${pResult.bytesRead} bayt, BCC: ${pResult.bccValid ? "OK" : "FAIL"}` });
            addLog(`  Paket (Mode ${pMode}) tamamlandı — ${pResult.bytesRead} bayt`);
            break;
          }

          case "obis_read": {
            const codes = ss.step.obisCodes ?? ["0.9.1", "0.9.2"];
            const values = await readObisBatch(codes);
            const obisLines: ObisLine[] = Object.entries(values).map(([code, v]) => ({
              code,
              rawValue: v,
              value: v.includes("*") ? v.split("*")[0] : v,
              unit: v.includes("*") ? v.split("*")[1] ?? null : null,
            }));
            const dur = Math.round(performance.now() - t0);
            sessions.push(makeSession("obis_read", obisLines, true, null, ["1"], dur));
            upd({ message: `${obisLines.length}/${codes.length} kod` });
            addLog(`  OBIS okuma tamamlandı — ${obisLines.length} kod`);
            break;
          }

          default:
            upd({ status: "skipped", message: `Bilinmeyen mod: ${ss.step.mode}` });
            curStatus = "skipped";
        }

        if (curStatus === "running") {
          upd({ status: "done" });
          curStatus = "done";
        }
      } catch (e) {
        upd({ status: "failed", message: String(e) });
        curStatus = "failed";
        addLog(`  HATA: ${e}`);
        const dur = Math.round(performance.now() - t0);
        const sType = (ss.step.mode === "short_read" ? "short_read"
          : ss.step.mode === "full_read" ? "full_read"
          : ss.step.mode === "load_profile" ? "load_profile"
          : "obis_read") as "short_read" | "full_read" | "load_profile" | "obis_read";
        sessions.push(makeSession(sType, [], false, null, [], dur, String(e)));
      }

      upd({ durationMs: Math.round(performance.now() - t0) });

      // Her adım bitiminde o ana kadar toplanan verilerle kısmi uyumluluk kontrolü yap
      if (sessions.length > 0 && curStatus !== "pending") {
        try {
          const partialLog: CommunicationLog = {
            meterSerial,
            meterManufacturer: identity?.manufacturer ?? "",
            meterModel: identity?.model ?? "",
            profileId: selectedProfileId,
            sessions: [...sessions],
          };
          const partialRes = await checkComplianceV3(partialLog, selectedProfileId);
          complianceStore.setResult(partialRes);
          addLog(`  → Kısmi kontrol: ${partialRes.errorCount} hata, ${partialRes.warningCount} uyarı, ${partialRes.infoCount} bilgi`);
        } catch (_) {
          // Kısmi kontrol hatası sessizce geç
        }
      }
    }

    // Veri karşılaştırması - farklı okuma türlerinden gelen OBIS verilerini kıyasla
    addLog("Veri karşılaştırması yapılıyor...");
    const shortReadSession = sessions.find(s => s.sessionType === "short_read");
    const fullReadSession = sessions.find(s => s.sessionType === "full_read");
    
    if (shortReadSession && fullReadSession) {
      const shortObisCodes = new Set(shortReadSession.obisLines.map(o => o.code));
      const fullObisCodes = new Set(fullReadSession.obisLines.map(o => o.code));
      
      const onlyInShort = [...shortObisCodes].filter(code => !fullObisCodes.has(code));
      const onlyInFull = [...fullObisCodes].filter(code => !shortObisCodes.has(code));
      const inBoth = [...shortObisCodes].filter(code => fullObisCodes.has(code));
      
      addLog(`  Kısa okumada: ${shortObisCodes.size} OBIS kodu`);
      addLog(`  Tam okumada: ${fullObisCodes.size} OBIS kodu`);
      addLog(`  Her ikisinde de var: ${inBoth.length} kod`);
      addLog(`  Sadece kısa okumada: ${onlyInShort.length} kod (${onlyInShort.slice(0, 5).join(', ')}${onlyInShort.length > 5 ? '...' : ''})`);
      addLog(`  Sadece tam okumada: ${onlyInFull.length} kod (${onlyInFull.slice(0, 5).join(', ')}${onlyInFull.length > 5 ? '...' : ''})`);
      
      // 0.0.0 kodunun durumunu özellikle kontrol et
      const shortHasSerial = shortObisCodes.has("0.0.0");
      const fullHasSerial = fullObisCodes.has("0.0.0");
      addLog(`  0.0.0 (seri numarası) - Kısa: ${shortHasSerial ? 'VAR' : 'YOK'}, Tam: ${fullHasSerial ? 'VAR' : 'YOK'}`);
      
      if (shortHasSerial !== fullHasSerial) {
        addLog(`  ⚠️  0.0.0 kodu farklı okumalarda farklı durum gösteriyor!`);
      }
    } else {
      addLog(`  Karşılaştırma için yeterli veri yok (kısa okuma: ${!!shortReadSession}, tam okuma: ${!!fullReadSession})`);
    }

    // CommunicationLog oluştur ve v3 uyumluluk kontrolü yap
    addLog("Uyumluluk kontrolü yapılıyor...");
    try {
      const log: CommunicationLog = {
        meterSerial,
        meterManufacturer: identity?.manufacturer ?? "",
        meterModel: identity?.model ?? "",
        profileId: selectedProfileId,
        sessions,
      };
      const res = await checkComplianceV3(log, selectedProfileId);
      // if we injected serial from handshake, still treat it as an ERROR
      // because the spec mandates 0.0.0 be present in the short read packet.
      // but keep the serial available for display by injecting earlier.
      if (injectedSerialFallback) {
        const err: import("$lib/stores/compliance").ComplianceIssue = {
          code: "ID-001b",
          category: "obis_existence",
          severity: "error",
          description: "Kısa okumada 0.0.0 OBIS satırı eksik olduğu için hata (sadece handshake'ten seri ulaşıldı).",
          expected: "0.0.0 kodunun short read paketinde yer alması",
          actual: "0.0.0 sadece handshake sırasında tespit edildi",
          specRef: "Şartname 3.1 - 0.0.0 zorunlu",
          cause: "Sayaç firmware'i short read paketinde seri numarası kodunu göndermiyor",
          remedy: "Firmware üreticisi ile iletişime geçin ve 0.0.0 OBIS kodunun short read çıktısına eklenmesini sağlayın.",
          sessionType: "short_read",
          obisCode: "0.0.0",
        };
        res.issues.push(err);
        res.errorCount += 1;
      }
      complianceStore.setResult(res);
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
    } finally {
      complianceStore.setRunning(false);
      meterStore.setReading(false);
      addLog("Test tamamlandı.");
    }
  }

  // ─── Kural Dosyası İşlemleri ─────────────────────────────────────────────────

  async function openRulesFile() {
    // prevent interaction during a running test
    if (running) return;
    if (!isTauri()) return;
    try {
      await openComplianceRulesFile();
    } catch (e) { errorToast(String(e)); }
  }

  async function reloadRules() {
    if (running) return;
    reloading = true;
    try { successToast(await reloadComplianceRules()); }
    catch (e) { errorToast(String(e)); }
    finally { reloading = false; }
  }

  async function updateRulesFromInternet() {
    if (running) return;
    showUpdateModal = false;
    updating = true;
    try { successToast(await updateComplianceRules()); }
    catch (e) { errorToast(String(e)); }
    finally { updating = false; }
  }

  async function updateRulesFromFile() {
    if (running) return;
    showUpdateModal = false;
    const selected = await openFileDialog({ filters: [{ name: "TOML", extensions: ["toml"] }], multiple: false });
    if (!selected) return;
    updating = true;
    try { successToast(await importComplianceRulesFromFile(selected)); }
    catch (e) { errorToast(String(e)); }
    finally { updating = false; }
  }

  // ─── Yük Profili Şifre Dialog ────────────────────────────────────────────────

  function createLpPasswordPromise() {
    lpPasswordPromise = new Promise<void>((resolve) => {
      lpPasswordResolve = resolve;
    });
  }

  function cancelLpPassword() {
    showLpPasswordDialog = false;
    lpPassword = "";
    lpPasswordLevel = 1;
    if (pendingLpStepIndex !== null) {
      complianceStore.updateStepState(pendingLpStepIndex, { status: "failed", message: "Şifre girilmedi" });
      pendingLpStepIndex = null;
    }
    if (lpPasswordResolve) {
      lpPasswordResolve();
      lpPasswordResolve = null;
      lpPasswordPromise = null;
    }
  }

  async function confirmLpPassword() {
    if (lpPassword.length !== 8 || !/^\d{8}$/.test(lpPassword)) {
      errorToast("Şifre tam olarak 8 rakam olmalıdır");
      return;
    }

    if (pendingLpStepIndex === null) return;
    const idx = pendingLpStepIndex;

    showLpPasswordDialog = false;
    pendingLpStepIndex = null;

    complianceStore.updateStepState(idx, { status: "running", message: "Şifre ile tekrar deneniyor..." });
    addLog(`  Yük profili şifre ile tekrar deneniyor (P${lpPasswordLevel})`);

    try {
      const t0 = performance.now();
      const lp = await readLoadProfile(1, null, null, lpPassword, lpPasswordLevel);
      const dur = Math.round(performance.now() - t0);
      if (lp.entries?.length > 0) meterStore.setLoadProfileData(lp as any);
      complianceStore.updateStepState(idx, { status: "done", message: `${lp.entries?.length ?? 0} kayıt`, durationMs: dur });
      addLog(`  Yük profili tamamlandı — ${lp.entries?.length ?? 0} kayıt`);
    } catch (e) {
      complianceStore.updateStepState(idx, { status: "failed", message: String(e) });
      addLog(`  Yük profili şifre ile başarısız: ${e}`);
    }

    lpPassword = "";
    lpPasswordLevel = 1;

    if (lpPasswordResolve) {
      lpPasswordResolve();
      lpPasswordResolve = null;
      lpPasswordPromise = null;
    }
  }

  // ─── Kural Yönetimi ──────────────────────────────────────────────────────────

  let showManageRules = $state(false);
  let managedRules = $state<ComplianceRuleDef[]>([]);
  let loadingRules = $state(false);
  let deletingCode = $state<string | null>(null);

  let showAddRule = $state(false);
  let editMode = $state(false);
  let saving = $state(false);

  // Kural form alanları (v3 OBIS tabanlı)
  let ruleCode = $state("");
  let ruleCategory = $state("obis_value");
  let ruleCheck = $state("range");
  let ruleSeverity = $state<"error" | "warning" | "info">("warning");
  let ruleObisCode = $state("");
  let ruleProfile = $state<0 | 1 | 3>(0);
  let ruleDesc = $state("");
  let ruleSpecRef = $state("");
  let ruleMin = $state("");
  let ruleMax = $state("");
  let ruleValue = $state("");
  let ruleBit = $state("");
  let ruleTolerance = $state("");
  let ruleMaxDrift = $state("");

  const CATEGORIES = [
    { value: "obis_existence", label: "OBIS Varlık" },
    { value: "obis_format",    label: "OBIS Format" },
    { value: "obis_value",     label: "OBIS Değer" },
    { value: "cross_value",    label: "Çapraz Değer" },
    { value: "protocol",       label: "Protokol" },
    { value: "session",        label: "Oturum" },
  ];

  const CHECK_BY_CATEGORY: Record<string, { value: string; label: string }[]> = {
    obis_existence: [{ value: "must_exist",      label: "Mevcut olmalı" }],
    obis_format:    [{ value: "value_format",     label: "Format kontrolü (regex)" }],
    obis_value: [
      { value: "range",         label: "Aralık (min–max)" },
      { value: "equals",        label: "Eşit" },
      { value: "not_equals",    label: "Eşit değil" },
      { value: "not_empty",     label: "Boş olmamalı" },
      { value: "bit_zero",      label: "Bit = 0" },
      { value: "bit_one",       label: "Bit = 1" },
      { value: "regex_match",   label: "Regex eşleşmesi" },
    ],
    cross_value: [
      { value: "tariff_balance",     label: "Tarife dengesi" },
      { value: "time_drift_minutes", label: "Saat sapması" },
    ],
    protocol: [
      { value: "handshake_complete",    label: "El sıkışma tamam" },
      { value: "baud_negotiation",      label: "Baud müzakeresi" },
      { value: "bcc_valid",             label: "BCC geçerli" },
      { value: "etx_present",           label: "ETX mevcut" },
      { value: "identification_format", label: "Kimlik formatı" },
      { value: "response_time",         label: "Yanıt süresi" },
    ],
    session: [
      { value: "must_succeed",  label: "Başarılı olmalı" },
      { value: "data_received", label: "Veri alınmalı" },
    ],
  };

  $effect(() => {
    const checks = CHECK_BY_CATEGORY[ruleCategory] ?? [];
    if (checks.length > 0 && !checks.find(c => c.value === ruleCheck)) {
      ruleCheck = checks[0].value;
    }
  });

  async function loadManagedRules() {
    loadingRules = true;
    try { managedRules = await listComplianceRules(); }
    catch (e) { errorToast(String(e)); }
    finally { loadingRules = false; }
  }

  async function openManageRules() {
    if (running) return;
    showManageRules = true;
    await loadManagedRules();
  }

  function openAddRuleModal(prefill?: ComplianceRuleDef) {
    editMode = !!prefill;
    ruleCode       = prefill?.code ?? "";
    ruleCategory   = prefill?.category ?? "obis_value";
    ruleCheck      = prefill?.check ?? "range";
    ruleSeverity   = (prefill?.severity as "error" | "warning" | "info") ?? "warning";
    ruleObisCode   = prefill?.obisCode ?? "";
    ruleProfile    = (prefill?.profile?.includes("three_phase_direct") ? 3
                    : prefill?.profile?.includes("single_phase") ? 1 : 0) as 0 | 1 | 3;
    ruleDesc       = prefill?.description ?? "";
    ruleSpecRef    = prefill?.specRef ?? "";
    ruleMin        = prefill?.min != null ? String(prefill.min) : "";
    ruleMax        = prefill?.max != null ? String(prefill.max) : "";
    ruleValue      = prefill?.value ?? "";
    ruleBit        = prefill?.bit != null ? String(prefill.bit) : "";
    ruleTolerance  = prefill?.tolerance != null ? String(prefill.tolerance) : "";
    ruleMaxDrift   = prefill?.maxDrift != null ? String(prefill.maxDrift) : "";
    showAddRule    = true;
  }

  async function saveRule() {
    if (!ruleCode.trim() || !ruleDesc.trim()) return;
    saving = true;
    try {
      const profile = ruleProfile === 3 ? ["three_phase_direct", "three_phase_ct"]
                    : ruleProfile === 1 ? ["single_phase"] : [];
      const rule: ComplianceRuleDef = {
        code: ruleCode.trim(),
        category: ruleCategory,
        check: ruleCheck,
        severity: ruleSeverity,
        description: ruleDesc.trim(),
        obisCode: ruleObisCode.trim() || null,
        obisCodes: [],
        specRef: ruleSpecRef.trim() || null,
        profile,
        sessionType: null,
        min: ruleCheck === "range" ? parseFloat(ruleMin) || null : null,
        max: ruleCheck === "range" ? parseFloat(ruleMax) || null : null,
        value: (ruleCheck === "equals" || ruleCheck === "not_equals") ? ruleValue : null,
        bit: (ruleCheck === "bit_zero" || ruleCheck === "bit_one") ? parseInt(ruleBit) : null,
        tolerance: ruleCheck === "tariff_balance" ? parseFloat(ruleTolerance) || null : null,
        maxDrift: ruleCheck === "time_drift_minutes" ? parseInt(ruleMaxDrift) || null : null,
        enabled: true,
        cause: null,
        remedy: null,
        field: null,
        phases: null,
      };

      if (editMode) {
        await updateComplianceRule(rule);
        successToast($t.complianceRuleUpdated);
      } else {
        let toml = `[[rules]]\n`;
        toml += `code = "${rule.code}"\ncategory = "${rule.category}"\ncheck = "${rule.check}"\nseverity = "${rule.severity}"\n`;
        toml += `description = "${rule.description}"\n`;
        if (rule.obisCode) toml += `obis_code = "${rule.obisCode}"\n`;
        if (rule.specRef)  toml += `spec_ref = "${rule.specRef}"\n`;
        if (profile.length > 0) toml += `profile = [${profile.map(p => `"${p}"`).join(", ")}]\n`;
        if (ruleCheck === "range") toml += `min = ${ruleMin}\nmax = ${ruleMax}\n`;
        else if (ruleCheck === "equals" || ruleCheck === "not_equals") toml += `value = "${ruleValue}"\n`;
        else if (ruleCheck === "bit_zero" || ruleCheck === "bit_one") toml += `bit = ${ruleBit}\n`;
        else if (ruleCheck === "tariff_balance") toml += `tolerance = ${ruleTolerance}\n`;
        else if (ruleCheck === "time_drift_minutes") toml += `max_drift = ${ruleMaxDrift}\n`;
        await addComplianceRule(toml);
        successToast($t.complianceRuleAdded);
      }
      await reloadComplianceRules();
      await loadManagedRules();
      showAddRule = false;
    } catch (e) { errorToast(String(e)); }
    finally { saving = false; }
  }

  async function confirmDelete(code: string) {
    if (!confirm($t.complianceDeleteConfirm + ` (${code})`)) return;
    deletingCode = code;
    try {
      await deleteComplianceRule(code);
      successToast($t.complianceRuleDeleted);
      await loadManagedRules();
      await reloadComplianceRules();
    } catch (e) { errorToast(String(e)); }
    finally { deletingCode = null; }
  }

  // ─── Gösterim Yardımcıları ────────────────────────────────────────────────────

  function formatTime(iso: string): string {
    try { return new Date(iso).toLocaleTimeString("tr-TR", { hour: "2-digit", minute: "2-digit", second: "2-digit" }); }
    catch { return iso; }
  }

  function severityIcon(s: string)   { return s === "error" ? "cancel" : s === "warning" ? "warning" : "info"; }
  function severityText(s: string)   { return s === "error" ? "text-red-500" : s === "warning" ? "text-yellow-500" : "text-blue-400"; }
  function severityBorder(s: string) { return s === "error" ? "bg-red-500/10 border-red-500/20" : s === "warning" ? "bg-yellow-500/10 border-yellow-500/20" : "bg-blue-500/10 border-blue-500/20"; }
  function severityDot(s: string)    { return s === "error" ? "bg-red-500" : s === "warning" ? "bg-yellow-500" : "bg-blue-400"; }

  function specSources(specRef: string): string[] {
    return specRef.split(" / ").map(part => part.trim().replace(/^TEDAS(\s|§)/, "TEDAŞ$1"));
  }

  function stepIcon(mode: string): string {
    return ({ short_read: "electric_meter", full_read: "description", load_profile: "bar_chart", packet_read: "inventory_2", obis_read: "schedule" } as Record<string,string>)[mode] ?? "help";
  }
  function statusIcon(s: StepStatus): string {
    return ({ pending: "radio_button_unchecked", running: "autorenew", done: "check_circle", failed: "error", skipped: "skip_next" } as Record<string,string>)[s];
  }
  function statusColor(s: StepStatus): string {
    return ({ pending: "text-slate-400", running: "text-primary", done: "text-emerald-500", failed: "text-red-500", skipped: "text-yellow-500" } as Record<string,string>)[s];
  }

  const completedSteps = $derived(stepStates.filter(s => s.status === "done" || s.status === "failed" || s.status === "skipped").length);
  const progressPct    = $derived(stepStates.length > 0 ? Math.round((completedSteps / stepStates.length) * 100) : 0);

  let activeFilter = $state<"error" | "warning" | "info" | null>(null);

  function toggleFilter(sev: "error" | "warning" | "info") {
    activeFilter = activeFilter === sev ? null : sev;
  }

  const sortedIssues   = $derived(
    result?.issues.slice().sort((a, b) => {
      const order: Record<string, number> = { error: 0, warning: 1, info: 2 };
      return (order[a.severity] ?? 3) - (order[b.severity] ?? 3);
    }) ?? []
  );

  const filteredIssues = $derived(
    activeFilter ? sortedIssues.filter(i => i.severity === activeFilter) : sortedIssues
  );

  // ─── PDF Export ──────────────────────────────────────────────────────────────

  function sev2label(s: string) { return s === "error" ? "HATA" : s === "warning" ? "UYARI" : "BİLGİ"; }

  function generateReportHtml(bw: boolean = false): string {
    if (!result) return "";
    const checkedAt = (() => { try { return new Date(result.checkedAt).toLocaleString("tr-TR"); } catch { return result.checkedAt; } })();
    const identity = $connectionStore.meterIdentity;
    const meterInfo = identity ? (identity.manufacturer + " " + identity.model).trim() : "";
    const passedCount = result.totalRulesChecked - result.errorCount - result.warningCount - result.infoCount;

    // ─── Renk paleti ───
    const c = bw ? {
      title: '#000', heading: '#000', body: '#1a1a1a', muted: '#555', faint: '#666',
      headerBorder: '#000', sectionBorder: '#999', rowBorder: '#ccc', thBg: '#eee',
      errBg: '#fff', errBorder: '#000', errBorderW: '2px', errNum: '#000',
      wrnBg: '#fff', wrnBorder: '#666', wrnBorderW: '1px', wrnNum: '#000',
      infBg: '#fff', infBorder: '#999', infBorderW: '1px', infNum: '#000',
      okBg: '#fff', okBorder: '#333', okNum: '#000',
      cardBorder: '2px solid #333', cardBg: '#fff',
      successClr: '#000', failClr: '#000', expectedClr: '#000', actualClr: '#000',
      sevErrClr: '#000', sevWrnClr: '#000', sevInfClr: '#000',
      footerBorder: '#999', footerClr: '#666',
      alertErrBg: '#fff', alertErrBorder: '2px solid #333', alertErrClr: '#000',
      alertWrnBg: '#fff', alertWrnBorder: '1px solid #999', alertWrnClr: '#333',
      alertInfBg: '#fff', alertInfBorder: '1px solid #999', alertInfClr: '#333',
      metaSpanBg: '#f5f5f5', metaSpanBorder: '1px solid #ddd',
      allPassBg: '#fff', allPassBorder: '2px solid #333', allPassClr: '#000',
    } : {
      title: '#1e3a5f', heading: '#1e3a5f', body: '#1e293b', muted: '#64748b', faint: '#94a3b8',
      headerBorder: '#279EA7', sectionBorder: '#e2e8f0', rowBorder: '#e2e8f0', thBg: '#f1f5f9',
      errBg: '#fef2f2', errBorder: '#fecaca', errBorderW: '1px', errNum: '#ef4444',
      wrnBg: '#fffbeb', wrnBorder: '#fde68a', wrnBorderW: '1px', wrnNum: '#f59e0b',
      infBg: '#eff6ff', infBorder: '#bfdbfe', infBorderW: '1px', infNum: '#3b82f6',
      okBg: '#f0fdf4', okBorder: '#bbf7d0', okNum: '#16a34a',
      cardBorder: '', cardBg: '',
      successClr: '#16a34a', failClr: '#ef4444', expectedClr: '#16a34a', actualClr: '#dc2626',
      sevErrClr: '#ef4444', sevWrnClr: '#f59e0b', sevInfClr: '#3b82f6',
      footerBorder: '#e2e8f0', footerClr: '#94a3b8',
      alertErrBg: '#fef2f2', alertErrBorder: '1px solid #fecaca', alertErrClr: '#dc2626',
      alertWrnBg: '#fffbeb', alertWrnBorder: '1px solid #fde68a', alertWrnClr: '#92400e',
      alertInfBg: '#eff6ff', alertInfBorder: '1px solid #bfdbfe', alertInfClr: '#1d4ed8',
      metaSpanBg: '#f1f5f9', metaSpanBorder: 'none',
      allPassBg: '#f0fdf4', allPassBorder: '1px solid #bbf7d0', allPassClr: '#16a34a',
    };

    // ─── Test Adımları HTML ───
    const stepsHtml = stepStates.length > 0
      ? '<div style="margin-bottom:14px;" class="avoid-break">'
        + '<h2 style="font-size:14px;color:' + c.heading + ';margin-bottom:6px;border-bottom:1px solid ' + c.sectionBorder + ';padding-bottom:4px;">Test Adımları</h2>'
        + '<table style="width:100%;border-collapse:collapse;font-size:11px;table-layout:fixed;">'
        + '<colgroup><col style="width:30px;"><col><col style="width:75px;"><col style="width:60px;"></colgroup>'
        + '<thead><tr style="background:' + c.thBg + ';">'
        + '<th style="padding:5px 6px;text-align:left;border-bottom:2px solid ' + c.sectionBorder + ';">#</th>'
        + '<th style="padding:5px 6px;text-align:left;border-bottom:2px solid ' + c.sectionBorder + ';">Adım</th>'
        + '<th style="padding:5px 6px;text-align:center;border-bottom:2px solid ' + c.sectionBorder + ';">Durum</th>'
        + '<th style="padding:5px 6px;text-align:right;border-bottom:2px solid ' + c.sectionBorder + ';">Süre</th>'
        + '</tr></thead><tbody>'
        + stepStates.map((ss, i) => {
          const statusLabel = bw
            ? (ss.status === "done" ? "✓ Başarılı" : ss.status === "failed" ? "✗ Başarısız" : ss.status === "skipped" ? "— Atlandı" : ss.status === "running" ? "⟳ Çalışıyor" : "○ Bekliyor")
            : (ss.status === "done" ? "Başarılı" : ss.status === "failed" ? "Başarısız" : ss.status === "skipped" ? "Atlandı" : ss.status === "running" ? "Çalışıyor" : "Bekliyor");
          const statusClr = bw ? '#000' : (ss.status === "done" ? '#16a34a' : ss.status === "failed" ? '#ef4444' : ss.status === "skipped" ? '#f59e0b' : '#64748b');
          const rowBg = bw ? '#fff' : (ss.status === "done" ? '#f0fdf4' : ss.status === "failed" ? '#fef2f2' : ss.status === "skipped" ? '#fffbeb' : '#fff');
          return '<tr style="border-bottom:1px solid ' + c.rowBorder + ';background:' + rowBg + ';">'
            + '<td style="padding:5px 6px;color:' + c.muted + ';font-weight:600;">' + (i + 1) + '</td>'
            + '<td style="padding:5px 6px;font-weight:500;color:' + c.body + ';">' + ss.step.name
            + (ss.message ? '<div style="font-size:9px;color:' + c.faint + ';margin-top:2px;">' + ss.message + '</div>' : '')
            + '</td>'
            + '<td style="padding:5px 6px;text-align:center;color:' + statusClr + ';font-weight:700;font-size:10px;">' + statusLabel + '</td>'
            + '<td style="padding:5px 6px;text-align:right;font-family:monospace;color:' + c.muted + ';font-size:10px;">' + (ss.durationMs > 0 ? (ss.durationMs / 1000).toFixed(1) + 's' : '—') + '</td>'
            + '</tr>';
        }).join("")
        + '</tbody></table></div>'
      : '';

    // ─── Session Sonuçları HTML ───
    const sessionsHtml = result.sessionResults && result.sessionResults.length > 0
      ? '<div style="margin-bottom:14px;" class="avoid-break">'
        + '<h2 style="font-size:14px;color:' + c.heading + ';margin-bottom:6px;border-bottom:1px solid ' + c.sectionBorder + ';padding-bottom:4px;">Oturum Sonuçları</h2>'
        + '<table style="width:100%;border-collapse:collapse;font-size:11px;table-layout:fixed;">'
        + '<colgroup><col style="width:35%;"><col style="width:18%;"><col style="width:15%;"><col style="width:15%;"><col style="width:17%;"></colgroup>'
        + '<thead><tr style="background:' + c.thBg + ';">'
        + '<th style="padding:5px 6px;text-align:left;border-bottom:2px solid ' + c.sectionBorder + ';">Oturum</th>'
        + '<th style="padding:5px 6px;text-align:center;border-bottom:2px solid ' + c.sectionBorder + ';">Durum</th>'
        + '<th style="padding:5px 6px;text-align:center;border-bottom:2px solid ' + c.sectionBorder + ';">OBIS</th>'
        + '<th style="padding:5px 6px;text-align:center;border-bottom:2px solid ' + c.sectionBorder + ';">Sorun</th>'
        + '<th style="padding:5px 6px;text-align:right;border-bottom:2px solid ' + c.sectionBorder + ';">Süre</th>'
        + '</tr></thead><tbody>'
        + result.sessionResults.map(sr => {
          const label = ({ short_read: "Kısa Okuma", full_read: "Tam Okuma", load_profile: "Yük Profili", obis_read: "OBIS Okuma", packet_read: "Paket Okuma" } as Record<string, string>)[sr.sessionType] ?? sr.sessionType;
          const rowBg = bw ? '#fff' : (sr.success ? '#f0fdf4' : '#fef2f2');
          const statusClr = bw ? '#000' : (sr.success ? '#16a34a' : '#ef4444');
          const issueClr = bw ? '#000' : (sr.issues.length > 0 ? '#ef4444' : '#16a34a');
          return '<tr style="border-bottom:1px solid ' + c.rowBorder + ';background:' + rowBg + ';">'
            + '<td style="padding:5px 6px;font-weight:500;color:' + c.body + ';">' + label + '</td>'
            + '<td style="padding:5px 6px;text-align:center;color:' + statusClr + ';font-weight:700;font-size:10px;">' + (bw ? (sr.success ? '✓ ' : '✗ ') : '') + (sr.success ? 'Başarılı' : 'Başarısız') + '</td>'
            + '<td style="padding:5px 6px;text-align:center;font-family:monospace;color:' + c.body + ';">' + sr.obisCount + '</td>'
            + '<td style="padding:5px 6px;text-align:center;font-family:monospace;color:' + issueClr + ';font-weight:' + (sr.issues.length > 0 ? '700' : '400') + ';">' + sr.issues.length + '</td>'
            + '<td style="padding:5px 6px;text-align:right;font-family:monospace;color:' + c.muted + ';">' + (sr.durationMs / 1000).toFixed(1) + 's</td>'
            + '</tr>';
        }).join("")
        + '</tbody></table></div>'
      : '';

    // ─── Kural Durumu Uyarısı ───
    const alertBoxBase = 'border-radius:6px;padding:10px 12px;margin-bottom:12px;font-size:12px;line-height:1.4;';
    let rulesStatusHtml = '';
    if (result.rulesStatus === "tooOld") {
      rulesStatusHtml = '<div style="' + alertBoxBase + 'background:' + c.alertErrBg + ';border:' + c.alertErrBorder + ';color:' + c.alertErrClr + ';font-weight:600;">' + (bw ? '[!] ' : '⚠ ') + 'Kural dosyası çok eski — sonuçlar güncel olmayabilir. Lütfen kuralları güncelleyin.</div>';
    } else if (result.rulesStatus === "offline") {
      rulesStatusHtml = '<div style="' + alertBoxBase + 'background:' + c.alertWrnBg + ';border:' + c.alertWrnBorder + ';color:' + c.alertWrnClr + ';">' + (bw ? '[!] ' : '⚠ ') + 'Güncelleme sunucusuna ulaşılamadı — mevcut kurallarla devam edildi.</div>';
    } else if (result.latestVersion && result.latestVersion !== result.configVersion) {
      rulesStatusHtml = '<div style="' + alertBoxBase + 'background:' + c.alertInfBg + ';border:' + c.alertInfBorder + ';color:' + c.alertInfClr + ';">' + (bw ? '[i] ' : 'ℹ ') + 'Yeni kural sürümü mevcut: v' + result.latestVersion + '</div>';
    }

    // ─── İhlaller ───
    const issueCards = sortedIssues.map(issue => {
      const sevLabel = sev2label(issue.severity);
      const sevColor = bw ? '#000' : (issue.severity === "error" ? c.sevErrClr : issue.severity === "warning" ? c.sevWrnClr : c.sevInfClr);
      const sevBg    = bw ? '#fff' : (issue.severity === "error" ? c.errBg : issue.severity === "warning" ? c.wrnBg : c.infBg);
      const sevBorder = bw
        ? (issue.severity === "error" ? c.errBorderW + ' solid ' + c.errBorder : issue.severity === "warning" ? c.wrnBorderW + ' solid ' + c.wrnBorder : c.infBorderW + ' solid ' + c.infBorder)
        : ('1px solid ' + (issue.severity === "error" ? c.errBorder : issue.severity === "warning" ? c.wrnBorder : c.infBorder));
      const sevDivider = bw ? '#ccc' : (issue.severity === "error" ? c.errBorder : issue.severity === "warning" ? c.wrnBorder : c.infBorder);
      const sevPrefix = bw ? (issue.severity === "error" ? '✗ ' : issue.severity === "warning" ? '! ' : 'i ') : '';

      const detailParts: string[] = [];
      detailParts.push('<span style="font-family:monospace;color:' + (bw ? '#1a1a1a' : '#334155') + ';font-weight:600;">' + issue.code + '</span>');
      if (issue.obisCode) detailParts.push('<span style="font-family:monospace;color:' + c.muted + ';">' + issue.obisCode + '</span>');

      let card = '<div class="avoid-break" style="border:' + sevBorder + ';border-radius:' + (bw ? '4' : '6') + 'px;margin-bottom:8px;">'
        + '<div style="padding:8px 10px;background:' + sevBg + ';">'
        + '<div style="margin-bottom:3px;">'
        + '<span style="color:' + sevColor + ';font-weight:800;font-size:13px;text-transform:uppercase;">' + sevPrefix + sevLabel + '</span>'
        + (issue.specRef ? '<span style="color:' + (bw ? '#999' : sevDivider) + ';margin:0 5px;">|</span><span style="font-size:9px;font-weight:400;color:' + (bw ? '#555' : sevColor) + ';' + (bw ? '' : 'opacity:0.7;') + '">' + issue.specRef + '</span>' : '')
        + '</div>'
        + '<div style="font-size:10px;margin-bottom:4px;color:' + c.muted + ';">'
        + detailParts.join(' &nbsp;·&nbsp; ')
        + '</div>'
        + '<div style="font-size:12px;font-weight:600;color:' + (bw ? '#000' : '#1e293b') + ';line-height:1.4;">' + issue.description + '</div>';

      if (issue.expected || issue.actual) {
        card += '<div style="margin-top:4px;font-family:monospace;font-size:10px;word-break:break-all;">'
          + (issue.expected ? '<span style="color:' + c.expectedClr + ';' + (bw ? 'font-weight:700;' : '') + '">' + (bw ? 'Beklenen: ' : '<span style="font-weight:600;">Beklenen:</span> ') + issue.expected + '</span>' : '')
          + (issue.expected && issue.actual ? ' &nbsp;&nbsp; ' : '')
          + (issue.actual   ? '<span style="color:' + c.actualClr + ';' + (bw ? 'font-weight:700;' : '') + '">' + (bw ? 'Gerçek: ' : '<span style="font-weight:600;">Gerçek:</span> ') + issue.actual + '</span>' : '')
          + '</div>';
      }
      card += '</div>';

      if (issue.cause || issue.remedy) {
        card += '<div style="padding:8px 10px;font-size:11px;color:' + (bw ? '#333' : '#475569') + ';line-height:1.5;border-top:1px solid ' + sevDivider + ';">';
        if (issue.cause) {
          card += '<div style="margin-bottom:' + (issue.remedy ? '4px' : '0') + ';"><span style="font-weight:700;color:' + (bw ? '#000' : '#334155') + ';">Neden:</span> ' + issue.cause + '</div>';
        }
        if (issue.remedy) {
          card += '<div><span style="font-weight:700;color:' + (bw ? '#000' : '#334155') + ';">Düzeltme:</span> ' + issue.remedy + '</div>';
        }
        card += '</div>';
      }

      card += '</div>';
      return card;
    }).join("");

    const issuesHtml = sortedIssues.length === 0
      ? '<div style="text-align:center;padding:24px;color:' + c.allPassClr + ';font-size:14px;font-weight:700;background:' + c.allPassBg + ';border:' + c.allPassBorder + ';border-radius:' + (bw ? '4' : '8') + 'px;">✓ Tüm kontroller başarıyla geçildi</div>'
      : '<div style="margin-bottom:14px;">'
        + '<h2 style="font-size:14px;color:' + c.heading + ';margin-bottom:6px;border-bottom:1px solid ' + c.sectionBorder + ';padding-bottom:4px;">Tespit Edilen Sorunlar (' + sortedIssues.length + ')</h2>'
        + issueCards + '</div>';

    // ─── Sayfa Birleştirme ───
    // NOTE: html2canvas has poor flexbox support. Use table-based layouts
    // for summary cards and avoid flex wherever possible so the PDF renders
    // correctly without clipping or missing elements.
    return '<!DOCTYPE html>'
      + '<ht'+'ml lang="tr"><head><meta charset="UTF-8"><title>TEDAŞ Uyumluluk Raporu</title>'
      + '<sty'+'le>'
      + '* { box-sizing: border-box; word-wrap: break-word; overflow-wrap: break-word; }'
      + 'h1, h2, h3, p, ul, ol, figure { margin: 0; padding: 0; }'
      + 'body { font-family: Arial, Helvetica, sans-serif; font-size: 12px; color: ' + c.body + '; background: #fff; padding: 0 0 20px 0; width: 100%; max-width: 100%; }'
      + 'table { table-layout: fixed; width: 100%; border-collapse: collapse; } td, th { word-wrap: break-word; overflow-wrap: break-word; }'
      + 'span[style*="monospace"] { word-break: break-all; }'
      + ('@'+'page { size: A4; margin: 10mm; }')
      + 'h1 { font-size: 18px; color: ' + c.title + '; }'
      + 'h2 { font-size: 14px; color: ' + c.heading + '; }'
      + '.header { border-bottom: 2px solid ' + c.headerBorder + '; padding-bottom: 10px; margin-bottom: 14px; }'
      + '.meta { margin-top: 8px; font-size: 11px; color: ' + (bw ? '#333' : '#475569') + '; }'
      + '.meta span { background: ' + c.metaSpanBg + '; padding: 3px 8px !important; border-radius: 4px; display: inline-block; margin: 2px 4px 2px 0; line-height: 1.3; border: ' + c.metaSpanBorder + '; }'
      + '.summary-table { width: 100%; border-collapse: separate; border-spacing: 8px 0; margin-bottom: 14px; table-layout: fixed; }'
      + '.summary-table td { border-radius: ' + (bw ? '4' : '8') + 'px; padding: 14px 8px !important; text-align: center; vertical-align: middle; width: 25%; }'
      + '.num { font-size: 24px; font-weight: 800; display: block; margin: 0; padding: 0; line-height: 1.2; }'
      + '.lbl { font-size: 10px; color: ' + c.muted + '; display: block; margin: 4px 0 0 0; padding: 0; line-height: 1.2; }'
      + (bw ? '' : 'td.err { background: ' + c.errBg + '; border: 1px solid ' + c.errBorder + '; } td.wrn { background: ' + c.wrnBg + '; border: 1px solid ' + c.wrnBorder + '; } td.inf { background: ' + c.infBg + '; border: 1px solid ' + c.infBorder + '; } td.ok { background: ' + c.okBg + '; border: 1px solid ' + c.okBorder + '; }')
      + (bw ? '' : 'td.err .num { color: ' + c.errNum + '; } td.wrn .num { color: ' + c.wrnNum + '; } td.inf .num { color: ' + c.infNum + '; } td.ok .num { color: ' + c.okNum + '; }')
      + '.footer { margin-top: 20px; padding-top: 8px; padding-bottom: 10px; border-top: 1px solid ' + c.footerBorder + '; font-size: 10px; color: ' + c.footerClr + '; text-align: right; page-break-inside: avoid; break-inside: avoid; }'
      + '.avoid-break { page-break-inside: avoid; break-inside: avoid; }'
      + '</sty'+'le></head><body>'
      // ─── Başlık ───
      + '<div class="header">'
      + '<h1>' + (bw
          ? (result.errorCount > 0 ? '[BAŞARISIZ] ' : result.warningCount > 0 ? '[UYARI] ' : '[BAŞARILI] ')
          : (result.errorCount > 0 ? '⛔ ' : result.warningCount > 0 ? '⚠️ ' : '✅ ')
        ) + 'TEDAŞ Uyumluluk Raporu</h1>'
      + (() => {
        const rows: [string, string][] = [];
        if (meterInfo) rows.push(['Sayaç', meterInfo]);
        rows.push(['Profil', result.profileName + ' (' + meterPhases + ' Faz)']);
        rows.push(['Tarih', checkedAt as string]);
        rows.push(['Kural Sürümü', 'v' + result.configVersion]);
        rows.push(['Kontrol', result.totalRulesChecked + ' kural kontrol edildi']);
        const th = 'style="padding:3px 10px 3px 0;text-align:left;font-size:11px;color:' + (bw ? '#555' : '#94a3b8') + ';font-weight:' + (bw ? '700' : '600') + ';white-space:nowrap;border:none;"';
        const td = 'style="padding:3px 0;text-align:left;font-size:11px;color:' + (bw ? '#1a1a1a' : '#475569') + ';font-weight:400;border:none;"';
        return '<table style="margin-top:8px;border-collapse:collapse;table-layout:auto;width:auto;">'
          + rows.map(([k, v]) => '<tr><td ' + th + '>' + k + '</td><td ' + td + '>' + v + '</td></tr>').join('')
          + '</table></div>';
      })()
      // ─── Kural durumu uyarısı ───
      + rulesStatusHtml
      // ─── Özet kartları (inline styles to survive global reset) ───
      + (() => {
        if (bw) {
          const cardStyle = 'style="border-radius:4px;padding:14px 8px;text-align:center;vertical-align:middle;width:25%;background:#fff;border:2px solid #333;"';
          const numStyle = 'style="font-size:24px;font-weight:800;display:block;margin:0;padding:0;line-height:1.2;color:#000;"';
          const lblStyle = 'style="font-size:10px;color:#333;display:block;margin:4px 0 0 0;padding:0;line-height:1.2;font-weight:600;"';
          return '<table style="width:100%;border-collapse:separate;border-spacing:8px 0;margin-bottom:14px;table-layout:fixed;"><tr>'
            + '<td ' + cardStyle + '><span ' + numStyle + '>' + result.errorCount + '</span><span ' + lblStyle + '>Hata</span></td>'
            + '<td ' + cardStyle + '><span ' + numStyle + '>' + result.warningCount + '</span><span ' + lblStyle + '>Uyarı</span></td>'
            + '<td ' + cardStyle + '><span ' + numStyle + '>' + result.infoCount + '</span><span ' + lblStyle + '>Bilgi</span></td>'
            + '<td ' + cardStyle + '><span ' + numStyle + '>' + passedCount + '</span><span ' + lblStyle + '>Geçen</span></td>'
            + '</tr></table>';
        }
        const cardStyle = (bg: string, border: string) =>
          'style="border-radius:8px;padding:14px 8px;text-align:center;vertical-align:middle;width:25%;background:' + bg + ';border:1px solid ' + border + ';"';
        const numStyle = (color: string) =>
          'style="font-size:24px;font-weight:800;display:block;margin:0;padding:0;line-height:1.2;color:' + color + ';"';
        const lblStyle = 'style="font-size:10px;color:#64748b;display:block;margin:4px 0 0 0;padding:0;line-height:1.2;"';

        const errCard = result.errorCount > 0
          ? '<td ' + cardStyle(c.errBg, c.errBorder) + '><span ' + numStyle(c.errNum) + '>' + result.errorCount + '</span><span ' + lblStyle + '>Hata</span></td>'
          : '<td ' + cardStyle(c.okBg, c.okBorder) + '><span ' + numStyle(c.okNum) + '>0</span><span ' + lblStyle + '>Hata</span></td>';
        const wrnCard = result.warningCount > 0
          ? '<td ' + cardStyle(c.wrnBg, c.wrnBorder) + '><span ' + numStyle(c.wrnNum) + '>' + result.warningCount + '</span><span ' + lblStyle + '>Uyarı</span></td>'
          : '<td ' + cardStyle(c.okBg, c.okBorder) + '><span ' + numStyle(c.okNum) + '>0</span><span ' + lblStyle + '>Uyarı</span></td>';
        const infCard = '<td ' + cardStyle(c.infBg, c.infBorder) + '><span ' + numStyle(c.infNum) + '>' + result.infoCount + '</span><span ' + lblStyle + '>Bilgi</span></td>';
        const okCard  = '<td ' + cardStyle(c.okBg, c.okBorder) + '><span ' + numStyle(c.okNum) + '>' + passedCount + '</span><span ' + lblStyle + '>Geçen</span></td>';

        return '<table style="width:100%;border-collapse:separate;border-spacing:8px 0;margin-bottom:14px;table-layout:fixed;"><tr>'
          + errCard + wrnCard + infCard + okCard + '</tr></table>';
      })()
      // ─── Test adımları ───
      + stepsHtml
      // ─── Session sonuçları ───
      + sessionsHtml
      // ─── İhlaller ───
      + issuesHtml
      // ─── Footer ───
      + '<div class="footer">OmniCore Meter Suite · TEDAŞ MLZ/2017-062.B · ' + checkedAt + '</div>'
      + '</body></ht'+'ml>';
  }

  let showPdfModeModal = $state(false);

  async function exportPdf(bw: boolean = false) {
    if (!result) return;
    showPdfModeModal = false;

    try {
      const html = generateReportHtml(bw);
      const now = new Date();
      const ts = `${now.getFullYear()}-${String(now.getMonth()+1).padStart(2,'0')}-${String(now.getDate()).padStart(2,'0')}_${String(now.getHours()).padStart(2,'0')}-${String(now.getMinutes()).padStart(2,'0')}`;
      const modeSuffix = bw ? '-SB' : '-Renkli';
      const filePath = await exportToPdf(html, `OmniCore-Uyumluluk-Raporu-${ts}${modeSuffix}`);

      if (filePath) {
        const fileName = filePath.split(/[\\/]/).pop() ?? filePath;
        successToast(`PDF kaydedildi: ${fileName}`, 8000, {
          label: "PDF'i Aç",
          onClick: () => { openPath(filePath).catch(() => {}); }
        });
      }
    } catch (error) {
      console.error("PDF oluşturma hatası:", error);
      errorToast("PDF oluşturulamadı");
    }
  }
</script>

<div class="space-y-6 transition-all duration-200" class:blur-sm={showLpPasswordDialog || showLpWarning || showUpdateModal || showAddRule || showManageRules || showPdfModeModal}>

  <!-- ─── Bölüm 1: Başlık Çubuğu ────────────────────────────────────────────── -->
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
      <button onclick={openRulesFile} disabled={running}
        class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium border border-slate-200 dark:border-slate-600 text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700 transition-all disabled:opacity-50">
        <span class="material-symbols-outlined text-sm">open_in_new</span>
        {$t.complianceOpenRules}
      </button>
      <button onclick={reloadRules} disabled={running || reloading}
        class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium border border-slate-200 dark:border-slate-600 text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700 transition-all disabled:opacity-50">
        <span class="material-symbols-outlined text-sm {reloading ? 'animate-spin-reverse' : ''}">refresh</span>
        {reloading ? $t.complianceReloading : $t.complianceReload}
      </button>
      <button onclick={() => showUpdateModal = true} disabled={running || updating}
        class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium border border-slate-200 dark:border-slate-600 text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700 transition-all disabled:opacity-50">
        <span class="material-symbols-outlined text-sm {updating ? 'animate-spin-reverse' : ''}">cloud_download</span>
        {updating ? $t.complianceUpdating : $t.complianceUpdate}
      </button>
      <button onclick={openManageRules} disabled={running}
        class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium border border-slate-200 dark:border-slate-600 text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700 transition-all disabled:opacity-50">
        <span class="material-symbols-outlined text-sm">rule_settings</span>
        {$t.complianceManageRules ?? "Kuralları Yönet"}
      </button>
    </div>
  </div>

  <!-- Bağlı değil uyarısı -->
  {#if !$isConnected}
    <div class="rounded-xl border border-dashed border-slate-300 dark:border-slate-600 p-8 text-center space-y-4">
      <span class="material-symbols-outlined text-5xl text-slate-300 dark:text-slate-600 block">sensors_off</span>
      <div>
        <p class="text-slate-600 dark:text-slate-400 font-medium">Sayaca bağlı değilsiniz</p>
        <p class="text-sm text-slate-500 dark:text-slate-500 mt-1">Uyumluluk testi için önce bir sayaca bağlanın.</p>
      </div>
      <button
        onclick={() => navigationStore.navigate("dashboard")}
        class="inline-flex items-center gap-2 px-5 py-2.5 bg-primary hover:bg-primary/90 text-white font-bold rounded-lg transition-all">
        <span class="material-symbols-outlined text-base">cable</span>
        Bağlantı Sayfasına Git
      </button>
    </div>

  {:else}

    <!-- ─── Bölüm 2: Test Çalıştırıcı ──────────────────────────────────────────── -->
    <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl shadow-sm overflow-hidden">
      <!-- Çalıştırıcı Başlığı -->
      <div class="bg-gradient-to-r from-primary/10 to-emerald-500/10 dark:from-primary/20 dark:to-emerald-500/20 px-6 py-4 border-b border-slate-200 dark:border-[#334a5e]">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-3">
            <div class="p-2 bg-primary/10 rounded-lg">
              <span class="material-symbols-outlined text-primary">play_circle</span>
            </div>
            <div>
              <h3 class="text-base font-bold text-slate-900 dark:text-white">Test Çalıştır</h3>
              <p class="text-xs text-slate-500 dark:text-slate-400">
                {testSteps.length} adımlı TEDAŞ uyumluluk testi
              </p>
            </div>
          </div>

          <div class="flex items-center gap-3">
            <!-- Profil seçici -->
            <div class="flex items-center gap-2">
              <span class="text-xs text-slate-500">Profil:</span>
              <select
                bind:value={selectedProfileId}
                disabled={running}
                class="px-3 py-1.5 text-xs font-medium rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-[#1a2632] text-slate-900 dark:text-white focus:border-primary focus:ring-1 focus:ring-primary transition-colors disabled:opacity-50">
                {#each profiles as p}
                  <option value={p.id}>{p.name}</option>
                {/each}
              </select>
            </div>

            <!-- Faz göstergesi -->
            <div class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg border border-slate-200 dark:border-slate-600 text-xs font-medium text-slate-600 dark:text-slate-300">
              <span class="material-symbols-outlined text-sm text-primary">electric_bolt</span>
              {meterPhases} Faz
            </div>

            <!-- Başlat butonu -->
            <button
              onclick={runFullTest}
              disabled={running || !$isConnected || $isMeterReading}
              class="flex items-center gap-2 px-5 py-2.5 rounded-lg font-bold text-sm transition-all
                {running
                  ? 'bg-primary/20 text-primary cursor-wait'
                  : 'bg-primary text-white hover:bg-primary/90 active:scale-95 shadow-lg shadow-primary/20'}">
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

      <!-- Test Adımları -->
      <div class="p-6">
        {#if stepStates.length === 0 && !running}
          {#if result}
            <!-- Önceki test tamamlanmış: özet göster -->
            <div class="flex items-center gap-4 p-4 rounded-lg border
              {result.errorCount > 0 ? 'bg-red-500/5 border-red-500/20' : result.warningCount > 0 ? 'bg-amber-500/5 border-amber-500/20' : 'bg-emerald-500/5 border-emerald-500/20'}">
              <span class="material-symbols-outlined text-2xl
                {result.errorCount > 0 ? 'text-red-500' : result.warningCount > 0 ? 'text-amber-500' : 'text-emerald-500'}">
                {result.errorCount > 0 ? 'cancel' : result.warningCount > 0 ? 'warning' : 'check_circle'}
              </span>
              <div class="flex-1">
                <p class="text-sm font-medium text-slate-700 dark:text-slate-200">
                  Son test tamamlandı — {result.errorCount} hata, {result.warningCount} uyarı, {result.infoCount} bilgi
                </p>
                <p class="text-xs text-slate-400 mt-0.5">
                  {(() => { try { return new Date(result.checkedAt).toLocaleString("tr-TR"); } catch { return result.checkedAt; } })()}
                  · {result.totalRulesChecked} kural kontrol edildi
                </p>
              </div>
              <span class="text-xs text-slate-400">Tekrar çalıştırmak için "Testi Başlat"a tıklayın</span>
            </div>
          {:else}
            <!-- Test henüz hiç başlamadı: planı göster -->
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
          {/if}

        {:else}
          <!-- Test sırasında / sonrasında: adım durumları -->

          <!-- İlerleme çubuğu -->
          {#if running}
            <div class="mb-5">
              <div class="flex items-center justify-between mb-1.5">
                <span class="text-xs font-medium text-slate-500">İlerleme</span>
                <span class="text-xs font-bold text-primary">{progressPct}%</span>
              </div>
              <div class="w-full h-2 bg-slate-100 dark:bg-slate-700 rounded-full overflow-hidden">
                <div
                  class="h-full bg-gradient-to-r from-primary to-emerald-400 rounded-full transition-all duration-500"
                  style="width: {progressPct}%">
                </div>
              </div>
            </div>
          {/if}

          <div class="space-y-2">
            {#each stepStates as ss}
              <div class="group flex items-center gap-4 p-3 rounded-lg border transition-all duration-200 hover:-translate-y-0.5 hover:shadow-md cursor-default
                {ss.status === 'running'  ? 'bg-primary/5 border-primary/20 ring-1 ring-primary/10 hover:ring-2 hover:shadow-primary/10'
                : ss.status === 'done'    ? 'bg-emerald-500/5 border-emerald-500/15 hover:bg-emerald-500/10 hover:border-emerald-500/30 hover:shadow-emerald-500/10'
                : ss.status === 'failed'  ? 'bg-red-500/5 border-red-500/15 hover:bg-red-500/10 hover:border-red-500/30 hover:shadow-red-500/10'
                : 'bg-slate-50 dark:bg-[#0f1821] border-slate-200 dark:border-[#334a5e] hover:bg-white dark:hover:bg-slate-800/60 hover:border-slate-300 dark:hover:border-slate-600'}">

                <div class="flex items-center justify-center w-8 h-8 rounded-full flex-shrink-0 transition-transform duration-200 group-hover:scale-110
                  {ss.status === 'done' ? 'bg-emerald-500/10' : ss.status === 'failed' ? 'bg-red-500/10' : ss.status === 'running' ? 'bg-primary/10' : 'bg-slate-100 dark:bg-slate-700'}">
                  <span class="material-symbols-outlined text-base {statusColor(ss.status)} {ss.status === 'running' ? 'animate-spin-reverse' : ''}">
                    {statusIcon(ss.status)}
                  </span>
                </div>

                <span class="material-symbols-outlined transition-transform duration-200 group-hover:scale-110 {ss.status === 'running' ? 'text-primary' : 'text-slate-400 group-hover:text-slate-600 dark:group-hover:text-slate-200'}">
                  {stepIcon(ss.step.mode)}
                </span>

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

                {#if ss.durationMs > 0}
                  <span class="text-xs font-mono text-slate-400 flex-shrink-0 transition-all duration-200 group-hover:text-slate-600 dark:group-hover:text-slate-200 group-hover:font-bold">
                    {(ss.durationMs / 1000).toFixed(1)}s
                  </span>
                {/if}
              </div>
            {/each}
          </div>

          <!-- Test günlüğü (daraltılabilir) -->
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

    <!-- ─── Bölüm 3: Sonuçlar ───────────────────────────────────────────────── -->
    {#if result}
      <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl shadow-sm overflow-hidden">
        <!-- Sonuç başlığı -->
        <div class="px-6 py-4 border-b border-slate-200 dark:border-[#334a5e]">
          <div class="flex items-center justify-between gap-3 flex-wrap">
            <div class="flex items-center gap-3">
              <span class="material-symbols-outlined text-xl {result.errorCount > 0 ? 'text-red-500' : result.warningCount > 0 ? 'text-yellow-500' : 'text-emerald-500'}">
                {result.errorCount > 0 ? 'gpp_bad' : result.warningCount > 0 ? 'gpp_maybe' : 'verified_user'}
              </span>
              <div>
                <h3 class="text-base font-bold text-slate-900 dark:text-white">Kontrol Sonuçları</h3>
                <div class="flex flex-wrap gap-1.5 mt-1.5">
                  {#if $connectionStore.meterIdentity}
                    <span class="inline-flex items-center text-xs bg-slate-100 dark:bg-slate-700/60 text-slate-600 dark:text-slate-300 px-2 py-1 rounded-md">
                      Sayaç: {$connectionStore.meterIdentity.manufacturer} {$connectionStore.meterIdentity.model}
                    </span>
                  {/if}
                  <span class="inline-flex items-center text-xs bg-slate-100 dark:bg-slate-700/60 text-slate-600 dark:text-slate-300 px-2 py-1 rounded-md">
                    Profil: {result.profileName} ({meterPhases} Faz)
                  </span>
                  <span class="inline-flex items-center text-xs bg-slate-100 dark:bg-slate-700/60 text-slate-600 dark:text-slate-300 px-2 py-1 rounded-md">
                    Tarih: {formatTime(result.checkedAt)}
                  </span>
                  <span class="inline-flex items-center text-xs bg-slate-100 dark:bg-slate-700/60 text-slate-600 dark:text-slate-300 px-2 py-1 rounded-md">
                    Kural sürümü: v{result.configVersion}
                  </span>
                  <span class="inline-flex items-center text-xs bg-slate-100 dark:bg-slate-700/60 text-slate-600 dark:text-slate-300 px-2 py-1 rounded-md">
                    {result.totalRulesChecked} kural kontrol edildi
                  </span>
                </div>
              </div>
            </div>
            <div class="flex items-center gap-2">
              <button
                onclick={(e) => {
                  if (running) {
                    const btn = e.currentTarget as HTMLElement;
                    btn.classList.remove('animate-shake-glow');
                    void btn.offsetWidth;
                    btn.classList.add('animate-shake-glow');
                    warningToast("Lütfen testin bitmesini bekleyin.", 3000);
                    return;
                  }
                  showPdfModeModal = true;
                }}
                class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium border transition-all
                  {running
                    ? 'bg-slate-100 dark:bg-slate-800 text-slate-400 dark:text-slate-500 border-slate-200 dark:border-slate-700 cursor-not-allowed opacity-60'
                    : 'bg-primary/10 hover:bg-primary/20 text-primary border-primary/20'}">
                <span class="material-symbols-outlined text-sm">picture_as_pdf</span>
                PDF Raporu
              </button>
            </div>
          </div>
        </div>

        <div class="p-6 space-y-5">
          <!-- Kural dosyası durum uyarıları -->
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

          <!-- Özet kartları (filtre olarak da çalışır) -->
          <div class="grid grid-cols-3 gap-3">
            <!-- Hata kartı -->
            <button
              onclick={() => result.errorCount > 0 && toggleFilter('error')}
              class="group rounded-xl p-4 border text-left transition-all duration-200 hover:scale-[1.03] hover:shadow-lg
                {result.errorCount === 0 ? 'cursor-default opacity-70' : 'cursor-pointer active:scale-95'}
                {activeFilter === 'error'
                  ? 'bg-red-500/20 border-red-500/50 ring-2 ring-red-500/40 shadow-lg shadow-red-500/15'
                  : result.errorCount > 0
                    ? 'bg-red-500/10 border-red-500/20 hover:shadow-red-500/15 hover:border-red-500/40'
                    : 'bg-slate-50 dark:bg-slate-800/50 border-slate-200 dark:border-slate-700'}">
              <div class="flex items-center gap-3">
                <span class="material-symbols-outlined text-2xl transition-transform duration-200 group-hover:scale-125 {result.errorCount > 0 ? 'text-red-500' : 'text-slate-300 dark:text-slate-600'}">
                  {result.errorCount > 0 ? 'cancel' : 'check_circle'}
                </span>
                <div>
                  <p class="text-2xl font-bold transition-all duration-200 group-hover:text-3xl {result.errorCount > 0 ? 'text-red-500' : 'text-slate-400'}">{result.errorCount}</p>
                  <p class="text-xs text-slate-500">{$t.complianceErrors}</p>
                </div>
              </div>
              {#if activeFilter === 'error'}
                <p class="text-[10px] text-red-400 mt-1.5 font-medium">Filtre aktif · tekrar tıkla iptal et</p>
              {:else if result.errorCount > 0}
                <p class="text-[10px] text-slate-400 mt-1.5 opacity-0 group-hover:opacity-100 transition-opacity">Sadece hataları göster</p>
              {/if}
            </button>

            <!-- Uyarı kartı -->
            <button
              onclick={() => result.warningCount > 0 && toggleFilter('warning')}
              class="group rounded-xl p-4 border text-left transition-all duration-200 hover:scale-[1.03] hover:shadow-lg
                {result.warningCount === 0 ? 'cursor-default opacity-70' : 'cursor-pointer active:scale-95'}
                {activeFilter === 'warning'
                  ? 'bg-yellow-500/20 border-yellow-500/50 ring-2 ring-yellow-500/40 shadow-lg shadow-yellow-500/15'
                  : result.warningCount > 0
                    ? 'bg-yellow-500/10 border-yellow-500/20 hover:shadow-yellow-500/15 hover:border-yellow-500/40'
                    : 'bg-slate-50 dark:bg-slate-800/50 border-slate-200 dark:border-slate-700'}">
              <div class="flex items-center gap-3">
                <span class="material-symbols-outlined text-2xl transition-transform duration-200 group-hover:scale-125 {result.warningCount > 0 ? 'text-yellow-500' : 'text-slate-300 dark:text-slate-600'}">
                  {result.warningCount > 0 ? 'warning' : 'check_circle'}
                </span>
                <div>
                  <p class="text-2xl font-bold transition-all duration-200 group-hover:text-3xl {result.warningCount > 0 ? 'text-yellow-500' : 'text-slate-400'}">{result.warningCount}</p>
                  <p class="text-xs text-slate-500">{$t.complianceWarnings}</p>
                </div>
              </div>
              {#if activeFilter === 'warning'}
                <p class="text-[10px] text-yellow-500 mt-1.5 font-medium">Filtre aktif · tekrar tıkla iptal et</p>
              {:else if result.warningCount > 0}
                <p class="text-[10px] text-slate-400 mt-1.5 opacity-0 group-hover:opacity-100 transition-opacity">Sadece uyarıları göster</p>
              {/if}
            </button>

            <!-- Bilgi kartı -->
            <button
              onclick={() => result.infoCount > 0 && toggleFilter('info')}
              class="group rounded-xl p-4 border text-left transition-all duration-200 hover:scale-[1.03] hover:shadow-lg hover:shadow-blue-500/10
                {result.infoCount === 0 ? 'cursor-default opacity-70' : 'cursor-pointer active:scale-95'}
                {activeFilter === 'info'
                  ? 'bg-blue-500/20 border-blue-500/50 ring-2 ring-blue-500/40 shadow-lg shadow-blue-500/15'
                  : result.infoCount > 0
                    ? 'bg-slate-50 dark:bg-slate-800/50 border-slate-200 dark:border-slate-700 hover:border-blue-400/30'
                    : 'bg-slate-50 dark:bg-slate-800/50 border-slate-200 dark:border-slate-700'}">
              <div class="flex items-center gap-3">
                <span class="material-symbols-outlined text-2xl text-blue-400 transition-transform duration-200 group-hover:scale-125">info</span>
                <div>
                  <p class="text-2xl font-bold text-slate-400 transition-all duration-200 group-hover:text-3xl">{result.infoCount}</p>
                  <p class="text-xs text-slate-500">Bilgi</p>
                </div>
              </div>
              {#if activeFilter === 'info'}
                <p class="text-[10px] text-blue-400 mt-1.5 font-medium">Filtre aktif · tekrar tıkla iptal et</p>
              {:else if result.infoCount > 0}
                <p class="text-[10px] text-slate-400 mt-1.5 opacity-0 group-hover:opacity-100 transition-opacity">Sadece bilgileri göster</p>
              {/if}
            </button>
          </div>

          <!-- Tümü geçti -->
          {#if sortedIssues.length === 0}
            <div class="group rounded-xl border border-emerald-500/20 bg-emerald-500/10 p-6 text-center transition-all duration-300 hover:scale-[1.01] hover:shadow-xl hover:shadow-emerald-500/20 hover:border-emerald-500/40 cursor-default">
              <span class="material-symbols-outlined text-4xl text-emerald-500 mb-2 block transition-transform duration-300 group-hover:scale-125 group-hover:rotate-12">verified</span>
              <p class="font-bold text-emerald-600 dark:text-emerald-400">{$t.complianceAllPassed}</p>
              <p class="text-sm text-slate-500 mt-1">{$t.complianceAllPassedDesc}</p>
            </div>
          {:else}
            <!-- Filtre durumu -->
            {#if activeFilter}
              <div class="px-3 py-2 rounded-lg bg-slate-100 dark:bg-slate-800 border border-slate-200 dark:border-slate-700">
                <span class="text-xs text-slate-500">
                  <span class="font-semibold {activeFilter === 'error' ? 'text-red-500' : activeFilter === 'warning' ? 'text-yellow-500' : 'text-blue-400'}">
                    {filteredIssues.length}
                  </span>
                  {activeFilter === 'error' ? ' hata' : activeFilter === 'warning' ? ' uyarı' : ' bilgi'} gösteriliyor
                  ({sortedIssues.length} toplam)
                </span>
              </div>
            {/if}

            <!-- İhlal listesi -->
            <div class="space-y-2">
              {#each filteredIssues as issue (issue.code + (activeFilter ?? ''))}
                <div class="group rounded-xl border overflow-hidden transition-all duration-200 hover:-translate-y-0.5 hover:shadow-lg cursor-default {severityBorder(issue.severity)}
                  {issue.severity === 'error' ? 'hover:shadow-red-500/10 hover:border-red-400/40' : issue.severity === 'warning' ? 'hover:shadow-yellow-500/10 hover:border-yellow-400/40' : 'hover:shadow-blue-500/10 hover:border-blue-400/40'}">
                  <div class="grid grid-cols-2 divide-x divide-slate-200/60 dark:divide-slate-700/60">

                    <!-- SOL: kural bilgisi -->
                    <div class="p-4 flex items-start gap-3 transition-colors duration-200
                      {issue.severity === 'error' ? 'group-hover:bg-red-500/5' : issue.severity === 'warning' ? 'group-hover:bg-yellow-500/5' : 'group-hover:bg-blue-500/5'}">
                      <span class="material-symbols-outlined text-xl mt-0.5 flex-shrink-0 transition-transform duration-200 group-hover:scale-125 {severityText(issue.severity)}">
                        {severityIcon(issue.severity)}
                      </span>
                      <div class="min-w-0">
                        <div class="flex items-center gap-1.5 mb-1.5 flex-wrap">
                          <span class="font-mono text-xs font-bold {severityText(issue.severity)}">{issue.code}</span>
                          {#if issue.obisCode}
                            <span class="text-[10px] bg-slate-200 dark:bg-slate-700 text-slate-600 dark:text-slate-300 px-1.5 py-0.5 rounded font-mono">
                              {issue.obisCode}
                            </span>
                          {/if}
                          {#if issue.specRef}
                            {#each specSources(issue.specRef) as src}
                              <span class="text-[10px] bg-slate-100 dark:bg-slate-700/60 text-slate-500 dark:text-slate-400 px-1.5 py-0.5 rounded-full font-medium">
                                {src}
                              </span>
                            {/each}
                          {/if}
                        </div>
                        <p class="text-sm font-medium text-slate-700 dark:text-slate-200 leading-relaxed">
                          {issue.description}
                        </p>
                        {#if issue.expected || issue.actual}
                          <div class="mt-2 flex gap-3 text-xs font-mono">
                            {#if issue.expected}
                              <span class="text-emerald-600 dark:text-emerald-400">Beklenen: {issue.expected}</span>
                            {/if}
                            {#if issue.actual}
                              <span class="text-red-500">Gerçek: {issue.actual}</span>
                            {/if}
                          </div>
                        {/if}
                      </div>
                    </div>

                    <!-- SAĞ: neden & düzeltme -->
                    <div class="p-4 flex flex-col gap-3 bg-black/[0.02] dark:bg-white/[0.02] transition-colors duration-200 group-hover:bg-black/[0.04] dark:group-hover:bg-white/[0.04]">
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
                        <div class="{issue.cause ? 'border-t border-slate-200/60 dark:border-slate-700/60 pt-3' : ''}">
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

<!-- ─── Kural Güncelleme Modalı ──────────────────────────────────────────────── -->
{#if showUpdateModal}
  <div
    role="presentation"
    class="fixed inset-0 z-40 flex items-center justify-center p-4 bg-black/75 backdrop-blur-lg"
    onclick={(e) => { if (e.target === e.currentTarget) showUpdateModal = false; }}
    onkeydown={(e) => { if (e.key === "Escape") showUpdateModal = false; }}
  >
    <div class="w-full max-w-sm bg-white dark:bg-slate-800 rounded-2xl shadow-2xl border border-slate-200 dark:border-slate-700 overflow-hidden">
      <div class="flex items-center justify-between px-5 py-4 border-b border-slate-200 dark:border-slate-700">
        <h3 class="font-bold text-slate-900 dark:text-white">Kural Dosyasını Güncelle</h3>
        <button onclick={() => showUpdateModal = false} class="text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 transition-colors">
          <span class="material-symbols-outlined">close</span>
        </button>
      </div>
      <div class="p-5 space-y-3">
        <button onclick={updateRulesFromInternet}
          class="w-full flex items-center gap-3 p-3 rounded-lg border border-slate-200 dark:border-slate-600 hover:bg-slate-50 dark:hover:bg-slate-700 transition-all text-left">
          <span class="material-symbols-outlined text-primary">cloud_download</span>
          <div>
            <p class="text-sm font-medium text-slate-900 dark:text-white">İnternetten İndir</p>
            <p class="text-xs text-slate-500">Sunucudan güncel kural dosyasını indir</p>
          </div>
        </button>
        <button onclick={updateRulesFromFile}
          class="w-full flex items-center gap-3 p-3 rounded-lg border border-slate-200 dark:border-slate-600 hover:bg-slate-50 dark:hover:bg-slate-700 transition-all text-left">
          <span class="material-symbols-outlined text-slate-500">upload_file</span>
          <div>
            <p class="text-sm font-medium text-slate-900 dark:text-white">Dosyadan İçe Aktar</p>
            <p class="text-xs text-slate-500">Yerel .toml dosyasını seç</p>
          </div>
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- ─── Kural Ekle / Düzenle Modalı ─────────────────────────────────────────── -->
{#if showAddRule}
  <div
    role="presentation"
    class="fixed inset-0 z-40 flex items-center justify-center p-4 bg-black/75 backdrop-blur-lg"
    onclick={(e) => { if (e.target === e.currentTarget) showAddRule = false; }}
    onkeydown={(e) => { if (e.key === "Escape") showAddRule = false; }}
  >
    <div class="w-full max-w-md bg-white dark:bg-slate-800 rounded-2xl shadow-2xl border border-slate-200 dark:border-slate-700 overflow-hidden">
      <div class="flex items-center justify-between px-5 py-4 border-b border-slate-200 dark:border-slate-700">
        <div class="flex items-center gap-2">
          <span class="material-symbols-outlined text-primary text-xl">{editMode ? "edit" : "add_circle"}</span>
          <h3 class="font-bold text-slate-900 dark:text-white">
            {editMode ? $t.complianceEditRule + ": " + ruleCode : $t.complianceAddRuleTitle}
          </h3>
        </div>
        <button onclick={() => showAddRule = false} class="text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 transition-colors">
          <span class="material-symbols-outlined">close</span>
        </button>
      </div>

      <div class="px-5 py-4 space-y-4 max-h-[70vh] overflow-y-auto">
        <!-- Temel alanlar -->
        <div class="grid grid-cols-2 gap-3">
          <div>
            <label for="rule-code" class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1">Kural Kodu *</label>
            <input id="rule-code" type="text" bind:value={ruleCode} placeholder="EL-005"
              class="w-full px-3 py-2 text-sm rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-700 text-slate-900 dark:text-white placeholder-slate-400 focus:outline-none focus:ring-2 focus:ring-primary/50" />
          </div>
          <div>
            <label for="rule-severity" class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1">Şiddet</label>
            <select id="rule-severity" bind:value={ruleSeverity}
              class="w-full px-3 py-2 text-sm rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-700 text-slate-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-primary/50">
              <option value="error">Hata</option>
              <option value="warning">Uyarı</option>
              <option value="info">Bilgi</option>
            </select>
          </div>
        </div>

        <!-- Profil -->
        <fieldset class="border-0 p-0 m-0">
          <legend class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1">Profil</legend>
          <div class="flex items-center rounded-lg border border-slate-200 dark:border-slate-600 overflow-hidden text-xs font-medium w-fit">
            <button type="button" onclick={() => ruleProfile = 0}
              class="px-3 py-2 transition-colors {ruleProfile === 0 ? 'bg-primary text-white' : 'text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700'}">Tümü</button>
            <button type="button" onclick={() => ruleProfile = 1}
              class="px-3 py-2 transition-colors {ruleProfile === 1 ? 'bg-primary text-white' : 'text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700'}">1 Faz</button>
            <button type="button" onclick={() => ruleProfile = 3}
              class="px-3 py-2 transition-colors {ruleProfile === 3 ? 'bg-primary text-white' : 'text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700'}">3 Faz</button>
          </div>
        </fieldset>

        <!-- Kontrol ayarları -->
        <div class="border-t border-slate-100 dark:border-slate-700 pt-3 space-y-3">
          <p class="text-xs font-semibold text-slate-400 dark:text-slate-500 uppercase tracking-wide">Kontrol</p>

          <div class="grid grid-cols-2 gap-3">
            <div>
              <label for="rule-category" class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1">Kategori</label>
              <select id="rule-category" bind:value={ruleCategory}
                class="w-full px-3 py-2 text-sm rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-700 text-slate-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-primary/50">
                {#each CATEGORIES as cat}
                  <option value={cat.value}>{cat.label}</option>
                {/each}
              </select>
            </div>
            <div>
              <label for="rule-check" class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1">Kontrol Tipi</label>
              <select id="rule-check" bind:value={ruleCheck}
                class="w-full px-3 py-2 text-sm rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-700 text-slate-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-primary/50">
                {#each CHECK_BY_CATEGORY[ruleCategory] ?? [] as chk}
                  <option value={chk.value}>{chk.label}</option>
                {/each}
              </select>
            </div>
          </div>

          <!-- OBIS Kodu -->
          {#if ruleCategory === "obis_value" || ruleCategory === "obis_format"}
            <div>
              <label for="rule-obis" class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1">OBIS Kodu</label>
              <input id="rule-obis" type="text" bind:value={ruleObisCode} placeholder="32.7.0"
                class="w-full px-3 py-2 text-sm rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-700 text-slate-900 dark:text-white font-mono placeholder-slate-400 focus:outline-none focus:ring-2 focus:ring-primary/50" />
            </div>
          {/if}

          <!-- Kontrol tipi parametreleri -->
          {#if ruleCheck === "range"}
            <div class="grid grid-cols-2 gap-3">
              <div>
                <label for="rule-min" class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1">Min</label>
                <input id="rule-min" type="number" bind:value={ruleMin} placeholder="90.0"
                  class="w-full px-3 py-2 text-sm rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-700 text-slate-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-primary/50" />
              </div>
              <div>
                <label for="rule-max" class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1">Max</label>
                <input id="rule-max" type="number" bind:value={ruleMax} placeholder="265.0"
                  class="w-full px-3 py-2 text-sm rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-700 text-slate-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-primary/50" />
              </div>
            </div>
          {:else if ruleCheck === "equals" || ruleCheck === "not_equals"}
            <div>
              <label for="rule-value" class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1">Değer</label>
              <input id="rule-value" type="text" bind:value={ruleValue} placeholder="15"
                class="w-full px-3 py-2 text-sm rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-700 text-slate-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-primary/50" />
            </div>
          {:else if ruleCheck === "bit_zero" || ruleCheck === "bit_one"}
            <div>
              <label for="rule-bit" class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1">Bit Numarası (0–63)</label>
              <input id="rule-bit" type="number" bind:value={ruleBit} placeholder="0" min="0" max="63"
                class="w-full px-3 py-2 text-sm rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-700 text-slate-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-primary/50" />
            </div>
          {:else if ruleCheck === "tariff_balance"}
            <div>
              <label for="rule-tolerance" class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1">Tolerans (kWh)</label>
              <input id="rule-tolerance" type="number" bind:value={ruleTolerance} placeholder="0.01" step="0.001"
                class="w-full px-3 py-2 text-sm rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-700 text-slate-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-primary/50" />
            </div>
          {:else if ruleCheck === "time_drift_minutes"}
            <div>
              <label for="rule-maxdrift" class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1">Maks. Sapma (dakika)</label>
              <input id="rule-maxdrift" type="number" bind:value={ruleMaxDrift} placeholder="5" min="1"
                class="w-full px-3 py-2 text-sm rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-700 text-slate-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-primary/50" />
            </div>
          {/if}
        </div>

        <!-- Açıklama -->
        <div class="border-t border-slate-100 dark:border-slate-700 pt-3 space-y-3">
          <p class="text-xs font-semibold text-slate-400 dark:text-slate-500 uppercase tracking-wide">Açıklama</p>
          <div>
            <label for="rule-desc" class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1">Açıklama *</label>
            <input id="rule-desc" type="text" bind:value={ruleDesc} placeholder="Kural açıklaması..."
              class="w-full px-3 py-2 text-sm rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-700 text-slate-900 dark:text-white placeholder-slate-400 focus:outline-none focus:ring-2 focus:ring-primary/50" />
          </div>
          <div>
            <label for="rule-specref" class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1">Şartname Referansı</label>
            <input id="rule-specref" type="text" bind:value={ruleSpecRef} placeholder="TEDAŞ Şartname 2.2.2"
              class="w-full px-3 py-2 text-sm rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-700 text-slate-900 dark:text-white placeholder-slate-400 focus:outline-none focus:ring-2 focus:ring-primary/50" />
          </div>
        </div>
      </div>

      <div class="flex items-center justify-end gap-3 px-5 py-4 border-t border-slate-200 dark:border-slate-700">
        <button onclick={() => showAddRule = false}
          class="px-4 py-2 rounded-lg text-sm font-medium text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700 transition-all">
          İptal
        </button>
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

<!-- ─── Kuralları Yönet Modalı ───────────────────────────────────────────────── -->
{#if showManageRules}
  <div
    role="presentation"
    class="fixed inset-0 z-40 flex items-center justify-center p-4 bg-black/75 backdrop-blur-lg"
    onclick={(e) => { if (e.target === e.currentTarget) showManageRules = false; }}
    onkeydown={(e) => { if (e.key === "Escape") showManageRules = false; }}
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
                <div class="flex items-center gap-2">
                  <span class="font-mono text-xs font-bold text-slate-700 dark:text-slate-200">{rule.code}</span>
                  <span class="text-[10px] text-slate-400 bg-slate-100 dark:bg-slate-700 px-1.5 py-0.5 rounded-full">{rule.category}</span>
                  {#if rule.obisCode}
                    <span class="text-[10px] text-primary font-mono bg-primary/10 px-1.5 py-0.5 rounded-full">{rule.obisCode}</span>
                  {/if}
                </div>
                <p class="text-xs text-slate-500 dark:text-slate-400 truncate mt-0.5">{rule.description}</p>
              </div>
              <div class="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                <button onclick={() => { showManageRules = false; openAddRuleModal(rule); }}
                  class="p-1.5 rounded-lg text-slate-400 hover:text-primary hover:bg-primary/10 transition-colors">
                  <span class="material-symbols-outlined text-sm">edit</span>
                </button>
                <button onclick={() => confirmDelete(rule.code)} disabled={deletingCode === rule.code}
                  class="p-1.5 rounded-lg text-slate-400 hover:text-red-500 hover:bg-red-500/10 transition-colors disabled:opacity-50">
                  <span class="material-symbols-outlined text-sm">{deletingCode === rule.code ? 'autorenew' : 'delete'}</span>
                </button>
              </div>
            </div>
          {/each}
        {/if}
      </div>
    </div>
  </div>
{/if}

<!-- ─── Yük Profili Uyarı Modalı ─────────────────────────────────────────────── -->
{#if showLpWarning}
  <div
    role="presentation"
    class="fixed inset-0 z-40 flex items-center justify-center p-4 bg-black/75 backdrop-blur-lg"
    onclick={(e) => { if (e.target === e.currentTarget) showLpWarning = false; }}
    onkeydown={(e) => { if (e.key === "Escape") showLpWarning = false; }}
  >
    <div class="w-full max-w-md bg-white dark:bg-slate-800 rounded-2xl shadow-2xl border border-slate-200 dark:border-slate-700 overflow-hidden">
      <div class="flex items-center gap-3 px-5 py-4 border-b border-amber-500/20 bg-amber-500/5">
        <span class="material-symbols-outlined text-2xl text-amber-500">warning</span>
        <h3 class="font-bold text-slate-900 dark:text-white">Yük Profili Okunamadı</h3>
        <button onclick={() => showLpWarning = false} class="ml-auto text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 transition-colors">
          <span class="material-symbols-outlined">close</span>
        </button>
      </div>
      <div class="px-5 py-4 space-y-3">
        <p class="text-sm text-slate-700 dark:text-slate-300 leading-relaxed">
          Sayaç yük profili okuma sırasında <strong>şifre istiyor</strong> olabilir. Eğer şifre girdiyseniz, girdiğiniz şifre <strong>yanlış veya yetersiz</strong> olabilir.
        </p>
        <div class="flex items-start gap-2 p-3 rounded-lg bg-blue-500/10 border border-blue-500/20">
          <span class="material-symbols-outlined text-blue-500 text-base mt-0.5">info</span>
          <p class="text-xs text-blue-600 dark:text-blue-400 leading-relaxed">
            Bazı sayaçlar yük profili okumak için P1 şifresi ister. Doğru şifreyi üst kısımdaki şifre alanına girip tekrar deneyebilirsiniz. Dikkat: Yanlış şifre 3 kez girilirse sayaç optik portunu kilitleyebilir.
          </p>
        </div>
      </div>
      <div class="flex justify-end px-5 py-3 border-t border-slate-200 dark:border-slate-700">
        <button onclick={() => showLpWarning = false}
          class="px-4 py-2 rounded-lg text-sm font-medium bg-primary hover:bg-primary/90 text-white transition-colors">
          Tamam
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- ─── Yük Profili Şifre Dialog'u ───────────────────────────────────────────── -->
{#if showLpPasswordDialog}
  <div
    role="presentation"
    class="fixed inset-0 z-[1000] flex items-center justify-center p-4 bg-black/75 backdrop-blur-lg"
    onclick={(e) => { if (e.target === e.currentTarget) cancelLpPassword(); }}
    onkeydown={(e) => { if (e.key === "Escape") cancelLpPassword(); }}
  >
    <div class="w-full max-w-sm bg-white dark:bg-slate-800 rounded-2xl shadow-2xl border border-slate-200 dark:border-slate-700 overflow-hidden">
      <div class="flex items-center gap-3 px-5 py-4 border-b border-primary/20 bg-primary/5">
        <span class="material-symbols-outlined text-2xl text-primary">key</span>
        <h3 class="font-bold text-slate-900 dark:text-white">Yük Profili Şifresi</h3>
        <button onclick={cancelLpPassword} class="ml-auto text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 transition-colors">
          <span class="material-symbols-outlined">close</span>
        </button>
      </div>
      <div class="px-5 py-4 space-y-4">
        <p class="text-sm text-slate-700 dark:text-slate-300 leading-relaxed">
          Sayaç yük profili okumak için şifre gerektiriyor. Lütfen doğru şifre seviyesini ve şifreyi giriniz.
        </p>

        <!-- Şifre seviyesi seçimi -->
        <div class="space-y-2">
          <label class="block text-xs font-medium text-slate-600 dark:text-slate-400">Şifre Seviyesi</label>
          <select
            bind:value={lpPasswordLevel}
            class="w-full px-3 py-2 text-sm rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-700 text-slate-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-primary/50">
            <option value={1}>P1 - Okuma Şifresi</option>
            <option value={2}>P2 - Yazma Şifresi</option>
            <option value={3}>P3 - Yönetici Şifresi</option>
          </select>
        </div>

        <!-- Şifre girişi -->
        <div class="space-y-2">
          <label class="block text-xs font-medium text-slate-600 dark:text-slate-400">Şifre (8 hane)</label>
          <input
            type="password"
            bind:value={lpPassword}
            maxlength={8}
            placeholder="12345678"
            class="w-full px-3 py-2 text-sm font-mono rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-700 text-slate-900 dark:text-white placeholder-slate-400 focus:outline-none focus:ring-2 focus:ring-primary/50"
            onkeydown={(e) => { if (e.key === "Enter") confirmLpPassword(); }}
          />
        </div>

        <div class="flex items-start gap-2 p-3 rounded-lg bg-amber-500/10 border border-amber-500/20">
          <span class="material-symbols-outlined text-amber-500 text-base mt-0.5">warning</span>
          <p class="text-xs text-amber-600 dark:text-amber-400 leading-relaxed">
            Yanlış şifre girilmesi sayacı kilitleyebilir. Emin değilseniz işlemi iptal edin.
          </p>
        </div>
      </div>
      <div class="flex justify-end gap-3 px-5 py-3 border-t border-slate-200 dark:border-slate-700">
        <button onclick={cancelLpPassword}
          class="px-4 py-2 rounded-lg text-sm font-medium text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-700 transition-colors">
          İptal
        </button>
        <button onclick={confirmLpPassword}
          class="px-4 py-2 rounded-lg text-sm font-medium bg-primary hover:bg-primary/90 text-white transition-colors">
          Devam Et
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- ─── PDF Mod Seçimi Modalı ──────────────────────────────────────────────── -->
{#if showPdfModeModal}
  <div
    role="presentation"
    class="fixed inset-0 z-40 flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm"
    onclick={(e) => { if (e.target === e.currentTarget) showPdfModeModal = false; }}
    onkeydown={(e) => { if (e.key === "Escape") showPdfModeModal = false; }}
  >
    <div class="w-full max-w-sm bg-white dark:bg-slate-800 rounded-2xl shadow-2xl border border-slate-200 dark:border-slate-700 overflow-hidden">
      <div class="flex items-center justify-between px-5 py-4 border-b border-slate-200 dark:border-slate-700">
        <div class="flex items-center gap-2">
          <span class="material-symbols-outlined text-primary text-xl">picture_as_pdf</span>
          <h3 class="font-bold text-slate-900 dark:text-white">PDF Çıktı Modu</h3>
        </div>
        <button onclick={() => showPdfModeModal = false} class="text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 transition-colors">
          <span class="material-symbols-outlined">close</span>
        </button>
      </div>
      <div class="p-5 space-y-3">
        <button
          onclick={() => exportPdf(false)}
          class="w-full flex items-center gap-4 p-4 rounded-xl border-2 border-slate-200 dark:border-slate-600 hover:border-primary hover:bg-primary/5 dark:hover:bg-primary/10 transition-all text-left group"
        >
          <div class="flex items-center justify-center w-12 h-12 rounded-lg bg-gradient-to-br from-primary/20 to-emerald-400/20 group-hover:from-primary/30 group-hover:to-emerald-400/30 transition-colors">
            <span class="material-symbols-outlined text-2xl text-primary">palette</span>
          </div>
          <div>
            <p class="text-sm font-bold text-slate-900 dark:text-white">Renkli</p>
            <p class="text-xs text-slate-500 dark:text-slate-400">Ekranda görüntüleme ve renkli yazıcı için</p>
          </div>
        </button>
        <button
          onclick={() => exportPdf(true)}
          class="w-full flex items-center gap-4 p-4 rounded-xl border-2 border-slate-200 dark:border-slate-600 hover:border-slate-400 hover:bg-slate-50 dark:hover:bg-slate-700 transition-all text-left group"
        >
          <div class="flex items-center justify-center w-12 h-12 rounded-lg bg-slate-100 dark:bg-slate-700 group-hover:bg-slate-200 dark:group-hover:bg-slate-600 transition-colors">
            <span class="material-symbols-outlined text-2xl text-slate-600 dark:text-slate-300">print</span>
          </div>
          <div>
            <p class="text-sm font-bold text-slate-900 dark:text-white">Siyah-Beyaz</p>
            <p class="text-xs text-slate-500 dark:text-slate-400">Siyah-beyaz yazıcıda baskı için optimize</p>
          </div>
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  @keyframes shake-glow {
    0%, 100% { transform: translateX(0); }
    10%, 30%, 50%, 70%, 90% { transform: translateX(-3px); }
    20%, 40%, 60%, 80% { transform: translateX(3px); }
  }
  :global(.animate-shake-glow) {
    animation: shake-glow 0.5s ease-in-out;
    box-shadow: 0 0 12px rgba(245, 158, 11, 0.5);
  }
</style>
