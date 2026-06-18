import { invoke } from "@tauri-apps/api/core";

export const sessionService = {
  /**
   * Fetches sessions list for a user.
   * @param {number} userId 
   * @returns {Promise<any[]>}
   */
  async getSessions(userId) {
    return await invoke("get_sessions", { userId });
  },

  /**
   * Creates a new session.
   * @param {number} userId 
   * @param {string} name 
   * @returns {Promise<any>}
   */
  async createSession(userId, name) {
    return await invoke("create_session", { userId, name });
  },

  /**
   * Renames a session.
   * @param {number} id 
   * @param {string} name 
   * @returns {Promise<void>}
   */
  async renameSession(id, name) {
    return await invoke("rename_session", { id, name });
  },

  /**
   * Deletes a session and all its captured records.
   * @param {number} id 
   * @returns {Promise<void>}
   */
  async deleteSession(id) {
    return await invoke("delete_session", { id });
  }
};
