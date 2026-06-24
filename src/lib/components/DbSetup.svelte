<script>
  import { getContext } from "svelte";
  import { authStore } from "$lib/stores/authStore.svelte.js";
  import { databaseService } from "$lib/services/databaseService.js";

  const t = getContext("t");

  /**
   * @type {{
   *   onRetry?: () => void | Promise<void>;
   * }}
   */
  let {
    onRetry = () => {}
  } = $props();

  let isProcessing = $state(false);
  let errorMsg = $state("");

  async function handleSelectDatabase() {
    isProcessing = true;
    errorMsg = "";
    try {
      const path = await databaseService.selectDatabaseFile();
      if (path) {
        await databaseService.configureDatabase(path);
        // Successful configuration, trigger retry/recheck to progress
        await onRetry();
      }
    } catch (e) {
      console.error(e);
      errorMsg = (/** @type {any} */ (e)).toString();
    } finally {
      isProcessing = false;
    }
  }
</script>

<div class="flex-1 flex flex-col justify-center items-center p-6 bg-slate-50 font-sans">
  <div class="w-full max-w-lg bg-white border border-slate-300 shadow-md p-8 flex flex-col space-y-6 rounded-none">
    <!-- Header -->
    <div class="flex items-center gap-3 text-slate-800 border-b border-slate-200 pb-4">
      <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-6 h-6 shrink-0 text-slate-700">
        <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125v-3.75m16.5 0v3.75m-16.5-3.75v3.75" />
      </svg>
      <h2 data-i18n-key="db-setup-title" class="text-md font-bold uppercase tracking-wider text-slate-900">
        {t("db-setup-title", "Database Configuration")}
      </h2>
    </div>

    <!-- Diagnostic Details & Copy -->
    <div class="space-y-4">
      {#if authStore.dbSetupState.type === "unconfigured"}
        <p data-i18n-key="db-setup-unconfigured-desc" class="text-xs text-slate-600 leading-relaxed">
          {t("db-setup-unconfigured-desc", "Welcome to the Herbarium Specimen Duplicate Finder. To start capturing records, you must configure the location of your reference database file. This database contains taxonomic catalog datasets (GBIF, WCVP) required for autocomplete and verification.")}
        </p>
      {:else if authStore.dbSetupState.type === "missing"}
        <div class="bg-amber-50 border border-amber-200 p-4 flex flex-col space-y-2">
          <div class="flex items-center gap-2 text-amber-800 text-xs font-bold uppercase tracking-wider">
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" class="w-4 h-4">
              <path fill-rule="evenodd" d="M9.401 3.003c1.155-2 4.043-2 5.197 0l7.355 12.748c1.154 2-.29 4.5-2.599 4.5H4.645c-2.309 0-3.752-2.5-2.598-4.5L9.4 3.003zM12 8.25a.75.75 0 01.75.75v3.75a.75.75 0 01-1.5 0V9a.75.75 0 01.75-.75zm0 8.25a.75.75 0 100-1.5.75.75 0 000 1.5z" clip-rule="evenodd" />
            </svg>
            <span>{t("db-status-missing", "Database File Missing")}</span>
          </div>
          <p data-i18n-key="db-setup-missing-desc" class="text-xs text-amber-900 leading-relaxed">
            {t("db-setup-missing-desc", "The database file could not be found at the previously configured location. It may have been moved, renamed, or deleted.")}
          </p>
          <div class="bg-white/60 border border-amber-200/50 p-2 text-[10px] font-mono break-all text-amber-955">
            <strong>{t("db-configured-path", "Configured path:")}</strong> {authStore.dbSetupState.path}
          </div>
        </div>
      {:else if authStore.dbSetupState.type === "invalid"}
        <div class="bg-red-50 border border-red-200 p-4 flex flex-col space-y-2">
          <div class="flex items-center gap-2 text-red-800 text-xs font-bold uppercase tracking-wider">
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" class="w-4 h-4">
              <path fill-rule="evenodd" d="M12 2.25c-5.385 0-9.75 4.365-9.75 9.75s4.365 9.75 9.75 9.75 9.75-4.365 9.75-9.75S17.385 2.25 12 2.25zm-1.72 6.97a.75.75 0 10-1.06 1.06L10.94 12l-1.72 1.72a.75.75 0 101.06 1.06L12 13.06l1.72 1.72a.75.75 0 101.06-1.06L13.06 12l1.72-1.72a.75.75 0 10-1.06-1.06L12 10.94l-1.72-1.72z" clip-rule="evenodd" />
            </svg>
            <span>{t("db-status-invalid", "Invalid Database Structure")}</span>
          </div>
          <p data-i18n-key="db-setup-invalid-desc" class="text-xs text-red-900 leading-relaxed">
            {t("db-setup-invalid-desc", "The selected file is not a valid database or is missing required tables (gbif, wcvp, captured_records). Please select a valid reference database file.")}
          </p>
          {#if authStore.dbSetupState.error}
            <div class="bg-white/60 border border-red-200/50 p-2 text-[10px] font-mono break-all text-red-955">
              <strong>{t("db-validation-error", "Validation error:")}</strong> {authStore.dbSetupState.error}
            </div>
          {/if}
        </div>
      {/if}

      {#if errorMsg}
        <div class="bg-rose-50 border border-rose-200 p-3 text-xs text-rose-800 font-mono break-all">
          <strong>{t("error-prefix", "Error:")}</strong> {errorMsg}
        </div>
      {/if}

      <div class="text-[11px] text-slate-550 leading-relaxed bg-slate-100 p-3 border border-slate-200/55">
        <span class="font-bold text-slate-700 block mb-1">{t("db-requirements", "Database Requirements:")}</span>
        <ul class="list-disc list-inside space-y-1">
          <li>{t("db-req-sqlite", "Must be a SQLite database file.")}</li>
          <li>{t("db-req-tables", "Must contain tables: gbif, wcvp, captured_records.")}</li>
          <li>{t("db-req-size", "Reference datasets can exceed 500MB (downloaded separately).")}</li>
        </ul>
      </div>
    </div>

    <!-- Actions -->
    <div class="flex pt-2 justify-end">
      <button
        type="button"
        disabled={isProcessing}
        onclick={handleSelectDatabase}
        class="bg-slate-800 hover:bg-slate-900 text-white disabled:bg-slate-300 disabled:cursor-not-allowed px-6 py-3 text-xs font-bold uppercase tracking-wider rounded-none transition-colors duration-150 cursor-pointer flex items-center gap-2"
      >
        {#if isProcessing}
          <div class="w-3.5 h-3.5 border-2 border-slate-400 border-t-white rounded-full animate-spin"></div>
          <span>{t("db-setup-processing", "Verifying...")}</span>
        {:else}
          <svg xmlns="http://www.w3.org/2005/svg" fill="none" viewBox="0 0 24 24" stroke-width="2.5" stroke="currentColor" class="w-4 h-4">
            <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
          </svg>
          <span>{t("select-db-btn", "Select Database File")}</span>
        {/if}
      </button>
    </div>
  </div>
</div>
