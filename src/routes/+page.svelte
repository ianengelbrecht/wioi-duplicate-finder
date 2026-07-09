<script>
  import { listen } from "@tauri-apps/api/event";
  import { setContext } from "svelte";
  import SearchPane from "$lib/components/SearchPane.svelte";
  import CaptureForm from "$lib/components/CaptureForm.svelte";
  
  // Components
  import Auth from "$lib/components/Auth.svelte";
  import DbRecovery from "$lib/components/DbRecovery.svelte";
  import DbSetup from "$lib/components/DbSetup.svelte";
  import RestoreConfirmModal from "$lib/components/RestoreConfirmModal.svelte";
  import SessionsTab from "$lib/components/SessionsTab.svelte";
  import SettingsTab from "$lib/components/SettingsTab.svelte";
  import ProfileTab from "$lib/components/ProfileTab.svelte";
  import UsersTab from "$lib/components/UsersTab.svelte";
  import CapturedRecordsTable from "$lib/components/CapturedRecordsTable.svelte";
  import QuestionIcon from "$lib/components/icons/QuestionIcon.svelte";

  // Stores and Services
  import { authStore } from "$lib/stores/authStore.svelte.js";
  import { workspaceStore } from "$lib/stores/workspaceStore.svelte.js";
  import { updaterStore } from "$lib/stores/updaterStore.svelte.js";
  import { authService } from "$lib/services/authService.js";
  import { databaseService } from "$lib/services/databaseService.js";
  import { sessionService } from "$lib/services/sessionService.js";
  import { specimenService } from "$lib/services/specimenService.js";
  import { taxonomyService } from "$lib/services/taxonomyService.js";
  import { referenceService } from "$lib/services/referenceService.js";
  import { backupService } from "$lib/services/backupService.js";
  import { exportService } from "$lib/services/exportService.js";
  import { COUNTRY_DATA } from "$lib/utils/countryData.js";
  
  // Use-case logic
  import { generateCSVContent } from "$lib/usecases/generateCSVContent.js";

  // Dashboard tab state (UI only)
  let activeTab = $state("sessions"); // "sessions", "settings"
  let searchPaneRef = $state(/** @type {any} */ (null));
  
  // Localization state (UI only)
  let currentLanguage = $state(localStorage.getItem("currentLanguage") || "EN");
  let translations = $state(/** @type {Record<string, string>} */ ({}));

  const allLanguages = ["EN", "FR", "PT", "MG"];
  let allowedLanguages = $derived.by(() => {
    const country = workspaceStore.homeCountry;
    if (country && COUNTRY_DATA[country]) {
      return COUNTRY_DATA[country].languages;
    }
    return allLanguages;
  });

  function t(/** @type {string} */ key, /** @type {string} */ defaultText) {
    return translations[key] || defaultText;
  }

  setContext("t", t);

  $effect(() => {
    localStorage.setItem("currentLanguage", currentLanguage);
    fetch(`/lang/${currentLanguage.toLowerCase()}.json`)
      .then(res => res.json())
      .then(data => {
        translations = data;
      })
      .catch(err => {
        console.error("Failed to load translations:", err);
      });
  });

  $effect(() => {
    // If the currently selected language is not allowed under the selected country, switch to the first allowed language
    if (!allowedLanguages.includes(currentLanguage) && allowedLanguages.length > 0) {
      currentLanguage = allowedLanguages[0];
    }
  });

  /**
   * Helper function to check and update reference dataset counts.
   * Forces the tab to "settings" if required datasets are missing.
   * @returns {Promise<void>}
   */
  async function checkDatasetCounts() {
    try {
      const gbifData = await referenceService.getReferenceMetadata();
      workspaceStore.gbifRecordCount = gbifData.recordCount;
      
      const wcvpData = await referenceService.getWcvpMetadata();
      workspaceStore.wcvpRecordCount = wcvpData.recordCount;

      if (!workspaceStore.hasRequiredDatasets) {
        activeTab = "settings";
      }
    } catch (err) {
      console.error("Failed to load dataset counts:", err);
    }
  }

  async function checkDb() {
    authStore.dbErrorMessage = "";
    try {
      await authService.initializeDatabase();
      
      const storedUser = localStorage.getItem("currentUser");
      if (storedUser) {
        authStore.setCurrentUser(JSON.parse(storedUser));
        await loadSessions();
        await loadExportSettings();
        await checkDatasetCounts();
        
        const storedSession = localStorage.getItem("lastActiveSession");
        if (storedSession) {
          const parsedSession = JSON.parse(storedSession);
          const sessionExists = workspaceStore.sessionList.find(s => s.id === parsedSession.id);
          if (sessionExists && workspaceStore.hasRequiredDatasets) {
            await selectSession(sessionExists);
            return;
          }
        }
        authStore.setView("dashboard");
      } else {
        authStore.setView("auth");
      }
    } catch (e) {
      console.error(e);
      const errStr = (/** @type {any} */ (e)).toString();
      authStore.dbErrorMessage = errStr;
      
      if (errStr.includes("DATABASE_UNCONFIGURED")) {
        authStore.dbSetupState = { type: "unconfigured", path: "", error: "" };
        authStore.setView("db_setup");
      } else if (errStr.includes("DATABASE_MISSING:")) {
        const parts = errStr.split("DATABASE_MISSING:");
        const path = parts[parts.length - 1];
        authStore.dbSetupState = { type: "missing", path, error: "" };
        authStore.setView("db_setup");
      } else if (errStr.includes("DATABASE_INVALID:")) {
        const parts = errStr.split("DATABASE_INVALID:");
        const error = parts[parts.length - 1];
        authStore.dbSetupState = { type: "invalid", path: "", error };
        authStore.setView("db_setup");
      } else {
        authStore.setView("db_restore");
      }
    }
  }

  $effect(() => {
    const unlistenPromise = listen("db-init-progress", (event) => {
      authStore.dbLoadingMessage = event.payload;
    });

    checkDb().then(() => {
      if (!updaterStore.hasChecked && updaterStore.status === "idle") {
        updaterStore.check();
      }
    });

    return () => {
      unlistenPromise.then((unlistenFn) => unlistenFn());
    };
  });

  async function loadSessions() {
    if (!authStore.currentUser) return;
    try {
      workspaceStore.sessionList = await sessionService.getSessions(authStore.currentUser.id);
    } catch (e) {
      console.error(e);
    }
  }

  async function selectSession(/** @type {any} */ session) {
    workspaceStore.selectSession(session);
    authStore.setView("workspace");
    workspaceStore.exportMessage = "";
    workspaceStore.exportError = "";
    
    await loadCapturedRecords();
  }

  async function loadCapturedRecords() {
    if (!workspaceStore.activeSession) return;
    try {
      workspaceStore.capturedRecords = await specimenService.getCapturedRecords(workspaceStore.activeSession.id);
      workspaceStore.activeSession.recordCount = workspaceStore.capturedRecords.length;
    } catch (e) {
      console.error(e);
    }
  }

  async function loadExportSettings() {
    if (!authStore.currentUser) return;
    try {
      try {
        const defaultDir = await backupService.getDefaultBackupDir();
        workspaceStore.defaultBackupLocation = /** @type {string} */ (defaultDir);
      } catch (err) {
        console.error("Failed to query default backup directory:", err);
      }

      let settings = /** @type {any} */ (await exportService.getExportSettings(authStore.currentUser.id));
      workspaceStore.exportFormat = settings.format || "DwC";
      workspaceStore.workingCollectionCode = settings.collectionCode || "RHOIO";
      workspaceStore.homeCountry = settings.homeCountry || "";
      workspaceStore.includeGridReference = settings.includeGridReference || false;
      workspaceStore.includeIslands = settings.includeIslands || false;
      workspaceStore.databaseBackupLocation = settings.backupLocation || workspaceStore.defaultBackupLocation;
    } catch (e) {
      console.error(e);
    }
  }

  async function handleRestoreBackup() {
    try {
      const path = await backupService.selectBackupFile();
      if (path) {
        workspaceStore.pendingRestorePath = /** @type {string} */ (path);
        workspaceStore.showRestoreConfirmModal = true;
      }
    } catch (err) {
      console.error("Failed to choose backup file:", err);
    }
  }

  async function returnToDashboard() {
    authStore.setView("dashboard");
    workspaceStore.clearWorkspace();
    await loadSessions();
  }

  function handleSelectSearchResult(/** @type {any} */ rec) {
    workspaceStore.activeRecord = { ...rec };
  }

  async function handleExportCSV() {
    if (!workspaceStore.activeSession) return;
    workspaceStore.exportMessage = "";
    workspaceStore.exportError = "";
    
    try {
      const records = /** @type {any[]} */ (await specimenService.getCapturedRecords(workspaceStore.activeSession.id));
      if (!records || records.length === 0) {
        workspaceStore.exportError = "No records to export in this session.";
        return;
      }

      const date = new Date();
      date.setMinutes(date.getMinutes() - date.getTimezoneOffset());

      const local = date
        .toISOString()
        .slice(0, 19)
        .replace(/[:T-]/g, "");

      let defaultName = `${workspaceStore.activeSession.name.replace(/[^a-zA-Z0-9]/g, "_")}_${local}.csv`;
      let path = await exportService.selectExportPath(defaultName);
      if (!path) {
        return;
      }

      const queries = records.map(rec => ({
        id: rec.id,
        taxonID: rec.taxonID,
        scientificName: rec.scientificName
      }));
      const familyMap = await taxonomyService.resolveWcvpFamilies(queries);
      
      const csvContent = generateCSVContent(records, workspaceStore.exportFormat, familyMap, workspaceStore.includeGridReference);
      
      let res = await exportService.exportSessionCsv(
        workspaceStore.activeSession.id,
        path,
        csvContent
      );
      workspaceStore.exportMessage = /** @type {string} */ (res);
    } catch (err) {
      workspaceStore.exportError = (/** @type {any} */ (err)).toString();
    }
  }

  function handleLogout() {
    authStore.setCurrentUser(null);
    workspaceStore.clearWorkspace();
    authStore.setView("auth");
    authStore.authError = "";
    authStore.authSuccess = "";
  }

  async function handleUpdateClick() {
    if (updaterStore.status === "downloaded") {
      await updaterStore.install();
    } else {
      await updaterStore.download();
    }
  }
</script>

<div class="{authStore.view === "dashboard" ? "h-screen" : "min-h-screen"} bg-slate-50 text-slate-800 flex flex-col font-sans">
  <!-- Top Navigation Header -->
  <header class="bg-white border-b border-slate-300 px-6 py-4 flex justify-between items-center z-10">
    <div class="flex items-center gap-3">
      <div class="bg-slate-800 text-white p-1.5 font-extrabold text-sm uppercase tracking-widest">
        RHOIO
      </div>
      <div>
        <h1 data-i18n-key="app-title" class="text-md font-bold tracking-tight text-slate-800">{t("app-title", "Herbarium Specimen Duplicate Finder")}</h1>
        <p data-i18n-key="app-subtitle" class="text-[10px] text-slate-500 font-semibold uppercase tracking-wider">{t("app-subtitle", "Offline Data Entry Accelerator")}</p>
      </div>
    </div>

    <!-- Active User & Session Control -->
    <div class="flex items-center gap-8 text-xs font-semibold">
      <!-- Language Selector and Github -->
      <div class="flex items-center gap-2">
        {#if updaterStore.hasChecked || updaterStore.status === 'checking'}
          {#if updaterStore.status === 'checking'}
            <div
              class="px-3 py-1.5 text-[10px] font-bold tracking-wider rounded-none border border-dashed border-slate-300 text-slate-400 bg-transparent select-none flex items-center gap-1.5"
            >
              <span class="w-2 h-2 border border-slate-400 border-t-transparent rounded-full animate-spin"></span>
              {t("update-status-checking", "Checking for updates...")}
            </div>
          {:else if updaterStore.isAvailable}
            <button
              type="button"
              onclick={handleUpdateClick}
              disabled={updaterStore.status === 'downloading' || updaterStore.status === 'installing'}
              class="px-3 py-1.5 text-[10px] font-bold tracking-wider rounded-none bg-emerald-600 hover:bg-emerald-700 text-white transition-colors cursor-pointer flex items-center gap-1.5 disabled:opacity-75 disabled:cursor-not-allowed"
            >
              {#if updaterStore.status === 'downloading'}
                <span class="w-2 h-2 border border-white border-t-transparent rounded-full animate-spin"></span>
                {t("update-btn-downloading", "Downloading")}{updaterStore.downloadProgress !== null ? ` ${updaterStore.downloadProgress}%` : '...'}
              {:else if updaterStore.status === 'installing'}
                <span class="w-2 h-2 border border-white border-t-transparent rounded-full animate-spin"></span>
                {t("update-btn-installing", "Installing")}{updaterStore.downloadProgress !== null ? ` ${updaterStore.downloadProgress}%` : '...'}
              {:else if updaterStore.status === 'downloaded'}
                {t("update-btn-install", "Install Update")}
              {:else}
                {t("update-btn-download", "Update")}
              {/if}
            </button>
          {:else}
            <button
              type="button"
              onclick={() => updaterStore.check()}
              title={t("update-status-check-now", "Click to check for updates")}
              class="px-3 py-1.5 text-[10px] font-bold tracking-wider rounded-none border border-slate-300 text-slate-500 hover:text-slate-700 hover:bg-slate-50 hover:border-slate-400 transition-all cursor-pointer"
            >
              V{updaterStore.currentVersion}: {t("update-status-uptodate", "up to date")}
            </button>
          {/if}
        {/if}

        <div class="flex items-center border border-slate-300 divide-x divide-slate-300 select-none">
          {#each allowedLanguages as lang}
            <button
              type="button"
              onclick={() => currentLanguage = lang}
              class="px-2.5 py-1.5 text-[10px] font-bold tracking-wider hover:bg-slate-50 transition-colors cursor-pointer {currentLanguage === lang ? 'bg-slate-800 text-white hover:bg-slate-800' : 'bg-white text-slate-650'}"
            >
              {lang}
            </button>
          {/each}
        </div>
        <!-- Github button -->
        <a
          href="https://github.com/ianengelbrecht/wioi-duplicate-finder"
          target="_blank"
          rel="noopener noreferrer"
          class="shrink-0"
          title="View on GitHub"
        >
          <img src="github.png" alt="GitHub" class="w-5 h-5 inline-block mr-1" />
        </a>
        <!-- Documentation -->
        <a
          href="https://ianengelbrecht.github.io/wioi-duplicate-finder-docs/"
          target="_blank"
          rel="noopener noreferrer"
          title="Documentation"
          class="w-6 h-6 shrink-0 text-slate-600"
        >
          <QuestionIcon  />
        </a>
      </div>
      
      {#if authStore.currentUser}
        <div class="text-right">
          <span data-i18n-key="logged-in-as" class="text-slate-400 block text-[9px] uppercase tracking-wider">{t("logged-in-as", "Logged In As")}</span>
          <span class="text-slate-800 font-bold">{authStore.currentUser.username}</span>
        </div>
        <button
          data-i18n-key="sign-out-btn"
          onclick={handleLogout}
          class="border border-slate-350 hover:bg-slate-100 px-3 py-1.5 text-[10px] uppercase font-bold tracking-wide rounded-none transition-colors"
        >
          {t("sign-out-btn", "Sign Out")}
        </button>
      {/if}
    </div>
  </header>

  <!-- Content Router View -->
  <main class="flex-1 flex flex-col min-h-0">
    <!-- VIEW 0: DATABASE INITIALIZATION LOADING SCREEN -->
    {#if authStore.view === "loading"}
      <div class="flex-1 flex flex-col justify-center items-center p-6 bg-slate-50">
        <div class="w-full max-w-sm bg-white border border-slate-300 shadow-sm p-8 flex flex-col items-center text-center space-y-4">
          <!-- Spinner -->
          <div class="w-8 h-8 border-4 border-slate-200 border-t-slate-800 rounded-full animate-spin"></div>
          <div>
            <h2 data-i18n-key="preparing-db" class="text-sm font-bold text-slate-800 uppercase tracking-wider">{t("preparing-db", "Preparing Database")}</h2>
            <p class="text-xs text-slate-550 mt-2 leading-relaxed">
              {authStore.dbLoadingMessage}
            </p>
          </div>
        </div>
      </div>
    {:else if authStore.view === "db_restore"}
      <DbRecovery onRestoreRequest={handleRestoreBackup} onRetry={checkDb} />
    {:else if authStore.view === "db_setup"}
      <DbSetup onRetry={checkDb} />
    {:else if authStore.view === "auth"}
      <Auth onLoginSuccess={async () => { await loadSessions(); await loadExportSettings(); await checkDatasetCounts(); }} />

    <!-- VIEW 2: SESSION MANAGEMENT & EXPORT SETTINGS DASHBOARD -->
    {:else if authStore.view === "dashboard"}
      <div class="relative flex-1 max-w-5xl mx-auto w-full p-6 grid grid-cols-1 md:grid-cols-3 gap-6 min-h-0">
        <!-- Sidebar Navigation Tabs -->
        <div class="md:col-span-1 flex flex-col justify-between ">
          <div class="flex flex-col gap-2">
            <button
              onclick={() => activeTab = "settings"}
              class="w-full text-left px-4 py-3 text-xs font-bold uppercase tracking-wider border rounded-none transition-all {activeTab === 'settings' ? 'bg-slate-800 text-white border-slate-800' : 'bg-white text-slate-700 border-slate-300 hover:bg-slate-50'}"
            >
              {t("application-settings", "Application Settings")}
            </button>
            <button
              onclick={() => {
                if (workspaceStore.hasRequiredDatasets) {
                  activeTab = "sessions";
                }
              }}
              disabled={!workspaceStore.hasRequiredDatasets}
              class="w-full text-left px-4 py-3 text-xs font-bold uppercase tracking-wider border rounded-none transition-all {activeTab === 'sessions' ? 'bg-slate-800 text-white border-slate-800' : 'bg-white text-slate-700 border-slate-300 hover:bg-slate-50'} disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-between"
              title={!workspaceStore.hasRequiredDatasets ? "Please import GBIF and WCVP datasets in settings first" : ""}
            >
              <span>{t("capture-sessions-heading", "Capture Sessions")}</span>
              {#if !workspaceStore.hasRequiredDatasets}
                <span class="text-[9px] bg-amber-100 text-amber-800 px-1.5 py-0.5 border border-amber-305 uppercase tracking-wide font-bold">Required</span>
              {/if}
            </button>
            <button
              onclick={() => activeTab = "profile"}
              class="w-full text-left px-4 py-3 text-xs font-bold uppercase tracking-wider border rounded-none transition-all {activeTab === 'profile' ? 'bg-slate-800 text-white border-slate-800' : 'bg-white text-slate-700 border-slate-300 hover:bg-slate-50'}"
            >
              {t("profile-settings-heading", "My Profile")}
            </button>
            {#if authStore.currentUser?.isAdmin}
              <button
                onclick={() => activeTab = "users"}
                class="w-full text-left px-4 py-3 text-xs font-bold uppercase tracking-wider border rounded-none transition-all {activeTab === 'users' ? 'bg-slate-800 text-white border-slate-800' : 'bg-white text-slate-700 border-slate-300 hover:bg-slate-50'}"
              >
                {t("manage-users-heading", "Manage Users")}
              </button>
            {/if}
          </div>
          <!-- Funders logos -->
          <div class="">
            <p class="text-xs font-bold uppercase text-right text-slate-600">{t("originally-developed", "Originally developed for the")} Réseau d'herbiers de l'océan Indien occidental {t("wioi-network", "")}</p>
            <a href="https://varuna-biodiversite.org" target="_blank" rel="noopener noreferrer">
              <img src="funders_desat.png" alt="Funders Logos" class="w-full" />
            </a>
            
          </div>
        </div>

        <!-- Main Dashboard Tab Panels -->
        <div class="md:col-span-2 bg-white border border-slate-300 p-6 flex flex-col min-h-0 overflow-y-auto">
          {#if activeTab === "sessions"}
            <SessionsTab currentLanguage={currentLanguage} onSelectSession={selectSession} />
          {:else if activeTab === "settings"}
            <SettingsTab onRestoreRequest={handleRestoreBackup} />
          {:else if activeTab === "profile"}
            <ProfileTab />
          {:else if activeTab === "users" && authStore.currentUser?.isAdmin}
            <UsersTab />
          {/if}
        </div>
        <p class="absolute bottom-6 -right-40 text-xs text-slate-600">Built with 
          <a href="https://svelte.dev" target="_blank" rel="noopener noreferrer" title="Svelte" class="underline hover:text-slate-800 transition-colors">
            <img src="svelte.svg" alt="Svelte Logo" class="inline-block w-4 h-4 ml-1" /> 
          </a> |
          <a href="https://tailwindcss.com" target="_blank" rel="noopener noreferrer" title="Tailwind CSS" class="underline hover:text-slate-800 transition-colors">
            <img src="tailwindcss.svg" alt="Tailwind CSS Logo" class="inline-block w-4 h-4 ml-1" />
          </a> |
          <a href="https://www.sqlite.org" target="_blank" rel="noopener noreferrer" title="SQLite" class="underline hover:text-slate-800 transition-colors">
            <img src="sqlite.svg" alt="SQLite Logo" class="inline-block w-4 h-4 ml-1" />
          </a> |
          <a href="https://tauri.app" target="_blank" rel="noopener noreferrer" title="Tauri" class="underline hover:text-slate-800 transition-colors">
            <img src="tauri.svg" alt="Tauri Logo" class="inline-block w-4 h-4 ml-1" />
          </a>
        </p>
      </div>

    <!-- VIEW 3: MAIN DUAL-PANE WORKSPACE -->
    {:else if authStore.view === "workspace"}
      <div class="flex-1 flex flex-col min-h-0 bg-slate-100">
        <!-- Workspace Header Sub-Bar -->
        <div class="px-6 py-3 bg-slate-800 text-white flex justify-between items-center shrink-0">
          <div class="flex items-center gap-3">
            <button
              onclick={returnToDashboard}
              title = "Return to Sessions"
              class="bg-slate-700 hover:bg-slate-600 text-xs font-bold uppercase px-3 py-1.5 tracking-wide rounded-none transition-colors"
            >
              <span>
                <svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" fill="currentColor" viewBox="0 0 256 256"><path d="M224,128a8,8,0,0,1-8,8H59.31l58.35,58.34a8,8,0,0,1-11.32,11.32l-72-72a8,8,0,0,1,0-11.32l72-72a8,8,0,0,1,11.32,11.32L59.31,120H216A8,8,0,0,1,224,128Z"></path></svg>
              </span> 
            </button>
            <div>
              <span data-i18n-key="active-session" class="text-[9px] uppercase tracking-wider text-slate-400 block font-semibold">{t("active-session", "Active Session")}</span>
              <span class="text-xs font-bold text-slate-100">{workspaceStore.activeSession.name}</span>
            </div>
          </div>

          <div class="flex items-center gap-4">
            <!-- Active stats count -->
            <div class="text-right">
              <span data-i18n-key="specimens-captured-heading" class="text-[9px] uppercase tracking-wider text-slate-400 block font-semibold">{t("specimens-captured-heading", "Specimens Captured")}</span>
              <span class="text-xs font-extrabold text-emerald-400">{workspaceStore.activeSession.recordCount} {t("records-count", "records")}</span>
            </div>
            
            <!-- Export Session CSV bar -->
            <div class="flex items-center">
              <button
                onclick={handleExportCSV}
                data-i18n-key="export-csv-btn"
                class="bg-emerald-600 hover:bg-emerald-700 text-white px-5 py-2 text-xs font-bold uppercase tracking-wider rounded-none transition-colors"
              >
                {t("export-csv-btn", "Export CSV")}
              </button>
            </div>
          </div>
        </div>

        <!-- Export status notification -->
        {#if workspaceStore.exportMessage}
          <div class="bg-emerald-50 border-b border-emerald-300 text-emerald-800 text-xs px-6 py-2 flex justify-between items-center">
            <span>{workspaceStore.exportMessage}</span>
            <button onclick={() => { workspaceStore.exportMessage = ""; }} class="font-bold">✕</button>
          </div>
        {:else if workspaceStore.exportError}
          <div class="bg-red-50 border-b border-red-300 text-red-800 text-xs px-6 py-2 flex justify-between items-center">
            <span>⚠️ Export failed: {workspaceStore.exportError}</span>
            <button onclick={() => { workspaceStore.exportError = ""; }} class="font-bold">✕</button>
          </div>
        {/if}

        <!-- Workspace Dual-Pane Layout -->
        <div class="flex-1 grid grid-cols-1 lg:grid-cols-2 gap-4 p-4 min-h-0 overflow-y-auto">
          <!-- Left Pane (Search and Results Table) -->
          <div class="flex flex-col h-[650px] min-h-0">
            <SearchPane bind:this={searchPaneRef} onSelectRecord={handleSelectSearchResult} {currentLanguage} />
          </div>

          <!-- Right Pane (Specimen Capture Form) -->
          <div class="flex flex-col h-[650px] min-h-0">
            <CaptureForm 
              sessionId={workspaceStore.activeSession.id} 
              collectionCode={workspaceStore.workingCollectionCode}
              bind:activeRecord={workspaceStore.activeRecord} 
              onSaveSuccess={async () => {
                await loadCapturedRecords();
                await loadSessions();
              }} 
              {currentLanguage}
            />
          </div>
        </div>

        <!-- Bottom Panel: Captured Specimens in This Session -->
        <CapturedRecordsTable />
      </div>
    {/if}

    <!-- Shared confirmation dialog for db restore -->
    <RestoreConfirmModal onRestoreSuccess={checkDb} />
  </main>
</div>
