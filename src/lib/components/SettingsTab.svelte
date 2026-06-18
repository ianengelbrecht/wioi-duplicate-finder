<script>
  import { getContext } from "svelte";
  import { authStore } from "$lib/stores/authStore.svelte.js";
  import { workspaceStore } from "$lib/stores/workspaceStore.svelte.js";
  import { exportService } from "$lib/services/exportService.js";
  import { backupService } from "$lib/services/backupService.js";

  const t = getContext("t");

  /**
   * @type {{
   *   onRestoreRequest?: (event: MouseEvent) => void | Promise<void>;
   * }}
   */
  let { onRestoreRequest = () => {} } = $props();

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
    <h2 data-i18n-key="application-settings" class="text-md font-bold text-slate-900 uppercase tracking-wide">{t("application-settings", "Application settings")}</h2>
    <p data-i18n-key="settings-description" class="text-xs text-slate-500 mt-1">{t("settings-description", "Configure the collection code and export format for your herbarium.")}</p>
  </div>

  {#if workspaceStore.settingsMessage}
    <div class="p-3 text-xs bg-emerald-50 border border-emerald-300 text-emerald-800 font-medium">
      {workspaceStore.settingsMessage}
    </div>
  {/if}

  <!-- Collection Code Setting -->
  <div class="space-y-2">
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
