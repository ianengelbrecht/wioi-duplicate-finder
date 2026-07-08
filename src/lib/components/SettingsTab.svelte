<script>
  import { getContext, onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { authStore } from "$lib/stores/authStore.svelte.js";
  import { workspaceStore } from "$lib/stores/workspaceStore.svelte.js";
  import { exportService } from "$lib/services/exportService.js";
  import { backupService } from "$lib/services/backupService.js";
  import { referenceService } from "$lib/services/referenceService.js";

  const t = getContext("t");

  /**
   * @type {{
   *   onRestoreRequest?: (event: MouseEvent) => void | Promise<void>;
   * }}
   */
  let { onRestoreRequest = () => {} } = $props();

  let recordCount = $state(0);
  let countries = $state(/** @type {{ country: string, count: number }[]} */ ([]));
  let collectionCodes = $state(/** @type {{ code: string, count: number }[]} */ ([]));
  let loadingMetadata = $state(false);

  /** @type {boolean} */
  let showReferenceImportDialog = $state(false);
  /** @type {string} */
  let selectedReferenceCsvPath = $state("");
  /** @type {string} */
  let referenceImportStatus = $state("");
  /** @type {string} */
  let referenceImportError = $state("");
  /** @type {boolean} */
  let appendReferenceData = $state(false);
  /** @type {boolean} */
  let isReferenceImporting = $state(false);

  /** @type {number} */
  let wcvpRecordCount = $state(0);
  /** @type {number} */
  let wcvpVersion = $state(15);
  /** @type {boolean} */
  let loadingWcvpMetadata = $state(false);
  /** @type {number} */
  let wcvpImportVersion = $state(15);
  /** @type {boolean} */
  let isWcvpImporting = $state(false);
  /** @type {string} */
  let wcvpImportStatus = $state("");
  /** @type {string} */
  let wcvpImportError = $state("");
  /** @type {boolean} */
  let showWcvpImportDialog = $state(false);
  /** @type {string} */
  let selectedWcvpCsvPath = $state("");

  async function loadMetadata() {
    loadingMetadata = true;
    try {
      const data = await referenceService.getReferenceMetadata();
      recordCount = data.recordCount;
      workspaceStore.gbifRecordCount = data.recordCount;
      countries = data.countries;
      collectionCodes = data.collectionCodes;
    } catch (err) {
      console.error("Failed to load reference metadata:", err);
    } finally {
      loadingMetadata = false;
    }
  }

  async function loadWcvpMetadata() {
    loadingWcvpMetadata = true;
    try {
      const data = await referenceService.getWcvpMetadata();
      wcvpRecordCount = data.recordCount;
      workspaceStore.wcvpRecordCount = data.recordCount;
      wcvpVersion = data.version;
    } catch (err) {
      console.error("Failed to load WCVP metadata:", err);
    } finally {
      loadingWcvpMetadata = false;
    }
  }

  $effect(() => {
    const unlistenReference = listen("import-progress", (event) => {
      const payload = event.payload;
      if (typeof payload === "number") {
        referenceImportStatus = `Imported ${payload.toLocaleString()} records...`;
      } else {
        referenceImportStatus = payload;
      }
    });

    const unlistenWcvp = listen("wcvp-import-progress", (event) => {
      const payload = event.payload;
      if (typeof payload === "number") {
        wcvpImportStatus = `Imported ${payload.toLocaleString()} records...`;
      } else {
        wcvpImportStatus = payload;
      }
    });

    return () => {
      unlistenReference.then((fn) => fn());
      unlistenWcvp.then((fn) => fn());
    };
  });

  onMount(() => {
    loadMetadata();
    loadWcvpMetadata();
  });

  function handleOpenWcvpImportDialog() {
    showWcvpImportDialog = true;
    selectedWcvpCsvPath = "";
    wcvpImportStatus = "";
    wcvpImportError = "";
    wcvpImportVersion = wcvpVersion >= 15 ? wcvpVersion : 15;
  }

  async function handleSelectWcvpCsvFile() {
    try {
      const filepath = await referenceService.selectCsvFile();
      if (filepath) {
        selectedWcvpCsvPath = filepath;
      }
    } catch (err) {
      console.error("Failed to select WCVP CSV file:", err);
    }
  }

  async function handleExecuteWcvpImport() {
    wcvpImportStatus = "";
    wcvpImportError = "";
    
    const versionNum = parseInt(/** @type {any} */ (wcvpImportVersion), 10);
    if (isNaN(versionNum) || versionNum < 15 || !Number.isInteger(versionNum)) {
      wcvpImportError = "WCVP version must be an integer greater than or equal to 15.";
      return;
    }

    if (versionNum <= wcvpVersion) {
      wcvpImportError = t("wcvp-version-must-be-greater", `New version number (${versionNum}) must be greater than the current version number (${wcvpVersion}).`);
      return;
    }

    if (!selectedWcvpCsvPath) {
      wcvpImportError = "Please select a WCVP CSV file.";
      return;
    }

    try {
      isWcvpImporting = true;
      wcvpImportStatus = "Opening file and starting WCVP import process...";
      
      await referenceService.importWcvpDataset(selectedWcvpCsvPath, versionNum);
      
      wcvpImportStatus = "WCVP dataset imported and updated successfully!";
      await loadWcvpMetadata();
      setTimeout(() => {
        showWcvpImportDialog = false;
        selectedWcvpCsvPath = "";
        wcvpImportStatus = "";
      }, 1500);
    } catch (e) {
      wcvpImportError = "Failed to import WCVP dataset: " + (/** @type {any} */ (e)).toString();
      wcvpImportStatus = "";
    } finally {
      isWcvpImporting = false;
    }
  }

  function handleOpenReferenceImportDialog() {
    showReferenceImportDialog = true;
    selectedReferenceCsvPath = "";
    referenceImportStatus = "";
    referenceImportError = "";
    appendReferenceData = false;
  }

  async function handleSelectReferenceCsvFile() {
    try {
      const filepath = await referenceService.selectCsvFile();
      if (filepath) {
        selectedReferenceCsvPath = filepath;
      }
    } catch (err) {
      console.error("Failed to select reference CSV file:", err);
    }
  }

  async function handleExecuteReferenceImport() {
    referenceImportStatus = "";
    referenceImportError = "";

    if (!selectedReferenceCsvPath) {
      referenceImportError = "Please select a reference CSV file.";
      return;
    }

    try {
      isReferenceImporting = true;
      referenceImportStatus = "Opening file and starting import process...";
      
      await referenceService.importReferenceDataset(selectedReferenceCsvPath, appendReferenceData);
      
      referenceImportStatus = "Reference dataset imported successfully!";
      await loadMetadata();
      setTimeout(() => {
        showReferenceImportDialog = false;
        selectedReferenceCsvPath = "";
        referenceImportStatus = "";
      }, 1500);
    } catch (e) {
      referenceImportError = "Failed to import reference dataset: " + (/** @type {any} */ (e)).toString();
      referenceImportStatus = "";
    } finally {
      isReferenceImporting = false;
    }
  }

  async function handleSaveSettings() {
    if (!authStore.currentUser) return;
    workspaceStore.settingsMessage = "";
    try {
      let mappingsObj = { 
        ...workspaceStore.customMappings, 
        collectionCode: workspaceStore.workingCollectionCode, 
        includeGridReference: workspaceStore.includeGridReference,
        backupLocation: workspaceStore.databaseBackupLocation.trim()
      };
      await exportService.saveExportSettings(
        authStore.currentUser.id,
        workspaceStore.exportFormat,
        JSON.stringify(mappingsObj)
      );
      workspaceStore.settingsMessage = "Settings saved successfully!";
      setTimeout(() => { workspaceStore.settingsMessage = ""; }, 3000);
    } catch (e) {
      workspaceStore.settingsMessage = "Error saving settings: " + (/** @type {any} */ (e)).toString();
    }
  }

  async function handleChooseBackupDirectory() {
    try {
      const path = await backupService.selectBackupDirectory();
      if (path) {
        workspaceStore.databaseBackupLocation = /** @type {string} */ (path);
      }
    } catch (err) {
      console.error("Failed to choose backup folder:", err);
    }
  }

  async function handleManualBackup() {
    workspaceStore.manualBackupMessage = "";
    workspaceStore.manualBackupError = "";
    try {
      const path = await backupService.performManualBackup();
      workspaceStore.manualBackupMessage = "Backup created successfully at: " + path;
      setTimeout(() => { workspaceStore.manualBackupMessage = ""; }, 5000);
    } catch (e) {
      workspaceStore.manualBackupError = "Backup failed: " + (/** @type {any} */ (e)).toString();
    }
  }
</script>

<div class="space-y-6">
  <div>
    <h2 data-i18n-key="application-settings" class="text-md font-bold text-slate-900 uppercase tracking-wide font-outfit">{t("application-settings", "Application settings")}</h2>
    <p data-i18n-key="settings-description" class="text-xs text-slate-500 mt-1 font-inter">{t("settings-description", "Configure the collection code and export format for your herbarium.")}</p>
  </div>

  {#if !workspaceStore.hasRequiredDatasets}
    <div class="p-4 bg-amber-50 border border-amber-300 text-amber-900 text-xs font-inter leading-relaxed flex items-start gap-3 rounded-none">
      <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5 text-amber-700 shrink-0 mt-0.5">
        <path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126zM12 15.75h.007v.008H12v-.008z" />
      </svg>
      <div>
        <strong class="uppercase font-bold tracking-wider block mb-1">Configuration Required</strong>
        Please import both the <strong>Reference Dataset (GBIF)</strong> and the <strong>WCVP Taxonomy Dataset</strong> using the options below. You will not be able to access capture sessions or capture specimen records until both datasets are loaded.
      </div>
    </div>
  {/if}

  {#if workspaceStore.settingsMessage}
    <div class="p-3 text-xs bg-emerald-50 border border-emerald-300 text-emerald-800 font-medium font-inter">
      {workspaceStore.settingsMessage}
    </div>
  {/if}

  <!-- Reference Dataset Setting -->
  <div class="space-y-2 pt-2 border-t border-slate-100 bg-white rounded-none">
    <div class="">
      <h3 data-i18n-key="reference-dataset" class="block text-xs font-bold text-slate-700 uppercase tracking-wider">
        {t("reference-dataset", "Reference Dataset")}
      </h3>
    </div>

    <!-- Stats -->
    <div class="grid grid-cols-1 md:grid-cols-3 gap-4 bg-slate-50 p-3 border border-slate-200">
      <div>
        <div data-i18n-key="record-count-label" class="text-[10px] font-bold text-slate-500 uppercase tracking-wider font-inter">
          {t("record-count-label", "Total Records")}
        </div>
        <div class="text-lg font-bold text-slate-900 mt-0.5 font-outfit">
          {loadingMetadata ? "..." : recordCount.toLocaleString()}
        </div>
      </div>
      <div>
        <div data-i18n-key="countries-label" class="text-[10px] font-bold text-slate-500 uppercase tracking-wider font-inter">
          {t("countries-label", "Countries")}
        </div>
        <div class="text-xs font-medium text-slate-800 mt-1 max-h-24 overflow-y-auto font-inter">
          {#if loadingMetadata}
            ...
          {:else if countries.length === 0}
            <span data-i18n-key="no-countries">{t("no-countries", "None")}</span>
          {:else}
            <div class="flex flex-wrap gap-1 mt-0.5">
              {#each countries as item}
                <span class="bg-slate-200 text-slate-700 px-1.5 py-0.5 text-[10px] font-semibold">{item.country} ({item.count.toLocaleString()})</span>
              {/each}
            </div>
          {/if}
        </div>
      </div>
      <div>
        <div data-i18n-key="collections-label" class="text-[10px] font-bold text-slate-500 uppercase tracking-wider font-inter">
          {t("collections-label", "Reference Collections")}
        </div>
        <div class="text-xs font-medium text-slate-800 mt-1 max-h-24 overflow-y-auto font-inter">
          {#if loadingMetadata}
            ...
          {:else if collectionCodes.length === 0}
            <span data-i18n-key="no-collections">{t("no-collections", "None")}</span>
          {:else}
            <div class="flex flex-wrap gap-1 mt-0.5">
              {#each collectionCodes as item}
                <span class="bg-slate-200 text-slate-700 px-1.5 py-0.5 text-[10px] font-semibold">{item.code} ({item.count.toLocaleString()})</span>
              {/each}
            </div>
          {/if}
        </div>
      </div>
    </div>

    <!-- Action Button -->
    <div class="flex flex-col sm:flex-row items-end sm:items-center justify-end gap-3 pt-2">
      <button
        type="button"
        data-i18n-key="load-new-dataset"
        onclick={handleOpenReferenceImportDialog}
        class="bg-slate-100 hover:bg-slate-200 text-slate-700 px-4 py-2 text-xs font-bold uppercase tracking-wider rounded-none transition-colors cursor-pointer font-inter"
      >
        {t("load-new-dataset", "Load New Dataset")}
      </button>
    </div>
  </div>

  <!-- WCVP Dataset Setting -->
  <div class="space-y-2 pt-2 border-t border-slate-100 bg-white rounded-none">
    <div class="">
      <h3 data-i18n-key="wcvp-dataset" class="block text-xs font-bold text-slate-700 uppercase tracking-wider">
        {t("wcvp-dataset", "WCVP Dataset")}
      </h3>
    </div>

    <!-- Stats -->
    <div class="grid grid-cols-1 md:grid-cols-2 gap-4 bg-slate-50 p-3 border border-slate-200">
      <div>
        <div data-i18n-key="wcvp-version-label" class="text-[10px] font-bold text-slate-500 uppercase tracking-wider font-inter">
          {t("wcvp-version-label", "Current Version")}
        </div>
        <div class="text-lg font-bold text-slate-900 mt-0.5 font-outfit">
          WCVP v{loadingWcvpMetadata ? "..." : wcvpVersion}
        </div>
      </div>
      <div>
        <div data-i18n-key="wcvp-count-label" class="text-[10px] font-bold text-slate-500 uppercase tracking-wider font-inter">
          {t("wcvp-count-label", "Total Taxa")}
        </div>
        <div class="text-lg font-bold text-slate-900 mt-0.5 font-outfit">
          {loadingWcvpMetadata ? "..." : wcvpRecordCount.toLocaleString()}
        </div>
      </div>
    </div>

    <!-- Action Button -->
    <div class="flex flex-col sm:flex-row items-end sm:items-center justify-end gap-3 pt-2">
      <button
        type="button"
        data-i18n-key="import-wcvp-btn"
        onclick={handleOpenWcvpImportDialog}
        class="bg-slate-100 hover:bg-slate-200 text-slate-700 px-4 py-2 text-xs font-bold uppercase tracking-wider rounded-none transition-colors cursor-pointer font-inter"
      >
        {t("import-wcvp-btn", "Import/Update WCVP")}
      </button>
    </div>
  </div>

  <!-- Collection Code Setting -->
  <div class="space-y-2 pt-2 border-t border-slate-100">
    <label for="settings-collectionCode" data-i18n-key="working-collection-code" class="block text-xs font-bold text-slate-700 uppercase tracking-wider">{t("working-collection-code", "Working Collection Code")}</label>
    <input
      id="settings-collectionCode"
      type="text"
      placeholder="e.g. TAN"
      bind:value={workspaceStore.workingCollectionCode}
      class="w-full sm:w-64 bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
    />
  </div>

  <!-- Format Choice -->
  <div class="space-y-2">
    <div>
      <span data-i18n-key="export-format" class="text-xs font-bold text-slate-700 uppercase tracking-wider">{t("export-format", "Export Format ")}</span>
      <span data-i18n-key="export-format-sub" class="text-xs text-slate-500">{t("export-format-sub", "(files are exported as comma separated values -- CSV).")}</span>
    </div>
    
    <div class="flex gap-4">
      <label class="flex items-center gap-2 text-xs font-medium text-slate-700 cursor-pointer">
        <input
          type="radio"
          name="export-format"
          value="DwC"
          bind:group={workspaceStore.exportFormat}
          class="text-slate-800"
        />
        <span>Darwin Core</span>
      </label>
      <label class="flex items-center gap-2 text-xs font-medium text-slate-700 cursor-pointer">
        <input
          type="radio"
          name="export-format"
          value="BRAHMS"
          bind:group={workspaceStore.exportFormat}
          class="text-slate-800"
        />
        <span>BRAHMS7</span>
      </label>
    </div>
  </div>

  <!-- Grid Reference Setting -->
  <div class="space-y-2 pt-2">
    <label class="flex items-center gap-2 text-xs font-bold text-slate-700 uppercase tracking-wider cursor-pointer">
      <input
        id="settings-qds"
        type="checkbox"
        bind:checked={workspaceStore.includeGridReference}
        class="w-4 h-4 text-slate-855 border-slate-300 rounded focus:ring-slate-500 focus:ring-1 cursor-pointer"
      />
      <span data-i18n-key="include-qds-label">{t("include-qds-label", "Include grid reference (QDS)")}</span>
    </label>
  </div>

  <!-- Database Backup Location Setting -->
  <div class="space-y-2 pt-2 border-t border-slate-100">
    <label for="settings-backup-location" class="block text-xs font-bold text-slate-700 uppercase tracking-wider">
      {t("backup-location-label", "Database Backup Location")}
    </label>
    <div class="flex gap-2">
      <input
        id="settings-backup-location"
        type="text"
        bind:value={workspaceStore.databaseBackupLocation}
        class="flex-1 bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
        placeholder="Database backup folder path"
      />
      <button
        type="button"
        onclick={handleChooseBackupDirectory}
        class="bg-slate-100 hover:bg-slate-200 text-slate-700 border border-slate-300 px-4 py-2 text-xs font-semibold rounded-none transition-colors"
      >
        {t("choose-folder-btn", "Choose Folder")}
      </button>
    </div>
    <div class="flex justify-end gap-2 mt-2 pt-1">
      <button
        type="button"
        onclick={onRestoreRequest}
        class="bg-slate-100 hover:bg-slate-200 text-slate-700 px-4 py-2 text-xs font-bold uppercase tracking-wider rounded-none transition-colors cursor-pointer"
      >
        {t("restore-backup-btn", "Restore Backup")}
      </button>
      <button
        type="button"
        onclick={handleManualBackup}
        class="bg-slate-100 hover:bg-slate-200 text-slate-700  px-4 py-2 text-xs font-bold uppercase tracking-wider rounded-none transition-colors cursor-pointer"
      >
        {t("backup-now-btn", "Back Up Now")}
      </button>
    </div>
    {#if workspaceStore.manualBackupMessage}
      <p class="text-xs text-green-700 font-medium mt-1.5 leading-relaxed">{workspaceStore.manualBackupMessage}</p>
    {/if}
    {#if workspaceStore.manualBackupError}
      <p class="text-xs text-red-700 font-medium mt-1.5 leading-relaxed">{workspaceStore.manualBackupError}</p>
    {/if}
  </div>

  <!-- Save settings button -->
  <div class="pt-4 border-t border-slate-100 flex justify-end">
    <button
      type="button"
      data-i18n-key="save-settings-btn"
      onclick={handleSaveSettings}
      class="bg-slate-800 hover:bg-slate-900 text-white px-6 py-2 text-xs font-bold uppercase tracking-wider rounded-none transition-colors"
    >
      {t("save-settings-btn", "Save Settings")}
    </button>
  </div>

  {#if showWcvpImportDialog}
    <div 
      class="fixed inset-0 z-[120] flex items-center justify-center bg-slate-900/40 backdrop-blur-xs p-4"
      role="dialog"
      aria-modal="true"
      tabindex="-1"
      onclick={(e) => { if (e.target === e.currentTarget && !isWcvpImporting) showWcvpImportDialog = false; }}
      onkeydown={(e) => { 
        if (e.key === "Escape" && !isWcvpImporting) {
          e.preventDefault();
          showWcvpImportDialog = false; 
        }
      }}
    >
      <div class="bg-white border border-slate-200 shadow-2xl max-w-md w-full p-5 flex flex-col gap-4 rounded-none font-inter">
        <div>
          <h3 data-i18n-key="wcvp-import-dialog-title" class="font-bold text-slate-900 uppercase text-xs tracking-wider">{t("wcvp-import-dialog-title", "Import/Update WCVP Checklist")}</h3>
        </div>

        <div class="space-y-3">
          <div class="flex flex-col gap-1">
            <label for="wcvp-dialog-version" class="text-[10px] font-bold text-slate-500 uppercase tracking-wider">
              {t("wcvp-import-version-label", "Import Version Number")}
            </label>
            <input
              id="wcvp-dialog-version"
              type="number"
              min="15"
              step="1"
              disabled={isWcvpImporting}
              bind:value={wcvpImportVersion}
              placeholder="e.g. 15"
              class="w-full bg-white border border-slate-300 text-slate-800 text-xs px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all font-inter"
            />
          </div>

          <div class="flex flex-col gap-1">
            <span class="text-[10px] font-bold text-slate-500 uppercase tracking-wider">
              {t("wcvp-dialog-file-label", "Select WCVP File")}
            </span>
            <div class="flex gap-2">
              <input
                type="text"
                readonly
                placeholder={t("wcvp-dialog-no-file", "No file selected")}
                bind:value={selectedWcvpCsvPath}
                class="flex-1 bg-slate-50 border border-slate-300 text-slate-600 text-xs px-3 py-2 outline-none rounded-none font-inter"
              />
              <button
                type="button"
                disabled={isWcvpImporting}
                onclick={handleSelectWcvpCsvFile}
                class="bg-slate-100 hover:bg-slate-200 text-slate-700 border border-slate-300 px-3 py-1.5 text-xs font-semibold rounded-none transition-colors cursor-pointer font-inter"
              >
                {t("choose-file-btn", "Choose File")}
              </button>
            </div>
          </div>
        </div>

        {#if wcvpImportStatus}
          <p class="text-xs text-emerald-700 font-semibold leading-relaxed mt-1 font-inter">{wcvpImportStatus}</p>
        {/if}
        {#if wcvpImportError}
          <p class="text-xs text-red-700 font-semibold leading-relaxed mt-1 font-inter">{wcvpImportError}</p>
        {/if}

        <div class="flex justify-end gap-2 mt-2 pt-2 border-t border-slate-100">
          <button
            type="button"
            data-i18n-key="cancel-btn"
            disabled={isWcvpImporting}
            onclick={() => { showWcvpImportDialog = false; }}
            class="px-3 py-1.5 text-xs font-semibold text-slate-500 hover:bg-slate-50 border border-slate-200 transition-colors cursor-pointer rounded-none font-inter"
          >
            {t("cancel-btn", "Cancel")}
          </button>
          <button
            type="button"
            data-i18n-key="start-import-btn"
            disabled={isWcvpImporting || !selectedWcvpCsvPath || wcvpImportVersion < 15}
            onclick={handleExecuteWcvpImport}
            class="px-4 py-1.5 text-xs font-bold uppercase tracking-wider text-white bg-slate-800 hover:bg-slate-900 transition-colors cursor-pointer rounded-none font-inter"
          >
            {isWcvpImporting ? t("importing-dataset", "Importing...") : t("start-import-btn", "Start Import")}
          </button>
        </div>
      </div>
    </div>
  {/if}

  {#if showReferenceImportDialog}
    <div 
      class="fixed inset-0 z-[120] flex items-center justify-center bg-slate-900/40 backdrop-blur-xs p-4"
      role="dialog"
      aria-modal="true"
      tabindex="-1"
      onclick={(e) => { if (e.target === e.currentTarget && !isReferenceImporting) showReferenceImportDialog = false; }}
      onkeydown={(e) => { 
        if (e.key === "Escape" && !isReferenceImporting) {
          e.preventDefault();
          showReferenceImportDialog = false; 
        }
      }}
    >
      <div class="bg-white border border-slate-200 shadow-2xl max-w-md w-full p-5 flex flex-col gap-4 rounded-none font-inter">
        <div>
          <h3 data-i18n-key="reference-import-dialog-title" class="font-bold text-slate-900 uppercase text-xs tracking-wider">{t("reference-import-dialog-title", "Import/Update Reference Dataset")}</h3>
        </div>

        <div class="space-y-3">
          <!-- Short note -->
          <div class="p-3 bg-slate-50 border border-slate-200 text-xs text-slate-600 leading-relaxed font-inter">
            {t("reference-import-note", "Existing reference data will be overwritten unless appended. Appending is a way to manage the reference data that are most appropriate for your herbarium.")}
          </div>

          <!-- Select File -->
          <div class="flex flex-col gap-1">
            <span class="text-[10px] font-bold text-slate-500 uppercase tracking-wider">
              {t("reference-dialog-file-label", "Select Reference CSV File")}
            </span>
            <div class="flex gap-2">
              <input
                type="text"
                readonly
                placeholder={t("reference-dialog-no-file", "No file selected")}
                bind:value={selectedReferenceCsvPath}
                class="flex-1 bg-slate-50 border border-slate-300 text-slate-600 text-xs px-3 py-2 outline-none rounded-none font-inter"
              />
              <button
                type="button"
                disabled={isReferenceImporting}
                onclick={handleSelectReferenceCsvFile}
                class="bg-slate-100 hover:bg-slate-200 text-slate-700 border border-slate-300 px-3 py-1.5 text-xs font-semibold rounded-none transition-colors cursor-pointer font-inter"
              >
                {t("choose-file-btn", "Choose File")}
              </button>
            </div>
          </div>

          <!-- Append Option -->
          <div class="flex items-center gap-2 pt-1">
            <input
              id="reference-dialog-append"
              type="checkbox"
              disabled={isReferenceImporting}
              bind:checked={appendReferenceData}
              class="w-4 h-4 text-slate-800 accent-slate-800 border-slate-300 rounded focus:ring-slate-500 focus:ring-1 cursor-pointer"
            />
            <label for="reference-dialog-append" class="text-xs font-bold text-slate-700 uppercase tracking-wider cursor-pointer">
              {t("reference-append-label", "Append to existing dataset")}
            </label>
          </div>
        </div>

        {#if referenceImportStatus}
          <p class="text-xs text-emerald-700 font-semibold leading-relaxed mt-1 font-inter">{referenceImportStatus}</p>
        {/if}
        {#if referenceImportError}
          <p class="text-xs text-red-700 font-semibold leading-relaxed mt-1 font-inter">{referenceImportError}</p>
        {/if}

        <div class="flex justify-end gap-2 mt-2 pt-2 border-t border-slate-100">
          <button
            type="button"
            data-i18n-key="cancel-btn"
            disabled={isReferenceImporting}
            onclick={() => { showReferenceImportDialog = false; }}
            class="px-3 py-1.5 text-xs font-semibold text-slate-500 hover:bg-slate-50 border border-slate-200 transition-colors cursor-pointer rounded-none font-inter"
          >
            {t("cancel-btn", "Cancel")}
          </button>
          <button
            type="button"
            data-i18n-key="start-import-btn"
            disabled={isReferenceImporting || !selectedReferenceCsvPath}
            onclick={handleExecuteReferenceImport}
            class="px-4 py-1.5 text-xs font-bold uppercase tracking-wider text-white bg-slate-800 hover:bg-slate-900 transition-colors cursor-pointer rounded-none font-inter"
          >
            {isReferenceImporting ? t("importing-dataset", "Importing...") : t("start-import-btn", "Start Import")}
          </button>
        </div>
      </div>
    </div>
  {/if}
</div>
