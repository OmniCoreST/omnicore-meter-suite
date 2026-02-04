import { writable, derived } from "svelte/store";

export interface SerialPort {
  name: string;
  description?: string;
  portType?: string;
}

export interface ConnectionParams {
  connectionType: "optical" | "rs485" | "tcpip";
  port: string;
  baudRate: number;
  timeout: number;
  meterAddress?: string;
  password?: string;
}

export interface ConnectionState {
  status: "disconnected" | "connecting" | "connected" | "error";
  availablePorts: SerialPort[];
  selectedPort: string | null;
  params: ConnectionParams;
  error: string | null;
  meterIdentity: MeterIdentity | null;
}

export interface MeterIdentity {
  flag: string;
  manufacturer: string;
  edasId: string;
  model: string;
  baudChar: string;
  serialNumber: string;
}

const defaultParams: ConnectionParams = {
  connectionType: "optical",
  port: "",
  baudRate: 9600,
  timeout: 2000,
  meterAddress: "",
  password: "",
};

const initialState: ConnectionState = {
  status: "disconnected",
  availablePorts: [],
  selectedPort: null,
  params: { ...defaultParams },
  error: null,
  meterIdentity: null,
};

function createConnectionStore() {
  const { subscribe, set, update } = writable<ConnectionState>(initialState);

  return {
    subscribe,
    setAvailablePorts: (ports: SerialPort[]) => {
      update((state) => ({ ...state, availablePorts: ports }));
    },
    selectPort: (port: string) => {
      update((state) => ({
        ...state,
        selectedPort: port,
        params: { ...state.params, port },
      }));
    },
    updateParams: (params: Partial<ConnectionParams>) => {
      update((state) => ({
        ...state,
        params: { ...state.params, ...params },
      }));
    },
    setConnecting: (connecting: boolean) => {
      update((state) => ({
        ...state,
        status: connecting ? "connecting" : state.status === "connecting" ? "disconnected" : state.status,
        error: null,
      }));
    },
    connect: (params: Partial<ConnectionParams>) => {
      update((state) => ({
        ...state,
        status: "connected",
        params: { ...state.params, ...params },
        error: null,
      }));
    },
    disconnect: () => {
      update((state) => ({
        ...state,
        status: "disconnected",
        meterIdentity: null,
        error: null,
      }));
    },
    setMeterIdentity: (identity: MeterIdentity) => {
      update((state) => ({
        ...state,
        meterIdentity: identity,
      }));
    },
    setConnected: (identity: MeterIdentity) => {
      update((state) => ({
        ...state,
        status: "connected",
        meterIdentity: identity,
        error: null,
      }));
    },
    setDisconnected: () => {
      update((state) => ({
        ...state,
        status: "disconnected",
        meterIdentity: null,
        error: null,
      }));
    },
    setError: (error: string) => {
      update((state) => ({
        ...state,
        status: "error",
        error,
      }));
    },
    reset: () => {
      set(initialState);
    },
  };
}

export const connectionStore = createConnectionStore();

export const isConnected = derived(
  connectionStore,
  ($connection) => $connection.status === "connected"
);

export const isConnecting = derived(
  connectionStore,
  ($connection) => $connection.status === "connecting"
);
