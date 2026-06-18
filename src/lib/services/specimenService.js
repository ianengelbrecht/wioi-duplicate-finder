import { invoke } from "@tauri-apps/api/core";

export const specimenService = {
  /**
   * Fetches captured records for a session.
   * @param {number} sessionId 
   * @returns {Promise<any[]>}
   */
  async getCapturedRecords(sessionId) {
    return await invoke("get_captured_records", { sessionId });
  },

  /**
   * Saves or updates a captured specimen record.
   * @param {any} record 
   * @returns {Promise<{ id: number, success: boolean }>}
   */
  async saveCapturedRecord(record) {
    return await invoke("save_captured_record", { record });
  },

  /**
   * Deletes a captured specimen record.
   * @param {number} id 
   * @returns {Promise<void>}
   */
  async deleteCapturedRecord(id) {
    return await invoke("delete_captured_record", { id });
  }
};
