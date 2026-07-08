export class WorkspaceStore {
  activeSession = $state(/** @type {any} */(null)); // { id, name, recordCount }
  sessionList = $state(/** @type {any[]} */([]));
  activeRecord = $state(/** @type {any} */(null)); // Specimen being captured or edited
  capturedRecords = $state(/** @type {any[]} */([])); // Captured specimens in active session

  exportFormat = $state("DwC"); // "DwC" or "BRAHMS"
  workingCollectionCode = $state("RHOIO");

  /** @type {boolean} */
  includeGridReference = $state(false);

  /** @type {boolean} */
  includeIslands = $state(false);

  databaseBackupLocation = $state("");
  defaultBackupLocation = $state("");
  showRestoreConfirmModal = $state(false);
  pendingRestorePath = $state("");
  manualBackupMessage = $state("");
  manualBackupError = $state("");
  settingsMessage = $state("");
  exportMessage = $state("");
  exportError = $state("");

  /** @type {number} */
  gbifRecordCount = $state(0);
  /** @type {number} */
  wcvpRecordCount = $state(0);

  /**
   * Getter to determine if both required datasets have been loaded.
   * @returns {boolean}
   */
  get hasRequiredDatasets() {
    return this.gbifRecordCount > 0 && this.wcvpRecordCount > 0;
  }

  /**
   * Sets the active session and stores it in local storage.
   * @param {any} session 
   */
  selectSession(session) {
    this.activeSession = session;
    if (session) {
      localStorage.setItem("lastActiveSession", JSON.stringify(session));
    } else {
      localStorage.removeItem("lastActiveSession");
    }
  }

  /**
   * Clears the active session and its related states.
   */
  clearWorkspace() {
    this.activeSession = null;
    this.activeRecord = null;
    this.capturedRecords = [];
    localStorage.removeItem("lastActiveSession");
  }
}

export const workspaceStore = new WorkspaceStore();
