<script>
  import { getContext, onMount } from "svelte";
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
  let countries = $state(/** @type {string[]} */ ([]));
  let collectionCodes = $state(/** @type {string[]} */ ([]));
  let loadingMetadata = $state(false);
  let importStatus = $state("");
  let importError = $state("");
  let isImporting = $state(false);

  async function loadMetadata() {
    loadingMetadata = true;
    try {
      const data = await referenceService.getReferenceMetadata();
      recordCount = data.recordCount;
      countries = data.countries;
      collectionCodes = data.collectionCodes;
    } catch (err) {
      console.error("Failed to load reference metadata:", err);
    } finally {
      loadingMetadata = false;
    }
  }

  onMount(() => {
    loadMetadata();
  });

  async function handleImportReferenceCsv() {
    importStatus = "";
    importError = "";
    try {
      const filepath = await referenceService.selectCsvFile();
      if (!filepath) {
        return;
      }
      isImporting = true;
      importStatus = "Opening file and starting import process...";
      
      await referenceService.importReferenceDataset(filepath);
      
      importStatus = "Reference dataset imported successfully!";
      await loadMetadata();
    } catch (e) {
      importError = "Failed to import reference dataset: " + (/** @type {any} */ (e)).toString();
      importStatus = "";
    } finally {
      isImporting = false;
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
              {#each countries as country}
                <span class="bg-slate-200 text-slate-700 px-1.5 py-0.5 text-[10px] font-semibold">{country}</span>
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
              {#each collectionCodes as code}
                <span class="bg-slate-200 text-slate-700 px-1.5 py-0.5 text-[10px] font-semibold">{code}</span>
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
        onclick={handleImportReferenceCsv}
        disabled={isImporting}
        class="bg-slate-100 hover:bg-slate-200 text-slate-700 disabled:text-slate-400 px-4 py-2 text-xs font-bold uppercase tracking-wider rounded-none transition-colors cursor-pointer font-inter"
      >
        {isImporting ? t("importing-dataset", "Importing...") : t("load-new-dataset", "Load New Dataset")}
      </button>
      {#if isImporting}
        <span data-i18n-key="import-in-progress" class="text-xs text-slate-500 animate-pulse font-inter">{t("import-in-progress", "Processing CSV, normalizing data, and rebuilding indexes. Please wait...")}</span>
      {/if}
    </div>

    {#if importStatus}
      <p class="text-xs text-emerald-700 font-medium leading-relaxed mt-2 font-inter">{importStatus}</p>
    {/if}
    {#if importError}
      <p class="text-xs text-red-700 font-medium leading-relaxed mt-2 font-inter">{importError}</p>
    {/if}
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
        <span>BRAHMS</span>
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
</div>
