import { writable } from "svelte/store";

type Theme = "light" | "dark";

function createThemeStore() {
  const { subscribe, set, update } = writable<Theme>("light");

  return {
    subscribe,
    set,
    update,
    toggle: () => {
      update((current) => {
        const newTheme = current === "light" ? "dark" : "light";
        localStorage.setItem("theme", newTheme);
        document.documentElement.classList.toggle("dark", newTheme === "dark");
        return newTheme;
      });
    },
    init: () => {
      const stored = localStorage.getItem("theme") as Theme | null;
      const prefersDark = window.matchMedia(
        "(prefers-color-scheme: dark)"
      ).matches;
      const theme = stored || (prefersDark ? "dark" : "light");
      set(theme);
      document.documentElement.classList.toggle("dark", theme === "dark");
    },
  };
}

export const themeStore = createThemeStore();
