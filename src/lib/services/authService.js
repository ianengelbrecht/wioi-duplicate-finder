import { invoke } from "@tauri-apps/api/core";

export const authService = {
  /**
   * Registers a new user.
   * @param {string} username 
   * @param {string} password 
   * @returns {Promise<string>}
   */
  async register(username, password) {
    return await invoke("register_user", { username, password });
  },

  /**
   * Logs in an existing user.
   * @param {string} username 
   * @param {string} password 
   * @returns {Promise<{ id: number, username: string }|null>}
   */
  async login(username, password) {
    return await invoke("login_user", { username, password });
  },

  /**
   * Initializes the SQLite database.
   * @returns {Promise<void>}
   */
  async initializeDatabase() {
    return await invoke("initialize_database");
  }
};
