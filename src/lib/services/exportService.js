import { invoke } from "@tauri-apps/api/core";

export const exportService = {
  /**
   * Prompts user to select a path for CSV export file.
   * @param {string} defaultName 
   * @returns {Promise<string|null>} Selected path
   */
  async selectExportPath(defaultName) {
    return await invoke("select_export_path", { defaultName });
  },

  /**
   * Saves custom export settings (format, collection code, grid reference, islands settings, database backup folder, home country).
   * @param {number} userId 
   * @param {string} format DwC or BRAHMS
   * @param {string} collectionCode Collection code
   * @param {boolean} includeGridReference Whether to include grid reference
   * @param {boolean} includeIslands Whether to include islands
   * @param {string} backupLocation Backup folder path
   * @param {string} homeCountry Selected home country
   * @returns {Promise<void>}
   */
  async saveExportSettings(userId, format, collectionCode, includeGridReference, includeIslands, backupLocation, homeCountry) {
    return await invoke("save_export_settings", {
      userId,
      format,
      collectionCode,
      includeGridReference,
      includeIslands,
      backupLocation,
      homeCountry,
    });
  },

  /**
   * Retrieves export settings.
   * @param {number} userId 
   * @returns {Promise<{ format: string, collectionCode: string, includeGridReference: boolean, includeIslands: boolean, backupLocation: string }>}
   */
  async getExportSettings(userId) {
    return await invoke("get_export_settings", { userId });
  },

  /**
   * Writes exported CSV data to disk and records session export timestamp.
   * @param {number} sessionId 
   * @param {string} filepath 
   * @param {string} csvContent 
   * @returns {Promise<string>} Status message
   */
  async exportSessionCsv(sessionId, filepath, csvContent) {
    return await invoke("export_session_csv", { sessionId, filepath, csvContent });
  }
};
