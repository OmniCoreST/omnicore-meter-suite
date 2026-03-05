import { writable, derived } from "svelte/store";

export interface ComplianceIssue {
  code: string;
  severity: "error" | "warning" | "info";
  field: string;
  expected: string;
  actual: string;
  description: string;
  specRef?: string;
  cause?: string;
  remedy?: string;
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

export interface ComplianceState {
  result: ComplianceResult | null;
  loading: boolean;
  error: string | null;
}

function createComplianceStore() {
  const { subscribe, set, update } = writable<ComplianceState>({
    result: null,
    loading: false,
    error: null,
  });

  return {
    subscribe,
    setLoading: () => update((s) => ({ ...s, loading: true, error: null })),
    setResult: (result: ComplianceResult) => set({ result, loading: false, error: null }),
    setError: (error: string) => update((s) => ({ ...s, loading: false, error })),
    clear: () => set({ result: null, loading: false, error: null }),
  };
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
