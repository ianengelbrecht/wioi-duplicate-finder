import { invoke } from "@tauri-apps/api/core";

export const taxonomyService = {
  /**
   * Searches reference records with search filters.
   * @param {any} filters 
   * @returns {Promise<any[]>}
   */
  async searchReference(filters) {
    return await invoke("search_reference", { filters });
  },

  /**
   * Autocompletes scientific names from wcvp taxonomy checklist.
   * @param {string} query 
   * @returns {Promise<any[]>}
   */
  async autocompleteScientificName(query) {
    return await invoke("autocomplete_scientific_name", { query });
  },

  /**
   * Resolves families for list of specimens using taxonomy tree.
   * @param {any[]} queries 
   * @returns {Promise<Record<string, string>>}
   */
  async resolveWcvpFamilies(queries) {
    return await invoke("resolve_wcvp_families", { queries });
  }
};
