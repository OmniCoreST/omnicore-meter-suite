import { writable } from "svelte/store";

export type Page =
  | "dashboard"
  | "short-read"
  | "full-read"
  | "load-profile"
  | "events"
  | "alarms"
  | "time-sync"
  | "password"
  | "dst"
  | "periods"
  | "tariffs";

function createNavigationStore() {
  const { subscribe, set } = writable<Page>("dashboard");

  return {
    subscribe,
    navigate: (page: Page) => {
      window.location.hash = page;
      set(page);
    },
    init: () => {
      const hash = window.location.hash.slice(1) as Page;
      if (hash) {
        set(hash);
      }

      window.addEventListener("hashchange", () => {
        const newHash = window.location.hash.slice(1) as Page;
        if (newHash) {
          set(newHash);
        }
      });
    },
  };
}

export const navigationStore = createNavigationStore();
