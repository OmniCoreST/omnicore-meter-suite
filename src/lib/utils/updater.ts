/**
 * Auto-updater utilities for Tauri
 */

import { isTauri } from "./tauri";

export interface UpdateInfo {
  version: string;
  date: string;
  body: string;
}

export interface UpdateProgress {
  downloaded: number;
  total: number;
}

let checkPromise: ReturnType<typeof import("@tauri-apps/plugin-updater").check> | null = null;

/**
 * Check for available updates
 */
export async function checkForUpdates(): Promise<UpdateInfo | null> {
  if (!isTauri()) {
    // Mock update info for development
    return null;
  }

  try {
    const { check } = await import("@tauri-apps/plugin-updater");
    const update = await check();

    if (update) {
      checkPromise = Promise.resolve(update);
      return {
        version: update.version,
        date: update.date || new Date().toISOString(),
        body: update.body || "",
      };
    }

    return null;
  } catch (error) {
    console.error("Failed to check for updates:", error);
    return null;
  }
}

/**
 * Download and install the update
 */
export async function downloadAndInstall(
  onProgress?: (progress: UpdateProgress) => void
): Promise<void> {
  if (!isTauri()) {
    // Mock download for development
    if (onProgress) {
      for (let i = 0; i <= 100; i += 10) {
        await new Promise((r) => setTimeout(r, 200));
        onProgress({ downloaded: i, total: 100 });
      }
    }
    return;
  }

  try {
    const { check } = await import("@tauri-apps/plugin-updater");
    const update = await check();

    if (!update) {
      throw new Error("No update available");
    }

    let downloaded = 0;
    let contentLength = 0;

    await update.downloadAndInstall((event) => {
      if (event.event === "Started") {
        contentLength = event.data.contentLength || 0;
      } else if (event.event === "Progress") {
        downloaded += event.data.chunkLength;
        if (onProgress && contentLength > 0) {
          onProgress({
            downloaded: Math.round((downloaded / contentLength) * 100),
            total: 100,
          });
        }
      } else if (event.event === "Finished") {
        if (onProgress) {
          onProgress({ downloaded: 100, total: 100 });
        }
      }
    });
  } catch (error) {
    console.error("Failed to download and install update:", error);
    throw error;
  }
}

/**
 * Restart the application to apply the update
 */
export async function relaunchApp(): Promise<void> {
  if (!isTauri()) {
    console.log("Mock: Relaunching application...");
    return;
  }

  try {
    const { relaunch } = await import("@tauri-apps/plugin-process");
    await relaunch();
  } catch (error) {
    console.error("Failed to relaunch app:", error);
    throw error;
  }
}
