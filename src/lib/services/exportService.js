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
   * Saves custom export settings (format, field mappings, backups folder).
   * @param {number} userId 
   * @param {string} format DwC or BRAHMS
   * @param {string} mappings Stringified JSON object
   * @returns {Promise<void>}
   */
  async saveExportSettings(userId, format, mappings) {
    return await invoke("save_export_settings", { userId, format, mappings });
  },

  /**
   * Retrieves export settings.
   * @param {number} userId 
   * @returns {Promise<{ format: string, mappings: string }>}
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
