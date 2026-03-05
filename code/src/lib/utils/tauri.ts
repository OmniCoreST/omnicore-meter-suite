/**
 * Tauri API wrapper for meter communication
 */
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

// Types
export interface PortInfo {
  name: string;
  description: string | null;
  portType: string;
}

export interface MeterIdentity {
  manufacturer: string;
  edasId: string;
  model: string;
  baudRateChar: string;
  generation: string;
  serialNumber: string | null;
}

export interface ConnectionParams {
  connectionType: string;
  port: string;
  baudRate: number;
  timeoutMs: number;
  meterAddress: string | null;
  password: string | null;
}

export interface ShortReadResult {
  serialNumber: string;
  programVersion: string;
  productionDate: string;
  calibrationDate: string;
  meterDate: string;
  meterTime: string;
  dayOfWeek: number;
  activeEnergyImportTotal: number;
  activeEnergyImportT1: number;
  activeEnergyImportT2: number;
  activeEnergyImportT3: number;
  activeEnergyImportT4: number;
  maxDemandImport: number;
  maxDemandImportTimestamp: string;
  voltageL1: number;
  voltageL2: number;
  voltageL3: number;
  currentL1: number;
  currentL2: number;
  currentL3: number;
  frequency: number;
  powerFactorL1: number;
  powerFactorL2: number;
  powerFactorL3: number;
  ffCode: string;
  gfCode: string;
  batteryStatus: "full" | "low" | "";
  relayStatus: "active" | "passive" | "";
  rawData: string | null;
}

export interface ProgressEvent {
  step: number;
  total: number;
  message: string;
}

export interface LogEvent {
  timestamp: string;
  logType: string;
  message: string;
  data: string | null;
}

// Check if running in Tauri
// Once detected, cache the result to survive HMR (hot module replacement)
let _tauriDetected = false;
export function isTauri(): boolean {
  if (_tauriDetected) return true;
  if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
    _tauriDetected = true;
    return true;
  }
  return false;
}

// Connection commands - All require Tauri context (no mocks)
export async function listSerialPorts(): Promise<PortInfo[]> {
  if (!isTauri()) {
    return []; // No ports available outside Tauri
  }
  return invoke<PortInfo[]>("list_serial_ports");
}

export async function connect(params: ConnectionParams): Promise<MeterIdentity> {
  if (!isTauri()) {
    throw new Error("Tauri bağlamı bulunamadı - uygulamayı yeniden başlatın");
  }
  return invoke<MeterIdentity>("connect", { params });
}

export async function disconnect(): Promise<void> {
  if (!isTauri()) {
    throw new Error("Tauri bağlamı bulunamadı - uygulamayı yeniden başlatın");
  }
  return invoke("disconnect");
}

export async function getConnectionStatus(): Promise<boolean> {
  if (!isTauri()) {
    return false;
  }
  return invoke<boolean>("get_connection_status");
}

export async function getMeterIdentity(): Promise<MeterIdentity | null> {
  if (!isTauri()) {
    return null;
  }
  return invoke<MeterIdentity | null>("get_meter_identity");
}

// Reading commands - All require Tauri context (no mocks)
export async function readShort(): Promise<ShortReadResult> {
  if (!isTauri()) {
    throw new Error("Tauri bağlamı bulunamadı - uygulamayı yeniden başlatın");
  }
  return invoke<ShortReadResult>("read_short");
}

export async function readFull(): Promise<ShortReadResult> {
  if (!isTauri()) {
    throw new Error("Tauri bağlamı bulunamadı - uygulamayı yeniden başlatın");
  }
  return invoke<ShortReadResult>("read_full");
}

export async function readObis(obisCode: string): Promise<string> {
  if (!isTauri()) {
    throw new Error("Tauri bağlamı bulunamadı - uygulamayı yeniden başlatın");
  }
  return invoke<string>("read_obis", { obisCode });
}

export async function readObisBatch(obisCodes: string[], password?: string): Promise<Record<string, string>> {
  if (!isTauri()) {
    throw new Error("Tauri bağlamı bulunamadı - uygulamayı yeniden başlatın");
  }
  return invoke<Record<string, string>>("read_obis_batch", { obisCodes, password: password || null });
}

// Load profile types
export interface LoadProfileEntry {
  timestamp: string;
  values: number[];
  status: string | null;
}

export interface LoadProfileResult {
  profileNumber: number;
  entries: LoadProfileEntry[];
  rawData: string;
}

export async function readLoadProfile(
  profileNumber: number,
  startTime: string | null,
  endTime: string | null,
  password?: string
): Promise<LoadProfileResult> {
  if (!isTauri()) {
    throw new Error("Tauri bağlamı bulunamadı - uygulamayı yeniden başlatın");
  }
  return invoke<LoadProfileResult>("read_load_profile", { profileNumber, startTime, endTime, password: password || null });
}

// Mode-specific packet read (Modes 5, 7, 8, 9)
export interface PacketReadResult {
  mode: number;
  rawData: string;
  bytesRead: number;
  readDurationMs: number;
  bccValid: boolean;
}

export async function readPacket(mode: number): Promise<PacketReadResult> {
  if (!isTauri()) {
    throw new Error("Tauri bağlamı bulunamadı - uygulamayı yeniden başlatın");
  }
  return invoke<PacketReadResult>("read_packet", { mode });
}

// Programming commands - NO mock fallbacks (these must only run in Tauri)
export async function authenticate(password: string, level: number = 1): Promise<boolean> {
  if (!isTauri()) {
    throw new Error("Tauri bağlamı bulunamadı - uygulamayı yeniden başlatın");
  }
  return invoke<boolean>("authenticate", { password, level });
}

export async function changePassword(currentPassword: string, newPassword: string, level: number = 3): Promise<string> {
  if (!isTauri()) {
    throw new Error("Tauri bağlamı bulunamadı - uygulamayı yeniden başlatın");
  }
  return invoke<string>("change_password", { currentPassword, newPassword, level });
}

export async function writeObis(obisCode: string, value: string): Promise<void> {
  if (!isTauri()) {
    throw new Error("Tauri bağlamı bulunamadı - uygulamayı yeniden başlatın");
  }
  return invoke("write_obis", { obisCode, value });
}

export async function syncTime(): Promise<void> {
  if (!isTauri()) {
    throw new Error("Tauri bağlamı bulunamadı - uygulamayı yeniden başlatın");
  }
  return invoke("sync_time");
}

export async function endSession(): Promise<void> {
  if (!isTauri()) {
    throw new Error("Tauri bağlamı bulunamadı - uygulamayı yeniden başlatın");
  }
  return invoke("end_session");
}

// Event listeners
export async function onReadProgress(
  callback: (event: ProgressEvent) => void
): Promise<UnlistenFn> {
  return listen<ProgressEvent>("read-progress", (event) => {
    callback(event.payload);
  });
}

export async function onCommLog(
  callback: (event: LogEvent) => void
): Promise<UnlistenFn> {
  return listen<LogEvent>("comm-log", (event) => {
    callback(event.payload);
  });
}

// Database types
export interface Session {
  id: number;
  meterSerial: string;
  meterModel: string;
  meterFlag: string;
  timestamp: string;
  connectionType: string;
  resultStatus: string;
  note: string | null;
  dataJson: string;
}

export interface Report {
  id: number;
  sessionId: number;
  reportType: string;
  filename: string;
  filepath: string;
  createdAt: string;
}

// Database commands
export async function saveSession(session: Omit<Session, "id">, overwrite: boolean): Promise<number> {
  if (!isTauri()) {
    throw new Error("Tauri bağlamı bulunamadı");
  }
  return invoke<number>("save_session", { session, overwrite });
}

export async function getSession(id: number): Promise<Session | null> {
  if (!isTauri()) {
    return null;
  }
  return invoke<Session | null>("get_session", { id });
}

export async function getRecentSessions(limit: number = 10): Promise<Session[]> {
  if (!isTauri()) {
    return [];
  }
  return invoke<Session[]>("get_recent_sessions", { limit });
}

export async function deleteSession(id: number): Promise<void> {
  if (!isTauri()) {
    return;
  }
  return invoke("delete_session", { id });
}

export async function saveReport(report: Omit<Report, "id" | "createdAt">): Promise<number> {
  if (!isTauri()) {
    throw new Error("Tauri bağlamı bulunamadı");
  }
  return invoke<number>("save_report", { report });
}

export async function getRecentReports(limit: number = 10): Promise<Report[]> {
  if (!isTauri()) {
    return [];
  }
  return invoke<Report[]>("get_recent_reports", { limit });
}

export async function getSetting(key: string): Promise<string | null> {
  if (!isTauri()) {
    return localStorage.getItem(key);
  }
  return invoke<string | null>("get_setting", { key });
}

export async function setSetting(key: string, value: string): Promise<void> {
  if (!isTauri()) {
    localStorage.setItem(key, value);
    return;
  }
  return invoke("set_setting", { key, value });
}

// Session file commands (file-based storage next to executable)
export interface SessionFileData {
  flag: string;
  serialNumber: string;
  model: string;
  savedAt: string;
  note: string;
  meterData: Record<string, unknown>;
  connectionInfo: Record<string, unknown>;
}

export interface SessionFileInfo {
  fileName: string;
  flag: string;
  serialNumber: string;
  model: string;
  savedAt: string;
  note: string;
}

export async function saveSessionFile(
  flag: string,
  serialNumber: string,
  model: string,
  note: string,
  meterData: Record<string, unknown>,
  connectionInfo: Record<string, unknown>,
  overwriteExisting: boolean
): Promise<string> {
  if (!isTauri()) {
    throw new Error("Tauri bağlamı bulunamadı");
  }
  return invoke<string>("save_session_file", {
    flag,
    serialNumber,
    model,
    note,
    meterData,
    connectionInfo,
    overwriteExisting,
  });
}

export async function listSessionFiles(): Promise<SessionFileInfo[]> {
  if (!isTauri()) {
    return [];
  }
  return invoke<SessionFileInfo[]>("list_session_files");
}

export async function loadSessionFile(filename: string): Promise<SessionFileData> {
  if (!isTauri()) {
    throw new Error("Tauri bağlamı bulunamadı");
  }
  return invoke<SessionFileData>("load_session_file", { filename });
}

export async function deleteSessionFile(filename: string): Promise<void> {
  if (!isTauri()) {
    return;
  }
  return invoke("delete_session_file", { filename });
}

// ─── Uyumluluk ────────────────────────────────────────────────────────────────

export interface ComplianceIssue {
  code: string;
  severity: "error" | "warning" | "info";
  field: string;
  expected: string;
  actual: string;
  description: string;
  specRef?: string;
}

export type RulesStatus = "ok" | "offline" | "tooOld";

export interface ComplianceResult {
  issues: ComplianceIssue[];
  errorCount: number;
  warningCount: number;
  infoCount: number;
  rulesVersion: string;
  latestVersion: string | null;
  rulesStatus: RulesStatus;
  checkedAt: string;
  rulesFilePath: string;
}

export async function checkCompliance(data: ShortReadResult, meterPhases: number = 3): Promise<ComplianceResult> {
  if (!isTauri()) throw new Error("Tauri bağlamı bulunamadı");
  return invoke<ComplianceResult>("check_compliance", { data, meterPhases });
}

export async function getComplianceRulesPath(): Promise<string> {
  if (!isTauri()) return "";
  return invoke<string>("get_compliance_rules_path");
}

export async function reloadComplianceRules(): Promise<string> {
  if (!isTauri()) throw new Error("Tauri bağlamı bulunamadı");
  return invoke<string>("reload_compliance_rules");
}

export async function updateComplianceRules(): Promise<string> {
  if (!isTauri()) throw new Error("Tauri bağlamı bulunamadı");
  return invoke<string>("update_compliance_rules");
}

export async function importComplianceRulesFromFile(path: string): Promise<string> {
  if (!isTauri()) throw new Error("Tauri bağlamı bulunamadı");
  return invoke<string>("import_compliance_rules_from_file", { path });
}

export async function addComplianceRule(ruleToml: string): Promise<string> {
  if (!isTauri()) throw new Error("Tauri bağlamı bulunamadı");
  return invoke<string>("add_compliance_rule", { ruleToml });
}

export interface ComplianceRuleDef {
  code: string;
  field: string;
  check: string;
  severity: string;
  description: string;
  min: number | null;
  max: number | null;
  value: string | null;
  bit: number | null;
  tolerance: number | null;
  max_drift: number | null;
  phases: number | null;
  spec_ref: string | null;
  cause: string | null;
  remedy: string | null;
}

export async function listComplianceRules(): Promise<ComplianceRuleDef[]> {
  if (!isTauri()) return [];
  return invoke<ComplianceRuleDef[]>("list_compliance_rules");
}

export async function updateComplianceRule(rule: ComplianceRuleDef): Promise<string> {
  if (!isTauri()) throw new Error("Tauri bağlamı bulunamadı");
  return invoke<string>("update_compliance_rule", { rule });
}

export async function deleteComplianceRule(code: string): Promise<string> {
  if (!isTauri()) throw new Error("Tauri bağlamı bulunamadı");
  return invoke<string>("delete_compliance_rule", { code });
}
