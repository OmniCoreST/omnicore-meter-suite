import { writable } from "svelte/store";
import { listSessionFiles, deleteSessionFile } from "$lib/utils/tauri";

export interface SessionInfo {
  id: number;
  flag: string;
  serialNumber: string;
  model: string;
  dateTime: string;
  note: string;
  success: boolean;
  fileName: string;
}

function createSessionsStore() {
  const { subscribe, set, update } = writable<SessionInfo[]>([]);

  return {
    subscribe,
    refresh: async () => {
      try {
        const sessions = await listSessionFiles();
        const mapped = sessions.map((s, index) => ({
          id: index + 1,
          flag: s.flag || "UNK",
          serialNumber: s.serialNumber || "Unknown",
          model: s.model || "",
          dateTime: s.savedAt || "",
          note: s.note || "",
          success: true,
          fileName: s.fileName || "",
        }));
        set(mapped);
      } catch (e) {
        console.error("Failed to load sessions:", e);
      }
    },
    delete: async (fileName: string) => {
      await deleteSessionFile(fileName);
      update(sessions => sessions.filter(s => s.fileName !== fileName));
    },
    clear: () => set([]),
  };
}

export const sessionsStore = createSessionsStore();
