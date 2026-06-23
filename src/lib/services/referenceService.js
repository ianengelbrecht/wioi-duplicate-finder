import { invoke } from "@tauri-apps/api/core";

export const referenceService = {
  /**
   * Retrieves reference dataset metadata (count, countries, collectionCodes).
   * @returns {Promise<{ recordCount: number, countries: string[], collectionCodes: string[] }>}
   */
  async getReferenceMetadata() {
    return await invoke("get_reference_metadata");
  },

  /**
   * Triggers open file dialog to select a CSV file.
   * @returns {Promise<string | null>} The selected CSV file path, or null if cancelled.
   */
  async selectCsvFile() {
    return await invoke("select_csv_file");
  },

  /**
   * Imports the reference dataset from a CSV file path.
   * @param {string} filepath The path to the CSV file.
   * @returns {Promise<void>}
   */
  async importReferenceDataset(filepath) {
    return await invoke("import_reference_dataset", { filepath });
  }
};
