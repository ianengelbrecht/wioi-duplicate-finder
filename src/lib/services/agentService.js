import { invoke } from "@tauri-apps/api/core";

export const agentService = {
  /**
   * Autocompletes unique collector names from recordedBy values.
   * @param {string} query 
   * @returns {Promise<string[]>}
   */
  async autocompleteRecordedBy(query) {
    return await invoke("autocomplete_recorded_by", { query });
  },

  /**
   * Autocompletes unique agent names from agents table.
   * @param {string} query 
   * @returns {Promise<string[]>}
   */
  async autocompleteAgent(query) {
    return await invoke("autocomplete_agent", { query });
  },

  /**
   * Checks if an agent already exists in the agents table.
   * @param {string} name 
   * @returns {Promise<boolean>}
   */
  async checkAgentExists(name) {
    return await invoke("check_agent_exists", { name });
  },

  /**
   * Manually adds a new agent to the agents table.
   * @param {string} name 
   * @returns {Promise<void>}
   */
  async addAgent(name) {
    return await invoke("add_agent", { name });
  }
};
