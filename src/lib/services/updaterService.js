import { check } from "@tauri-apps/plugin-updater";

/**
 * @typedef {import('@tauri-apps/plugin-updater').Update} Update
 * @typedef {import('@tauri-apps/plugin-updater').DownloadEvent} DownloadEvent
 */

/** @type {Update | null} */
let activeUpdate = null;

/**
 * Service for checking and performing application updates.
 */
export const updaterService = {
  /**
   * Checks for available application updates.
   * @returns {Promise<{ available: boolean; version: string } | null>}
   */
  async checkForUpdates() {
    const update = await check();
    activeUpdate = update;
    if (update) {
      return {
        available: true,
        version: update.version
      };
    }
    return null;
  },

  /**
   * Downloads the current update.
   * @param {(event: DownloadEvent) => void} [onProgress]
   * @returns {Promise<void>}
   */
  async downloadUpdate(onProgress) {
    if (!activeUpdate) {
      throw new Error("No active update available to download");
    }
    await activeUpdate.download(onProgress);
  },

  /**
   * Installs the downloaded update, downloading it first if not already downloaded in the current session.
   * @param {(event: DownloadEvent) => void} [onProgress]
   * @returns {Promise<void>}
   */
  async installUpdate(onProgress) {
    if (!activeUpdate) {
      throw new Error("No active update available to install");
    }
    const updateAny = /** @type {any} */ (activeUpdate);
    if (!updateAny.downloadedBytes) {
      // Redownload silently or with progress in this session
      await activeUpdate.download(onProgress);
    }
    await activeUpdate.install();
  },

  /**
   * Checks if the update is ready to install (i.e. downloaded in the current session).
   * @returns {boolean}
   */
  isDownloaded() {
    if (!activeUpdate) return false;
    const updateAny = /** @type {any} */ (activeUpdate);
    return !!updateAny.downloadedBytes;
  }
};
