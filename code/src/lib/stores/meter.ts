import { writable, derived } from "svelte/store";

export interface ShortReadData {
  // Meter Identity
  serialNumber: string;
  programVersion: string;
  productionDate: string;
  calibrationDate: string;

  // Date/Time
  meterDate: string;
  meterTime: string;
  dayOfWeek: number;

  // Active Energy Import (+)
  activeEnergyImportTotal: number;
  activeEnergyImportT1: number;
  activeEnergyImportT2: number;
  activeEnergyImportT3: number;
  activeEnergyImportT4: number;

  // Active Energy Export (-) - bidirectional only
  activeEnergyExportTotal?: number;
  activeEnergyExportT1?: number;
  activeEnergyExportT2?: number;
  activeEnergyExportT3?: number;
  activeEnergyExportT4?: number;

  // Reactive Energy - Kombi only
  reactiveEnergyInductiveImport?: number;
  reactiveEnergyCapacitiveImport?: number;
  reactiveEnergyInductiveExport?: number;
  reactiveEnergyCapacitiveExport?: number;

  // Maximum Demand
  maxDemandImport: number;
  maxDemandImportTimestamp: string;
  maxDemandExport?: number;
  maxDemandExportTimestamp?: string;

  // Instantaneous Values
  voltageL1: number;
  voltageL2?: number;
  voltageL3?: number;
  currentL1: number;
  currentL2?: number;
  currentL3?: number;
  frequency: number;
  powerFactorL1: number;
  powerFactorL2?: number;
  powerFactorL3?: number;

  // Status Codes
  ffCode: string;
  gfCode: string;
  batteryStatus: "full" | "low" | "";
  relayStatus: "active" | "passive" | "";

  // Raw data for parsing profile definitions etc.
  rawData?: string | null;
}

export interface FullReadData extends ShortReadData {
  // Historical Data (12 months)
  monthlyData: MonthlyData[];

  // Warnings
  voltageWarnings: EventRecord[];
  currentWarnings: EventRecord[];
  magneticFieldWarnings: EventRecord[];
  coverOpenings: EventRecord[];

  // Outage Records
  threePhaseOutagesLong: OutageRecord[];
  threePhaseOutagesShort: OutageRecord[];
  l1OutagesLong: OutageRecord[];
  l1OutagesShort: OutageRecord[];
  l2OutagesLong: OutageRecord[];
  l2OutagesShort: OutageRecord[];
  l3OutagesLong: OutageRecord[];
  l3OutagesShort: OutageRecord[];

  // Technical Quality
  technicalQualityParams?: Record<string, string>;
}

export interface MonthlyData {
  month: number; // 1-12
  activeEnergyImportT1: number;
  activeEnergyImportT2: number;
  activeEnergyImportT3: number;
  activeEnergyImportT4: number;
  activeEnergyExportT1?: number;
  activeEnergyExportT2?: number;
  activeEnergyExportT3?: number;
  activeEnergyExportT4?: number;
  reactiveEnergyInductive?: number;
  reactiveEnergyCapacitive?: number;
  maxDemand: number;
  maxDemandTimestamp: string;
  demandResetDate: string;
  terminalCoverOpenings: number;
}

export interface EventRecord {
  id: number;
  type: string;
  startTime: string;
  endTime?: string;
  duration?: number;
  detail?: string;
}

export interface OutageRecord {
  id: number;
  startTime: string;
  endTime: string;
  duration: number; // in seconds
}

export interface LoadProfileEntry {
  timestamp: string;
  values: Record<string, number>;
}

export interface LoadProfileData {
  profileNumber: 1 | 2 | 3;
  periodMinutes: number;
  startDate: string;
  endDate: string;
  columns: string[];
  entries: LoadProfileEntry[];
}

export interface MeterState {
  shortReadData: ShortReadData | null;
  fullReadData: FullReadData | null;
  loadProfileData: LoadProfileData | null;
  meterType: "single-phase" | "three-phase" | "kombi" | null;
  isBidirectional: boolean;
  lastReadTime: Date | null;
  isReading: boolean;
}

const initialState: MeterState = {
  shortReadData: null,
  fullReadData: null,
  loadProfileData: null,
  meterType: null,
  isBidirectional: false,
  lastReadTime: null,
  isReading: false,
};

function createMeterStore() {
  const { subscribe, set, update } = writable<MeterState>(initialState);

  return {
    subscribe,
    setShortReadData: (data: ShortReadData, meterType: MeterState["meterType"], isBidirectional: boolean) => {
      console.log("[meterStore] setShortReadData called with:", { data, meterType, isBidirectional });
      update((state) => {
        const newState = {
          ...state,
          shortReadData: data,
          meterType,
          isBidirectional,
          lastReadTime: new Date(),
        };
        console.log("[meterStore] State updated, new shortReadData:", newState.shortReadData);
        return newState;
      });
    },
    setFullReadData: (data: FullReadData) => {
      update((state) => ({
        ...state,
        fullReadData: data,
        lastReadTime: new Date(),
      }));
    },
    setLoadProfileData: (data: LoadProfileData) => {
      update((state) => ({
        ...state,
        loadProfileData: data,
        lastReadTime: new Date(),
      }));
    },
    setAlarmCodes: (codes: { ffCode: string; gfCode: string }) => {
      update((state) => ({
        ...state,
        shortReadData: state.shortReadData ? { ...state.shortReadData, ...codes } : null,
        fullReadData: state.fullReadData ? { ...state.fullReadData, ...codes } : null,
      }));
    },
    setReading: (isReading: boolean) => {
      update((state) => ({ ...state, isReading }));
    },
    clear: () => {
      set(initialState);
    },
  };
}

export const meterStore = createMeterStore();

export const isMeterReading = derived(meterStore, ($meter) => $meter.isReading);
