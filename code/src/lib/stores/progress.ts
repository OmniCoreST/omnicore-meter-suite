import { writable, derived } from "svelte/store";

export interface ProgressStep {
  id: string;
  label: string;
  status: "pending" | "in_progress" | "completed" | "error";
}

export interface ProgressState {
  active: boolean;
  title: string;
  steps: ProgressStep[];
  currentStepIndex: number;
  percentage: number;
  startTime: number | null;
  elapsedTime: number;
  error: string | null;
  canCancel: boolean;
}

const initialState: ProgressState = {
  active: false,
  title: "",
  steps: [],
  currentStepIndex: -1,
  percentage: 0,
  startTime: null,
  elapsedTime: 0,
  error: null,
  canCancel: true,
};

function createProgressStore() {
  const { subscribe, set, update } = writable<ProgressState>(initialState);
  let timerInterval: ReturnType<typeof setInterval> | null = null;

  const startTimer = () => {
    if (timerInterval) clearInterval(timerInterval);
    timerInterval = setInterval(() => {
      update((state) => {
        if (state.startTime) {
          return {
            ...state,
            elapsedTime: (Date.now() - state.startTime) / 1000,
          };
        }
        return state;
      });
    }, 100);
  };

  const stopTimer = () => {
    if (timerInterval) {
      clearInterval(timerInterval);
      timerInterval = null;
    }
  };

  return {
    subscribe,
    start: (title: string, steps: Array<{ id: string; label: string }>) => {
      const progressSteps: ProgressStep[] = steps.map((s) => ({
        ...s,
        status: "pending",
      }));

      set({
        active: true,
        title,
        steps: progressSteps,
        currentStepIndex: -1,
        percentage: 0,
        startTime: Date.now(),
        elapsedTime: 0,
        error: null,
        canCancel: true,
      });

      startTimer();
    },
    nextStep: () => {
      update((state) => {
        const newIndex = state.currentStepIndex + 1;
        const updatedSteps = state.steps.map((step, i) => {
          if (i < newIndex) return { ...step, status: "completed" as const };
          if (i === newIndex) return { ...step, status: "in_progress" as const };
          return step;
        });

        const percentage = Math.round(((newIndex + 1) / state.steps.length) * 100);

        return {
          ...state,
          steps: updatedSteps,
          currentStepIndex: newIndex,
          percentage: Math.min(percentage, 99),
        };
      });
    },
    setStep: (stepId: string) => {
      update((state) => {
        const stepIndex = state.steps.findIndex((s) => s.id === stepId);
        if (stepIndex === -1) return state;

        const updatedSteps = state.steps.map((step, i) => {
          if (i < stepIndex) return { ...step, status: "completed" as const };
          if (i === stepIndex) return { ...step, status: "in_progress" as const };
          return { ...step, status: "pending" as const };
        });

        const percentage = Math.round(((stepIndex + 1) / state.steps.length) * 100);

        return {
          ...state,
          steps: updatedSteps,
          currentStepIndex: stepIndex,
          percentage: Math.min(percentage, 99),
        };
      });
    },
    complete: () => {
      update((state) => ({
        ...state,
        steps: state.steps.map((s) => ({ ...s, status: "completed" as const })),
        currentStepIndex: state.steps.length - 1,
        percentage: 100,
        canCancel: false,
      }));
      stopTimer();
    },
    setError: (error: string) => {
      update((state) => {
        const updatedSteps = state.steps.map((step, i) => {
          if (i === state.currentStepIndex) return { ...step, status: "error" as const };
          return step;
        });
        return {
          ...state,
          steps: updatedSteps,
          error,
          canCancel: false,
        };
      });
      stopTimer();
    },
    cancel: () => {
      stopTimer();
      set(initialState);
    },
    reset: () => {
      stopTimer();
      set(initialState);
    },
  };
}

export const progressStore = createProgressStore();

export const isProgressActive = derived(
  progressStore,
  ($progress) => $progress.active
);
