import { writable } from "svelte/store";

export type Page =
  | "dashboard"  // Dashboard - device connection
  | "sessions"   // All sessions view
  // OKUMA (Reading) Section - 8 pages
  | "overview"
  | "live-measurements"
  | "energy"
  | "demand"
  | "load-profile"
  | "warnings"
  | "outages"
  | "status-codes"
  // AYARLAR (Settings) Section - 7 pages
  | "time-date"
  | "password"
  | "dst"
  | "tariffs"
  | "periods"
  | "relay-control"
  | "obis-reader";

// URL aliases for backward compatibility
const pageAliases: Record<string, Page> = {
  "short-read": "energy",
  "full-read": "energy",
  "events": "warnings",
  "alarms": "status-codes",
  "time-sync": "time-date",
};

function createNavigationStore() {
  const { subscribe, set } = writable<Page>("dashboard");

  return {
    subscribe,
    navigate: (page: Page) => {
      window.location.hash = page;
      set(page);
    },
    init: () => {
      let hash = window.location.hash.slice(1) as string;

      // Apply alias if exists
      if (hash && pageAliases[hash]) {
        hash = pageAliases[hash];
        window.location.hash = hash;
      }

      if (hash) {
        set(hash as Page);
      }

      window.addEventListener("hashchange", () => {
        let newHash = window.location.hash.slice(1) as string;

        // Apply alias if exists
        if (newHash && pageAliases[newHash]) {
          newHash = pageAliases[newHash];
          window.location.hash = newHash;
        }

        if (newHash) {
          set(newHash as Page);
        }
      });
    },
  };
}

export const navigationStore = createNavigationStore();
