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
   * Sets the current active application view.
   * @param {string} viewName 
   */
  setView(viewName) {
    this.view = viewName;
  }
}

export const authStore = new AuthStore();
