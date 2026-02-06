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
export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

// Connection commands
export async function listSerialPorts(): Promise<PortInfo[]> {
  if (!isTauri()) {
    // Mock data for development
    return [
      { name: "COM3", description: "USB Serial Port", portType: "usb" },
      { name: "COM4", description: "Optical Probe", portType: "usb" },
    ];
  }
  return invoke<PortInfo[]>("list_serial_ports");
}

export async function connect(params: ConnectionParams): Promise<MeterIdentity> {
  if (!isTauri()) {
    // Mock connection for development
    await new Promise((r) => setTimeout(r, 1000));
    return {
      manufacturer: "MKS",
      edasId: "ADM",
      model: "M550.2251",
      baudRateChar: "5",
      generation: "2",
      serialNumber: "123456789",
    };
  }
  return invoke<MeterIdentity>("connect", { params });
}

export async function disconnect(): Promise<void> {
  if (!isTauri()) {
    return;
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

// Reading commands
export async function readShort(): Promise<ShortReadResult> {
  if (!isTauri()) {
    // Mock data for development
    await new Promise((r) => setTimeout(r, 2000));
    return {
      serialNumber: "123456789",
      programVersion: "V01.00",
      productionDate: "2024-06-30",
      calibrationDate: "2024-06-30",
      meterDate: "2024-12-15",
      meterTime: "14:30:35",
      dayOfWeek: 4,
      activeEnergyImportTotal: 123456.789,
      activeEnergyImportT1: 45678.123,
      activeEnergyImportT2: 34567.234,
      activeEnergyImportT3: 43211.432,
      activeEnergyImportT4: 0,
      maxDemandImport: 123.456,
      maxDemandImportTimestamp: "2024-02-01 13:30",
      voltageL1: 220.5,
      voltageL2: 221.3,
      voltageL3: 219.8,
      currentL1: 16.5,
      currentL2: 15.8,
      currentL3: 17.2,
      frequency: 49.9,
      powerFactorL1: 0.97,
      powerFactorL2: 0.96,
      powerFactorL3: 0.98,
      ffCode: "0000000000000090",
      gfCode: "0000000000000004",
      batteryStatus: "full",
      relayStatus: "active",
      rawData: null,
    };
  }
  return invoke<ShortReadResult>("read_short");
}

export async function readFull(): Promise<ShortReadResult> {
  if (!isTauri()) {
    return readShort(); // Use same mock for now
  }
  return invoke<ShortReadResult>("read_full");
}

export async function readObis(obisCode: string): Promise<string> {
  if (!isTauri()) {
    return "mock-value";
  }
  return invoke<string>("read_obis", { obisCode });
}

export async function readObisBatch(obisCodes: string[], password?: string): Promise<Record<string, string>> {
  if (!isTauri()) {
    // Mock data for development
    await new Promise((r) => setTimeout(r, 1500));
    const result: Record<string, string> = {};
    for (const code of obisCodes) {
      result[code] = `${(Math.random() * 1000).toFixed(3)}*kWh`;
    }
    return result;
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
  endTime: string | null
): Promise<LoadProfileResult> {
  if (!isTauri()) {
    // Mock data for development
    await new Promise((r) => setTimeout(r, 2000));
    const entries: LoadProfileEntry[] = [];
    const start = startTime ? new Date(`20${startTime.replace(",", " ")}`) : new Date("2024-12-01");
    const end = endTime ? new Date(`20${endTime.replace(",", " ")}`) : new Date("2024-12-15");
    const interval = 15 * 60 * 1000;

    let current = new Date(start);
    while (current <= end) {
      entries.push({
        timestamp: current.toISOString().slice(2, 16).replace("T", ",").replace(/-/g, "-"),
        values: [
          Math.random() * 10 + 50,
          220 + Math.random() * 5 - 2.5,
          220 + Math.random() * 5 - 2.5,
          220 + Math.random() * 5 - 2.5,
        ],
        status: null,
      });
      current = new Date(current.getTime() + interval);
    }

    return {
      profileNumber,
      entries,
      rawData: "",
    };
  }
  return invoke<LoadProfileResult>("read_load_profile", { profileNumber, startTime, endTime });
}

// Programming commands
export async function authenticate(password: string): Promise<boolean> {
  if (!isTauri()) {
    return true;
  }
  return invoke<boolean>("authenticate", { password });
}

export async function writeObis(obisCode: string, value: string): Promise<void> {
  if (!isTauri()) {
    return;
  }
  return invoke("write_obis", { obisCode, value });
}

export async function syncTime(): Promise<void> {
  if (!isTauri()) {
    return;
  }
  return invoke("sync_time");
}

export async function endSession(): Promise<void> {
  if (!isTauri()) {
    return;
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
    return 1;
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
    // Mock data for development
    return [
      {
        id: 1,
        meterSerial: "123456789",
        meterModel: "M550.2251",
        meterFlag: "MKS",
        timestamp: "2024-12-15 14:30:00",
        connectionType: "optical",
        resultStatus: "success",
        note: null,
        dataJson: "{}",
      },
    ];
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
    return 1;
  }
  return invoke<number>("save_report", { report });
}

export async function getRecentReports(limit: number = 10): Promise<Report[]> {
  if (!isTauri()) {
    // Mock data for development
    return [
      {
        id: 1,
        sessionId: 1,
        reportType: "csv",
        filename: "short_read_2024-12-15.csv",
        filepath: "/reports/short_read_2024-12-15.csv",
        createdAt: "2024-12-15 14:35:00",
      },
    ];
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
    // Mock for development
    const timestamp = new Date().toISOString().replace(/[-:T]/g, "").slice(0, 12);
    return `${flag}-${serialNumber}-${timestamp}.json`;
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
    // Mock for development
    return [
      {
        fileName: "MKS-123456789-202412151430.json",
        flag: "MKS",
        serialNumber: "123456789",
        model: "M550.2251",
        savedAt: "2024-12-15T14:30:00Z",
        note: "Test session",
      },
    ];
  }
  return invoke<SessionFileInfo[]>("list_session_files");
}

export async function loadSessionFile(filename: string): Promise<SessionFileData> {
  if (!isTauri()) {
    // Mock for development
    return {
      flag: "MKS",
      serialNumber: "123456789",
      model: "M550.2251",
      savedAt: "2024-12-15T14:30:00Z",
      note: "Test session",
      meterData: {},
      connectionInfo: {},
    };
  }
  return invoke<SessionFileData>("load_session_file", { filename });
}

export async function deleteSessionFile(filename: string): Promise<void> {
  if (!isTauri()) {
    return;
  }
  return invoke("delete_session_file", { filename });
}
