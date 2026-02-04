import { writable, derived } from "svelte/store";

export type LogType = "info" | "warn" | "success" | "error" | "tx" | "rx";

export interface LogEntry {
  id: number;
  timestamp: Date;
  type: LogType;
  message: string;
  raw?: string;
}

function createLogsStore() {
  const { subscribe, update, set } = writable<LogEntry[]>([]);
  let nextId = 1;

  const add = (type: LogType, message: string, raw?: string) => {
    const entry: LogEntry = {
      id: nextId++,
      timestamp: new Date(),
      type,
      message,
      raw,
    };
    update((logs) => [...logs, entry]);
    return entry;
  };

  return {
    subscribe,
    add,
    info: (message: string) => add("info", message),
    warn: (message: string) => add("warn", message),
    success: (message: string) => add("success", message),
    error: (message: string) => add("error", message),
    tx: (message: string, raw?: string) => add("tx", message, raw),
    rx: (message: string, raw?: string) => add("rx", message, raw),
    clear: () => {
      set([]);
      nextId = 1;
    },
  };
}

export const logsStore = createLogsStore();

// Helper to add logs from outside the store
export function addLog(type: LogType, message: string, raw?: string) {
  return logsStore.add(type, message, raw);
}

export const recentLogs = derived(logsStore, ($logs) => $logs.slice(-100));
