import { writable, derived } from "svelte/store";

export interface ComplianceIssue {
  code: string;
  category: string;
  severity: "error" | "warning" | "info";
  description: string;
  expected: string;
  actual: string;
  specRef?: string;
  cause?: string;
  remedy?: string;
  sessionType?: string;
  obisCode?: string;
}

export interface SessionCheckResult {
  sessionType: string;
  success: boolean;
  issues: ComplianceIssue[];
  obisCount: number;
  durationMs: number;
}

export type RulesStatus = "ok" | "offline" | "tooOld";

export interface ComplianceResult {
  issues: ComplianceIssue[];
  sessionResults: SessionCheckResult[];
  errorCount: number;
  warningCount: number;
  infoCount: number;
  totalRulesChecked: number;
  configVersion: string;
  profileId: string;
  profileName: string;
  checkedAt: string;
  configFilePath: string;
  latestVersion: string | null;
  rulesStatus: RulesStatus;
}

export interface ComplianceProfile {
  id: string;
  name: string;
  phases: number;
  connection: string;
  description: string;
}

export interface TestStep {
  id: string;
  name: string;
  mode: string;
  enabled: boolean;
  timeoutSeconds: number;
  retryCount: number;
  obisCodes: string[];
  packetMode: number | null;
}

export interface TestPlan {
  name: string;
  description: string;
  steps: TestStep[];
}

// ─── Communication Log types (for v3 API) ───

export interface ObisLine {
  code: string;
  rawValue: string;
  value: string;
  unit: string | null;
}

export interface HandshakeLog {
  requestSent: boolean;
  identificationReceived: boolean;
  identificationRaw: string;
  identificationFormatValid: boolean;
  ackSent: boolean;
  ackMode: string | null;
  ackBaudChar: string | null;
  baudNegotiationSuccess: boolean;
  initialBaud: number;
  targetBaud: number;
  responseTimeMs: number;
}

export interface ProtocolEvent {
  eventType: string;
  timestampMs: number;
  detail: string;
  success: boolean;
}

export type SessionType =
  | "short_read"
  | "full_read"
  | "load_profile"
  | "obis_read"
  | "time_sync"
  | "obis_write";

export interface SessionLog {
  sessionType: SessionType;
  timestamp: string;
  handshake: HandshakeLog;
  obisLines: ObisLine[];
  protocolEvents: ProtocolEvent[];
  bccValid: boolean | null;
  etxFound: boolean;
  durationMs: number;
  success: boolean;
  error: string | null;
  modesUsed: string[];
}

export interface CommunicationLog {
  meterSerial: string;
  meterManufacturer: string;
  meterModel: string;
  profileId: string;
  sessions: SessionLog[];
}

// ─── Step state for persistent test progress ───

export type StepStatus = "pending" | "running" | "done" | "failed" | "skipped";

export interface StepState {
  step: TestStep;
  status: StepStatus;
  message: string;
  durationMs: number;
}

// ─── Store ───

export interface ComplianceState {
  result: ComplianceResult | null;
  profiles: ComplianceProfile[];
  testPlan: TestPlan | null;
  selectedProfileId: string;
  loading: boolean;
  error: string | null;
  // Persistent test run state
  running: boolean;
  stepStates: StepState[];
  runLog: string[];
}

function createComplianceStore() {
  const { subscribe, set, update } = writable<ComplianceState>({
    result: null,
    profiles: [],
    testPlan: null,
    selectedProfileId: "",
    loading: false,
    error: null,
    running: false,
    stepStates: [],
    runLog: [],
  });

  return {
    subscribe,
    setLoading: () => update((s) => ({ ...s, loading: true, error: null })),
    setResult: (result: ComplianceResult) =>
      set({ ...get(), result, loading: false, error: null }),
    setError: (error: string) =>
      update((s) => ({ ...s, loading: false, error })),
    setProfiles: (profiles: ComplianceProfile[]) =>
      update((s) => ({ ...s, profiles })),
    setTestPlan: (testPlan: TestPlan | null) =>
      update((s) => ({ ...s, testPlan })),
    setSelectedProfile: (id: string) =>
      update((s) => ({ ...s, selectedProfileId: id })),
    // Test run state
    setRunning: (running: boolean) =>
      update((s) => ({ ...s, running })),
    setStepStates: (stepStates: StepState[]) =>
      update((s) => ({ ...s, stepStates })),
    updateStepState: (index: number, patch: Partial<StepState>) =>
      update((s) => {
        const stepStates = [...s.stepStates];
        if (stepStates[index]) stepStates[index] = { ...stepStates[index], ...patch };
        return { ...s, stepStates };
      }),
    addRunLog: (line: string) =>
      update((s) => ({ ...s, runLog: [...s.runLog, line] })),
    clearRunLog: () =>
      update((s) => ({ ...s, runLog: [] })),
    clear: () =>
      set({
        result: null,
        profiles: [],
        testPlan: null,
        selectedProfileId: "",
        loading: false,
        error: null,
        running: false,
        stepStates: [],
        runLog: [],
      }),
  };
}

// Helper to get current value
function get(): ComplianceState {
  let val: ComplianceState;
  complianceStore.subscribe((v) => (val = v))();
  return val!;
}

export const complianceStore = createComplianceStore();

export const hasComplianceErrors = derived(
  complianceStore,
  ($s) => ($s.result?.errorCount ?? 0) > 0
);

export const hasComplianceWarnings = derived(
  complianceStore,
  ($s) => ($s.result?.warningCount ?? 0) > 0
);

export const complianceErrorCount = derived(
  complianceStore,
  ($s) => $s.result?.errorCount ?? 0
);

export const complianceWarningCount = derived(
  complianceStore,
  ($s) => $s.result?.warningCount ?? 0
);
