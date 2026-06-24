import { invoke } from "@tauri-apps/api/core";

export const databaseService = {
  /**
   * Prompts the user to select an existing SQLite database file.
   * @returns {Promise<string|null>} Path to the selected database file
   */
  async selectDatabaseFile() {
    return await invoke("select_database_file");
  },

  /**
   * Configures the application to use the database file at the specified path.
   * @param {string} path 
   * @returns {Promise<void>}
   */
  async configureDatabase(path) {
    return await invoke("configure_database", { path });
  }
};
