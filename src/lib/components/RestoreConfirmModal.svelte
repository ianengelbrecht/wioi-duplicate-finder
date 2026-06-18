<script>
  import { getContext } from "svelte";
  import { workspaceStore } from "$lib/stores/workspaceStore.svelte.js";
  import { backupService } from "$lib/services/backupService.js";

  const t = getContext("t");

  let { onRestoreSuccess = async () => {} } = $props();

  async function executeRestore() {
    workspaceStore.showRestoreConfirmModal = false;
    if (!workspaceStore.pendingRestorePath) return;
    
    try {
      await backupService.restoreDatabaseFromBackup(workspaceStore.pendingRestorePath);
      alert(t("db-restore-success", "Database restored successfully! Re-initializing..."));
      workspaceStore.pendingRestorePath = "";
      
      await onRestoreSuccess();
    } catch (e) {
      alert("Restore failed: " + (/** @type {any} */ (e)).toString());
    }
  }
</script>

{#if workspaceStore.showRestoreConfirmModal}
  <div 
    class="fixed inset-0 z-[120] flex items-center justify-center bg-slate-900/40 backdrop-blur-xs p-4"
    role="dialog"
    aria-modal="true"
    tabindex="-1"
    onclick={(e) => { if (e.target === e.currentTarget) workspaceStore.showRestoreConfirmModal = false; }}
    onkeydown={(e) => { 
      if (e.key === "Escape") {
        e.preventDefault();
        workspaceStore.showRestoreConfirmModal = false; 
      } else if (e.key === "Enter") {
        e.preventDefault();
        executeRestore();
      }
    }}
  >
    <div class="bg-white border border-slate-200 shadow-2xl max-w-sm w-full p-5 flex flex-col gap-4 rounded-none">
      <div class="flex items-start gap-3">
        <div class="p-2 bg-rose-50 text-rose-650 rounded-full shrink-0">
          <svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256" class="w-5 h-5 text-rose-700">
            <path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm0,192a88,88,0,1,1,88-88A88.09,88.09,0,0,1,128,216Zm-8-80V80a8,8,0,0,1,16,0v56a8,8,0,0,1-16,0Zm20,36a12,12,0,1,1-12-12A12,12,0,0,1,140,172Z"></path>
          </svg>
        </div>
        <div class="space-y-2">
          <h3 data-i18n-key="restore-confirm-title" class="font-bold text-rose-700">{t("restore-confirm-title", "Restore Database Backup")}</h3>
          <p data-i18n-key="restore-confirm-desc" class="text-sm text-slate-500 leading-relaxed">
            {t("restore-confirm-desc", "Are you sure you want to restore this database backup? This will completely overwrite your current database and cannot be undone.")}
          </p>
          {#if workspaceStore.pendingRestorePath}
            <p class="text-[10px] font-semibold text-slate-700 bg-slate-50 p-2 border border-slate-150 break-all">
              {workspaceStore.pendingRestorePath}
            </p>
          {/if}
        </div>
      </div>
      
      <div class="flex justify-end gap-2 mt-2">
        <button
          type="button"
          data-i18n-key="cancel-btn"
          onclick={() => { workspaceStore.showRestoreConfirmModal = false; }}
          class="px-3 py-1.5 text-xs font-semibold text-slate-500 hover:bg-slate-50 border border-slate-200 transition-colors cursor-pointer rounded-none"
        >
          {t("cancel-btn", "Cancel")}
        </button>
        <button
          type="button"
          data-i18n-key="restore-btn"
          onclick={executeRestore}
          class="px-3.5 py-1.5 text-xs font-semibold text-white bg-rose-700 hover:bg-rose-800 transition-colors cursor-pointer rounded-none"
        >
          {t("restore-btn", "Restore")}
        </button>
      </div>
    </div>
  </div>
{/if}
