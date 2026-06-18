<script>
  import { getContext } from "svelte";
  import { sessionService } from "$lib/services/sessionService.js";
  import { authStore } from "$lib/stores/authStore.svelte.js";
  import { workspaceStore } from "$lib/stores/workspaceStore.svelte.js";

  const t = getContext("t");

  let { currentLanguage = "EN", onSelectSession = () => {} } = $props();

  let sessionName = $state("");
  let editingSessionId = $state(/** @type {number|null} */ (null));
  let editingName = $state("");

  // Custom delete session modal state (UI only)
  let showDeleteSessionModal = $state(false);
  let pendingDeleteSessionId = $state(/** @type {number|null} */ (null));
  let pendingDeleteSessionName = $state("");

  function isExportWarning(/** @type {any} */ ses) {
    if (!ses.lastRecordAt) return false;
    if (!ses.lastExportedAt) return ses.recordCount > 0;
    
    const recordTime = new Date(ses.lastRecordAt.replace(' ', 'T') + 'Z').getTime();
    const exportTime = new Date(ses.lastExportedAt.replace(' ', 'T') + 'Z').getTime();
    return exportTime < recordTime;
  }

  async function handleCreateSession(/** @type {any} */ e) {
    if (e) e.preventDefault();
    if (sessionName.trim().length === 0) return;

    try {
      let session = await sessionService.createSession(authStore.currentUser.id, sessionName);
      sessionName = "";
      // Refresh session list
      workspaceStore.sessionList = await sessionService.getSessions(authStore.currentUser.id);
      // Auto-enter workspace
      onSelectSession(session);
    } catch (err) {
      alert("Error creating session: " + (/** @type {any} */ (err)).toString());
    }
  }

  function startEditing(/** @type {any} */ ses, /** @type {MouseEvent} */ e) {
    if (e) e.stopPropagation();
    editingSessionId = ses.id;
    editingName = ses.name;
  }

  async function saveSessionName(/** @type {any} */ ses) {
    if (editingSessionId === null) return;
    
    let newName = editingName.trim();
    let oldName = ses.name;
    
    editingSessionId = null;
    
    if (newName.length === 0 || newName === oldName) {
      return;
    }
    
    try {
      await sessionService.renameSession(ses.id, newName);
      // Refresh session list
      workspaceStore.sessionList = await sessionService.getSessions(authStore.currentUser.id);
    } catch (err) {
      alert("Error renaming session: " + (/** @type {any} */ (err)).toString());
    }
  }

  function handleEditingKeydown(/** @type {KeyboardEvent} */ e, /** @type {any} */ ses) {
    if (e.key === "Enter") {
      saveSessionName(ses);
    } else if (e.key === "Escape") {
      editingSessionId = null;
    }
  }

  function promptDeleteSession(/** @type {number} */ id, /** @type {string} */ name, /** @type {any} */ e) {
    if (e) e.stopPropagation();
    pendingDeleteSessionId = id;
    pendingDeleteSessionName = name;
    showDeleteSessionModal = true;
  }

  async function confirmDeleteSession() {
    if (pendingDeleteSessionId === null) return;
    
    const id = pendingDeleteSessionId;
    pendingDeleteSessionId = null;
    showDeleteSessionModal = false;
    
    try {
      await sessionService.deleteSession(id);
      // Refresh session list
      workspaceStore.sessionList = await sessionService.getSessions(authStore.currentUser.id);
      
      const storedSession = localStorage.getItem("lastActiveSession");
      if (storedSession) {
        const parsed = JSON.parse(storedSession);
        if (parsed.id === id) {
          localStorage.removeItem("lastActiveSession");
        }
      }
      
      if (workspaceStore.activeSession && workspaceStore.activeSession.id === id) {
        workspaceStore.activeSession = null;
        authStore.setView("dashboard");
      }
    } catch (err) {
      alert("Error deleting session: " + (/** @type {any} */ (err)).toString());
    }
  }

  function cancelDeleteSession() {
    pendingDeleteSessionId = null;
    pendingDeleteSessionName = "";
    showDeleteSessionModal = false;
  }
</script>

<div class="space-y-6 flex-1 flex flex-col">
  <div>
    <h2 data-i18n-key="dashboard-title" class="text-md font-bold text-slate-900 uppercase tracking-wide">{t("dashboard-title", "Data Capture Sessions")}</h2>
    <p data-i18n-key="select-session-desc" class="text-xs text-slate-500 mt-1">{t("select-session-desc", "Select a session to start capturing or launch a new named session.")}</p>
  </div>

  <!-- Session Creator Form -->
  <form onsubmit={handleCreateSession} class="flex gap-2">
    <input
      type="text"
      placeholder={t("session-name-placeholder", "eg Malvaceae Cupboard 2")}
      bind:value={sessionName}
      class="flex-1 bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
    />
    <button
      type="submit"
      data-i18n-key="create-session-btn"
      class="bg-slate-800 hover:bg-slate-900 text-white px-6 py-2 text-xs font-bold uppercase tracking-wider rounded-none transition-colors"
    >
      {t("create-session-btn", "Create Session")}
    </button>
  </form>

  <!-- Session Listing -->
  <div class="flex-1 min-h-0">
    <h3 data-i18n-key="sessions-heading" class="text-[10px] font-bold text-slate-400 uppercase tracking-wider mb-2 border-b border-slate-100 pb-1">{t("sessions-heading", "Sessions")}</h3>
    
    {#if workspaceStore.sessionList.length > 0}
      <ul class="border border-slate-200 divide-y divide-slate-200">
        {#each workspaceStore.sessionList as ses}
          <li class="hover:bg-slate-50 transition-colors flex justify-between items-center pr-4">
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
              onclick={() => onSelectSession(ses)}
              class="flex-1 text-left p-4 flex justify-between items-center cursor-pointer outline-none"
            >
              <!-- svelte-ignore a11y_click_events_have_key_events -->
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <div class="flex-1 mr-4">
                {#if editingSessionId === ses.id}
                  <!-- svelte-ignore a11y_autofocus -->
                  <input
                    type="text"
                    bind:value={editingName}
                    onblur={() => saveSessionName(ses)}
                    onkeydown={(e) => handleEditingKeydown(e, ses)}
                    class="bg-white border border-slate-300 text-slate-800 text-sm px-2 py-1 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none w-full"
                    autofocus
                  />
                {:else}
                  <span 
                    onclick={(e) => startEditing(ses, e)}
                    class="text-sm font-semibold text-slate-900 block cursor-text hover:text-slate-600 hover:underline w-fit"
                    title="Click to rename"
                  >
                    {ses.name}
                  </span>
                {/if}
                <div class="flex flex-wrap items-center gap-3 mt-1 text-[10px] text-slate-500">
                  {#if ses.lastRecordAt}
                    <span>
                      {t("last-record", "last record")}:
                      <strong class="text-slate-700 font-semibold">
                        {new Date(ses.lastRecordAt.replace(' ', 'T') + 'Z').toLocaleString(currentLanguage === "EN" ? "en-US" : currentLanguage.toLowerCase(), {
                          year: 'numeric',
                          month: 'short',
                          day: 'numeric',
                          hour: '2-digit',
                          minute: '2-digit'
                        })}
                      </strong>
                    </span>
                  {:else}
                    <span data-i18n-key="no-records-captured" class="text-slate-400 italic">
                      {t("no-records-captured", "No records captured")}
                    </span>
                  {/if}

                  {#if ses.recordCount > 0}
                    {#if ses.lastExportedAt}
                      <span>
                        {t("last-export", "last export")}:
                        <span
                          class="px-1 py-0.5 rounded-none font-medium {isExportWarning(ses)
                            ? 'bg-red-50 text-red-700 border border-red-200'
                            : 'text-slate-700'}"
                        >
                          {new Date(ses.lastExportedAt.replace(' ', 'T') + 'Z').toLocaleString(currentLanguage === "EN" ? "en-US" : currentLanguage.toLowerCase(), {
                            year: 'numeric',
                            month: 'short',
                            day: 'numeric',
                            hour: '2-digit',
                            minute: '2-digit'
                          })}
                        </span>
                      </span>
                    {:else}
                      <span>
                        <span data-i18n-key="never-exported" class="px-1 py-0.5 rounded-none font-medium bg-red-50 text-red-700 border border-red-200">
                          {t("never-exported", "Never exported")}
                        </span>
                      </span>
                    {/if}
                  {/if}
                </div>
              </div>
              <div class="flex items-center gap-3">
                <span class="text-xs bg-slate-100 font-bold px-2 py-1 border border-slate-300">
                  {ses.recordCount} {t("specimens-count", "specimens")}
                </span>
              </div>
            </div>
            <button
              data-i18n-key="delete-btn"
              onclick={(e) => promptDeleteSession(ses.id, ses.name, e)}
              class="bg-red-55 bg-red-50 hover:bg-red-100 text-red-650 border border-red-200 px-3 py-1.5 text-xs font-bold uppercase tracking-wide transition-colors ml-2"
            >
              {t("delete-btn", "Delete")}
            </button>
          </li>
        {/each}
      </ul>
    {:else}
      <div class="h-48 flex flex-col justify-center items-center text-slate-400 border border-dashed border-slate-300 p-6 text-center">
        <span data-i18n-key="no-sessions-title" class="text-xs font-medium uppercase tracking-wider mb-1">{t("no-sessions-title", "No Sessions Available")}</span>
        <span data-i18n-key="no-sessions-desc" class="text-[11px] text-slate-400 max-w-xs">{t("no-sessions-desc", "Create a new named session above to get started with capturing specimens.")}</span>
      </div>
    {/if}
  </div>
</div>

{#if showDeleteSessionModal}
  <div 
    class="fixed inset-0 z-[120] flex items-center justify-center bg-slate-900/40 backdrop-blur-xs p-4"
    role="dialog"
    aria-modal="true"
    tabindex="-1"
    onclick={(e) => { if (e.target === e.currentTarget) cancelDeleteSession(); }}
    onkeydown={(e) => { 
      if (e.key === "Escape") {
        e.preventDefault();
        cancelDeleteSession(); 
      } else if (e.key === "Enter") {
        e.preventDefault();
        confirmDeleteSession();
      }
    }}
  >
    <div class="bg-white border border-slate-200 shadow-2xl max-w-sm w-full p-5 flex flex-col gap-4 rounded-none">
      <div class="flex items-start gap-3">
        <div class="p-2 bg-red-50 text-red-650 rounded-full shrink-0">
          <svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256" class="w-5 h-5">
            <path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm0,192a88,88,0,1,1,88-88A88.09,88.09,0,0,1,128,216Zm-8-80V80a8,8,0,0,1,16,0v56a8,8,0,0,1-16,0Zm20,36a12,12,0,1,1-12-12A12,12,0,0,1,140,172Z"></path>
          </svg>
        </div>
        <div class="space-y-2">
          <h3 data-i18n-key="delete-session-heading" class="font-bold text-red-700">{t("delete-session-heading", "Delete Capture Session")}</h3>
          <p data-i18n-key="delete-session-confirm" class="text-sm text-slate-500 leading-relaxed">
            {t("delete-session-confirm", "Are you sure you want to permanently delete this capture session?")}
          </p>
          <p class="text-xs font-semibold text-slate-700 bg-slate-50 p-2 border border-slate-150 break-all">
            {t("delete-session-label", "Session:")} {pendingDeleteSessionName}
          </p>
          <p data-i18n-key="delete-session-warning" class="text-xs text-red-600 font-medium leading-relaxed mt-1">
            {t("delete-session-warning", "WARNING: This will permanently delete all captured records associated with this session.")}
          </p>
        </div>
      </div>
      
      <div class="flex justify-end gap-2 mt-2">
        <button
          type="button"
          data-i18n-key="cancel-btn"
          onclick={cancelDeleteSession}
          class="px-3 py-1.5 text-xs font-semibold text-slate-500 hover:bg-slate-50 border border-slate-200 transition-colors cursor-pointer rounded-none"
        >
          {t("cancel-btn", "Cancel")}
        </button>
        <button
          type="button"
          data-i18n-key="delete-btn"
          onclick={confirmDeleteSession}
          class="px-3.5 py-1.5 text-xs font-semibold text-white bg-red-650 bg-red-400 hover:bg-red-700 transition-colors cursor-pointer rounded-none"
        >
          {t("delete-btn", "Delete")}
        </button>
      </div>
    </div>
  </div>
{/if}
