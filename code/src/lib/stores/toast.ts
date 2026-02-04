import { writable } from "svelte/store";

export type ToastType = "success" | "error" | "warning" | "info";

export interface Toast {
  id: number;
  type: ToastType;
  message: string;
  duration: number;
}

function createToastStore() {
  const { subscribe, update } = writable<Toast[]>([]);
  let nextId = 1;

  return {
    subscribe,
    show: (type: ToastType, message: string, duration: number = 3000) => {
      const id = nextId++;
      const toast: Toast = { id, type, message, duration };

      update((toasts) => [...toasts, toast]);

      // Auto-remove after duration
      if (duration > 0) {
        setTimeout(() => {
          update((toasts) => toasts.filter((t) => t.id !== id));
        }, duration);
      }

      return id;
    },
    dismiss: (id: number) => {
      update((toasts) => toasts.filter((t) => t.id !== id));
    },
    clear: () => {
      update(() => []);
    },
  };
}

export const toastStore = createToastStore();

// Helper functions
export function showToast(type: ToastType, message: string, duration?: number) {
  return toastStore.show(type, message, duration);
}

export function successToast(message: string, duration?: number) {
  return toastStore.show("success", message, duration);
}

export function errorToast(message: string, duration?: number) {
  return toastStore.show("error", message, duration);
}

export function warningToast(message: string, duration?: number) {
  return toastStore.show("warning", message, duration);
}

export function infoToast(message: string, duration?: number) {
  return toastStore.show("info", message, duration);
}
