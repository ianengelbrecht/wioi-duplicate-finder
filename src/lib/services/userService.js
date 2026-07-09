import { invoke } from "@tauri-apps/api/core";

/**
 * @typedef {Object} UserDto
 * @property {number} id
 * @property {string} username
 * @property {string} givenName
 * @property {string} familyName
 * @property {string} initials
 * @property {boolean} isAdmin
 */

export const userService = {
  /**
   * Fetches user details by ID.
   * @param {number} id 
   * @returns {Promise<UserDto|null>}
   */
  async getUserById(id) {
    return await invoke("get_user_by_id", { id });
  },

  /**
   * Fetches all users (only accessible by administrators).
   * @param {number} callerId 
   * @returns {Promise<UserDto[]>}
   */
  async getAllUsers(callerId) {
    return await invoke("get_all_users", { callerId });
  },

  /**
   * Updates the current user's profile details.
   * @param {number} userId 
   * @param {string} givenName 
   * @param {string} familyName 
   * @param {string} initials 
   * @returns {Promise<void>}
   */
  async updateUserProfile(userId, givenName, familyName, initials) {
    return await invoke("update_user_profile", { userId, givenName, familyName, initials });
  },

  /**
   * Updates any user's profile details and admin status (only accessible by administrators).
   * @param {number} callerId 
   * @param {number} targetUserId 
   * @param {string} givenName 
   * @param {string} familyName 
   * @param {string} initials 
   * @param {boolean} isAdmin 
   * @returns {Promise<void>}
   */
  async updateUserByAdmin(callerId, targetUserId, givenName, familyName, initials, isAdmin) {
    return await invoke("update_user_by_admin", {
      callerId,
      targetUserId,
      givenName,
      familyName,
      initials,
      isAdmin
    });
  }
};
