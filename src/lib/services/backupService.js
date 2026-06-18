import { invoke } from "@tauri-apps/api/core";

export const backupService = {
  /**
   * Retrieves default database backup directory path.
   * @returns {Promise<string>}
   */
  async getDefaultBackupDir() {
    return await invoke("get_default_backup_dir");
  },

  /**
   * Prompts user to select a folder for database backups.
   * @returns {Promise<string|null>}
   */
  async selectBackupDirectory() {
    return await invoke("select_backup_directory");
  },

  /**
   * Triggers a manual database backup immediately.
   * @returns {Promise<string>} Path to the created backup file
   */
  async performManualBackup() {
    return await invoke("perform_manual_backup");
  },

  /**
   * Prompts user to pick a SQLite database backup file to restore.
   * @returns {Promise<string|null>} Path to selected file
   */
  async selectBackupFile() {
    return await invoke("select_backup_file");
  },

  /**
   * Restores database from chosen backup file path.
   * @param {string} backupPath 
   * @returns {Promise<void>}
   */
  async restoreDatabaseFromBackup(backupPath) {
    return await invoke("restore_database_from_backup", { backupPath });
  }
};
