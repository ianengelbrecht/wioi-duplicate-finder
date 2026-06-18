<script>
  import { getContext } from "svelte";
  import { authStore } from "$lib/stores/authStore.svelte.js";

  const t = getContext("t");

  /**
   * @type {{
   *   onRestoreRequest?: (event: MouseEvent) => void | Promise<void>;
   *   onRetry?: (event: MouseEvent) => void | Promise<void>;
   * }}
   */
  let {
    onRestoreRequest = () => {},
    onRetry = () => {}
  } = $props();
</script>

<div class="flex-1 flex flex-col justify-center items-center p-6 bg-slate-50">
  <div class="w-full max-w-md bg-white border border-red-200 shadow-lg p-8 flex flex-col space-y-6">
    <div class="flex items-center gap-3 text-red-700 border-b border-red-100 pb-3">
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" class="w-6 h-6 shrink-0">
        <path fill-rule="evenodd" d="M9.401 3.003c1.155-2 4.043-2 5.197 0l7.355 12.748c1.154 2-.29 4.5-2.599 4.5H4.645c-2.309 0-3.752-2.5-2.598-4.5L9.4 3.003zM12 8.25a.75.75 0 01.75.75v3.75a.75.75 0 01-1.5 0V9a.75.75 0 01.75-.75zm0 8.25a.75.75 0 100-1.5.75.75 0 000 1.5z" clip-rule="evenodd" />
      </svg>
      <h2 data-i18n-key="db-recovery-title" class="text-base font-bold uppercase tracking-wider">
        {t("db-recovery-title", "Database Recovery Needed")}
      </h2>
    </div>
    
    <div class="space-y-3">
      <p data-i18n-key="db-recovery-desc" class="text-xs text-slate-600 leading-relaxed">
        {t("db-recovery-desc", "The application failed to open or verify the integrity of the specimen database. This usually means the database is corrupted or missing. Please select a backup file to restore or retry the connection.")}
      </p>
      
      {#if authStore.dbErrorMessage}
        <div class="bg-red-50 border border-red-100 p-3 text-xs text-red-800 font-mono break-all whitespace-pre-wrap max-h-40 overflow-y-auto">
          <strong>Error details:</strong><br/>{authStore.dbErrorMessage}
        </div>
      {/if}
    </div>
    
    <div class="flex flex-col sm:flex-row gap-3 pt-2">
      <button
        type="button"
        onclick={onRestoreRequest}
        class="flex-1 bg-rose-700 hover:bg-rose-800 text-white px-4 py-2.5 text-xs font-bold uppercase tracking-wider rounded-none transition-colors text-center cursor-pointer"
      >
        {t("restore-backup-btn", "Restore Backup")}
      </button>
      <button
        type="button"
        onclick={onRetry}
        class="flex-1 bg-slate-800 hover:bg-slate-900 text-white px-4 py-2.5 text-xs font-bold uppercase tracking-wider rounded-none transition-colors text-center cursor-pointer"
      >
        {t("retry-connection-btn", "Retry Connection")}
      </button>
    </div>
  </div>
</div>
