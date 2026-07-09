export class AuthStore {
  currentUser = $state(/** @type {any} */ (null)); // { id, username }
  view = $state("loading"); // "loading", "auth", "dashboard", "workspace", "db_restore", "db_setup"
  dbLoadingMessage = $state("Checking database and indexing reference records...");
  dbErrorMessage = $state("");
  dbSetupState = $state({ type: "", path: "", error: "" });
  authError = $state("");
  authSuccess = $state("");

  /**
   * Sets the currently logged-in user and saves to local storage.
   * @param {any} user 
   */
  setCurrentUser(user) {
    this.currentUser = user;
    if (user) {
      localStorage.setItem("currentUser", JSON.stringify(user));
    } else {
      localStorage.removeItem("currentUser");
      localStorage.removeItem("lastActiveSession");
    }
  }

  /**
   * Updates the current user's profile details in the store.
   * @param {string} givenName 
   * @param {string} familyName 
   * @param {string} initials 
   */
  updateCurrentUserDetails(givenName, familyName, initials) {
    if (this.currentUser) {
      this.currentUser.givenName = givenName;
      this.currentUser.familyName = familyName;
      this.currentUser.initials = initials;
      localStorage.setItem("currentUser", JSON.stringify(this.currentUser));
    }
  }

  /**
   * Sets the current active application view.
   * @param {string} viewName 
   */
  setView(viewName) {
    this.view = viewName;
  }
}

export const authStore = new AuthStore();
