import { invoke } from "@tauri-apps/api/core";

export const geographyService = {
  /**
   * Autocompletes locality names using FTS5 index.
   * @param {string} query 
   * @returns {Promise<string[]>}
   */
  async autocompleteLocality(query) {
    return await invoke("autocomplete_locality", { query });
  },

  /**
   * Autocompletes hierarchical geography fields (country, stateProvince, county, municipality).
   * @param {string} field 
   * @param {string} query 
   * @param {string} country 
   * @param {string} stateProvince 
   * @param {string} county 
   * @returns {Promise<string[]>}
   */
  async autocompleteGeography(field, query, country, stateProvince, county) {
    return await invoke("autocomplete_geography", {
      field,
      query,
      country,
      stateProvince,
      county
    });
  },

  /**
   * Retrieves reference tables counts.
   * @returns {Promise<{ gbif: number, wcvp: number }>}
   */
  async getTableCounts() {
    return await invoke("get_table_counts");
  }
};
