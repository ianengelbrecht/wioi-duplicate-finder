<script>
  import { invoke } from "@tauri-apps/api/core";
  import SearchPane from "../components/SearchPane.svelte";
  import CaptureForm from "../components/CaptureForm.svelte";

  // State Management
  let currentUser = $state(/** @type {any} */ (null)); // { id, username }
  let activeSession = $state(/** @type {any} */ (null)); // { id, name, recordCount }
  let view = $state("auth"); // "auth", "dashboard", "workspace"

  // Auth form state
  let isRegister = $state(false);
  let authUsername = $state("");
  let authPassword = $state("");
  let authError = $state("");
  let authSuccess = $state("");

  // Dashboard state
  let sessionName = $state("");
  let sessionList = $state(/** @type {any[]} */ ([]));
  let activeTab = $state("sessions"); // "sessions", "settings"
  
  // Export Settings state
  let exportFormat = $state("DwC"); // "DwC" or "BRAHMS"
  let workingCollectionCode = $state("WIOI");
  let customMappings = $state(/** @type {any} */ ({
    recordedBy: "",
    recordNumber: "",
    locality: "",
    scientificName: "",
    family: "",
    genus: "",
    specificEpithet: "",
    country: "",
    stateProvince: "",
    year: "",
    month: "",
    day: ""
  }));
  let settingsMessage = $state("");

  // Workspace state
  let activeRecord = $state(/** @type {any} */ (null)); // Selected record for CaptureForm
  let capturedRecords = $state(/** @type {any[]} */ ([])); // Records captured in the current session
  let exportMessage = $state("");
  let exportError = $state("");

  // -------------------------------------------------------------
  // Authentication Logic
  // -------------------------------------------------------------
  async function handleAuth(/** @type {any} */ e) {
    if (e) e.preventDefault();
    authError = "";
    authSuccess = "";

    if (authUsername.trim().length === 0 || authPassword.trim().length === 0) {
      authError = "Please fill out all fields.";
      return;
    }

    try {
      if (isRegister) {
        let msg = await invoke("register_user", { username: authUsername, password: authPassword });
        authSuccess = /** @type {string} */ (msg);
        isRegister = false;
        authPassword = "";
      } else {
        let user = await invoke("login_user", { username: authUsername, password: authPassword });
        if (user) {
          currentUser = user;
          view = "dashboard";
          await loadSessions();
          await loadExportSettings();
        } else {
          authError = "Invalid username or password.";
        }
      }
    } catch (err) {
      authError = (/** @type {any} */ (err)).toString();
    }
  }

  // -------------------------------------------------------------
  // Sessions Dashboard Logic
  // -------------------------------------------------------------
  async function loadSessions() {
    if (!currentUser) return;
    try {
      sessionList = await invoke("get_sessions", { userId: currentUser.id });
    } catch (e) {
      console.error(e);
    }
  }

  async function handleCreateSession(/** @type {any} */ e) {
    if (e) e.preventDefault();
    if (sessionName.trim().length === 0) return;

    try {
      let session = await invoke("create_session", { userId: currentUser.id, name: sessionName });
      sessionName = "";
      await loadSessions();
      // Auto-enter workspace
      selectSession(session);
    } catch (err) {
      alert("Error creating session: " + (/** @type {any} */ (err)).toString());
    }
  }

  async function selectSession(/** @type {any} */ session) {
    activeSession = session;
    view = "workspace";
    exportMessage = "";
    exportError = "";
    
    await loadCapturedRecords();
  }

  // -------------------------------------------------------------
  // Export Settings Mappings Logic
  // -------------------------------------------------------------
  async function loadExportSettings() {
    if (!currentUser) return;
    try {
      let settings = /** @type {any} */ (await invoke("get_export_settings", { userId: currentUser.id }));
      exportFormat = settings.format || "DwC";
      if (settings.mappings) {
        let maps = JSON.parse(settings.mappings);
        workingCollectionCode = maps.collectionCode || "WIOI";
        customMappings = {
          recordedBy: maps.recordedBy || "",
          recordNumber: maps.recordNumber || "",
          locality: maps.locality || "",
          scientificName: maps.scientificName || "",
          family: maps.family || "",
          genus: maps.genus || "",
          specificEpithet: maps.specificEpithet || "",
          country: maps.country || "",
          stateProvince: maps.stateProvince || "",
          year: maps.year || "",
          month: maps.month || "",
          day: maps.day || ""
        };
      }
    } catch (e) {
      console.error(e);
    }
  }

  async function handleSaveSettings() {
    if (!currentUser) return;
    settingsMessage = "";
    try {
      let mappingsObj = { ...customMappings, collectionCode: workingCollectionCode };
      await invoke("save_export_settings", {
        userId: currentUser.id,
        format: exportFormat,
        mappings: JSON.stringify(mappingsObj)
      });
      settingsMessage = "Settings saved successfully!";
      setTimeout(() => { settingsMessage = ""; }, 3000);
    } catch (e) {
      settingsMessage = "Error saving settings: " + (/** @type {any} */ (e)).toString();
    }
  }

  // -------------------------------------------------------------
  // Workspace Capturing & Data Entry Logic
  // -------------------------------------------------------------
  async function loadCapturedRecords() {
    if (!activeSession) return;
    try {
      capturedRecords = await invoke("get_captured_records", { sessionId: activeSession.id });
      // Update local active session count
      activeSession.recordCount = capturedRecords.length;
    } catch (e) {
      console.error(e);
    }
  }

  function handleSelectSearchResult(/** @type {any} */ rec) {
    // Populate the form. Note: rec contains reference DB records or previously captured records.
    activeRecord = rec;
  }

  async function handleDeleteCapturedRecord(/** @type {any} */ id, /** @type {any} */ e) {
    if (e) e.stopPropagation();
    if (!confirm("Are you sure you want to delete this captured specimen?")) return;
    try {
      await invoke("delete_captured_record", { id });
      await loadCapturedRecords();
      await loadSessions(); // Reload stats count in dashboard background
    } catch (err) {
      alert("Error deleting record: " + (/** @type {any} */ (err)).toString());
    }
  }

  async function handleDeleteSession(/** @type {number} */ id, /** @type {string} */ name, /** @type {any} */ e) {
    if (e) e.stopPropagation();
    let msg = `WARNING: Are you sure you want to permanently delete the capture session "${name}"?\n\nThis will permanently delete all captured records associated with this session. Have you downloaded/exported the data for this session?`;
    if (!confirm(msg)) return;
    
    try {
      await invoke("delete_session", { id });
      await loadSessions();
      if (activeSession && activeSession.id === id) {
        activeSession = null;
        view = "dashboard";
      }
    } catch (err) {
      alert("Error deleting session: " + (/** @type {any} */ (err)).toString());
    }
  }

  async function handleExportCSV() {
    if (!activeSession) return;
    exportMessage = "";
    exportError = "";
    
    try {
      let defaultName = `${activeSession.name.replace(/[^a-zA-Z0-9]/g, "_")}_captured.csv`;
      let path = await invoke("select_export_path", { defaultName });
      if (!path) {
        // User cancelled the dialog
        return;
      }
      
      let res = await invoke("export_session_csv", {
        sessionId: activeSession.id,
        filepath: path
      });
      exportMessage = /** @type {string} */ (res);
    } catch (err) {
      exportError = (/** @type {any} */ (err)).toString();
    }
  }

  function handleLogout() {
    currentUser = null;
    activeSession = null;
    view = "auth";
    authUsername = "";
    authPassword = "";
    authError = "";
    authSuccess = "";
  }
</script>

<div class="min-h-screen bg-slate-50 text-slate-800 flex flex-col font-sans">
  <!-- Top Navigation Header -->
  <header class="bg-white border-b border-slate-300 px-6 py-4 flex justify-between items-center z-10">
    <div class="flex items-center gap-3">
      <div class="bg-slate-800 text-white p-1.5 font-extrabold text-sm uppercase tracking-widest">
        WIOI
      </div>
      <div>
        <h1 class="text-md font-bold tracking-tight text-slate-800">Herbarium Specimen Duplicate Finder</h1>
        <p class="text-[10px] text-slate-500 font-semibold uppercase tracking-wider">Offline Data Entry Accelerator</p>
      </div>
    </div>

    <!-- Active User & Session Control -->
    {#if currentUser}
      <div class="flex items-center gap-4 text-xs font-semibold">
        <div class="text-right">
          <span class="text-slate-400 block text-[9px] uppercase tracking-wider">Logged In As</span>
          <span class="text-slate-800 font-bold">{currentUser.username}</span>
        </div>
        <button
          onclick={handleLogout}
          class="border border-slate-350 hover:bg-slate-100 px-3 py-1.5 text-[10px] uppercase font-bold tracking-wide rounded-none transition-colors"
        >
          Sign Out
        </button>
      </div>
    {/if}
  </header>

  <!-- Content Router View -->
  <main class="flex-1 flex flex-col min-h-0">
    <!-- VIEW 1: AUTHENTICATION (SIGN UP & LOGIN) -->
    {#if view === "auth"}
      <div class="flex-1 flex justify-center items-center p-6">
        <div class="w-full max-w-sm bg-white border border-slate-300 shadow-sm p-6 space-y-6">
          <div class="text-center">
            <h2 class="text-xl font-bold tracking-tight text-slate-900">{isRegister ? "Create Local Account" : "Access Database"}</h2>
            <p class="text-xs text-slate-500 mt-1 leading-relaxed">
              {isRegister 
                ? "Configure login details to manage captured sessions locally." 
                : "Enter credentials to unlock specimen databases."}
            </p>
          </div>

          {#if authError}
            <div class="p-3 text-xs bg-red-50 border border-red-200 text-red-700 font-medium">
              {authError}
            </div>
          {/if}

          {#if authSuccess}
            <div class="p-3 text-xs bg-emerald-50 border border-emerald-200 text-emerald-700 font-medium">
              {authSuccess}
            </div>
          {/if}

          <form onsubmit={handleAuth} class="space-y-4">
            <div>
              <label for="username" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">Username</label>
              <input
                id="username"
                type="text"
                placeholder="Enter username"
                bind:value={authUsername}
                class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
              />
            </div>
            <div>
              <label for="password" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">Password</label>
              <input
                id="password"
                type="password"
                placeholder="Enter password"
                bind:value={authPassword}
                class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
              />
            </div>

            <button
              type="submit"
              class="w-full bg-slate-800 hover:bg-slate-900 text-white py-2 text-xs font-bold uppercase tracking-wider rounded-none transition-colors"
            >
              {isRegister ? "Sign Up / Register" : "Sign In"}
            </button>
          </form>

          <div class="text-center pt-2">
            <button
              type="button"
              onclick={() => { isRegister = !isRegister; authError = ""; authSuccess = ""; }}
              class="text-xs text-slate-500 hover:text-slate-800 underline font-medium"
            >
              {isRegister ? "Already registered? Sign in" : "First time? Create a sign-in account"}
            </button>
          </div>
        </div>
      </div>

    <!-- VIEW 2: SESSION MANAGEMENT & EXPORT SETTINGS DASHBOARD -->
    {:else if view === "dashboard"}
      <div class="flex-1 max-w-5xl mx-auto w-full p-6 grid grid-cols-1 md:grid-cols-3 gap-6 min-h-0">
        <!-- Sidebar Navigation Tabs -->
        <div class="md:col-span-1 flex flex-col gap-2">
          <button
            onclick={() => activeTab = "sessions"}
            class="w-full text-left px-4 py-3 text-xs font-bold uppercase tracking-wider border rounded-none transition-all {activeTab === 'sessions' ? 'bg-slate-800 text-white border-slate-800' : 'bg-white text-slate-700 border-slate-300 hover:bg-slate-50'}"
          >
            📂 Capture Sessions
          </button>
          <button
            onclick={() => activeTab = "settings"}
            class="w-full text-left px-4 py-3 text-xs font-bold uppercase tracking-wider border rounded-none transition-all {activeTab === 'settings' ? 'bg-slate-800 text-white border-slate-800' : 'bg-white text-slate-700 border-slate-300 hover:bg-slate-50'}"
          >
            ⚙️ CSV Export Settings
          </button>
        </div>

        <!-- Main Dashboard Tab Panels -->
        <div class="md:col-span-2 bg-white border border-slate-300 p-6 flex flex-col min-h-0 overflow-y-auto">
          <!-- TAB 2.1: SESSIONS MANAGER -->
          {#if activeTab === "sessions"}
            <div class="space-y-6 flex-1 flex flex-col">
              <div>
                <h2 class="text-md font-bold text-slate-900 uppercase tracking-wide">Data Capture Sessions</h2>
                <p class="text-xs text-slate-500 mt-1">Select a session to start capturing or launch a new named session.</p>
              </div>

              <!-- Session Creator Form -->
              <form onsubmit={handleCreateSession} class="flex gap-2">
                <input
                  type="text"
                  placeholder="e.g. Kirstenbosch Field Trip Jan 2026"
                  bind:value={sessionName}
                  class="flex-1 bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
                />
                <button
                  type="submit"
                  class="bg-slate-800 hover:bg-slate-900 text-white px-6 py-2 text-xs font-bold uppercase tracking-wider rounded-none transition-colors"
                >
                  Create Session
                </button>
              </form>

              <!-- Session Listing -->
              <div class="flex-1 min-h-0">
                <h3 class="text-[10px] font-bold text-slate-400 uppercase tracking-wider mb-2 border-b border-slate-100 pb-1">Session History</h3>
                
                {#if sessionList.length > 0}
                  <ul class="border border-slate-200 divide-y divide-slate-200">
                    {#each sessionList as ses}
                      <li class="hover:bg-slate-50 transition-colors flex justify-between items-center pr-4">
                        <button
                          onclick={() => selectSession(ses)}
                          class="flex-1 text-left p-4 flex justify-between items-center outline-none"
                        >
                          <div>
                            <span class="text-sm font-semibold text-slate-900 block">{ses.name}</span>
                            <span class="text-[10px] text-slate-400 font-semibold uppercase">ID: {ses.id}</span>
                          </div>
                          <div class="flex items-center gap-3">
                            <span class="text-xs bg-slate-100 font-bold px-2 py-1 border border-slate-300">
                              {ses.recordCount} specimens
                            </span>
                            <span class="text-slate-400 font-bold text-sm">→</span>
                          </div>
                        </button>
                        <button
                          onclick={(e) => handleDeleteSession(ses.id, ses.name, e)}
                          class="bg-red-50 hover:bg-red-100 text-red-650 border border-red-200 px-3 py-1.5 text-xs font-bold uppercase tracking-wide transition-colors ml-2"
                        >
                          Delete
                        </button>
                      </li>
                    {/each}
                  </ul>
                {:else}
                  <div class="h-48 flex flex-col justify-center items-center text-slate-400 border border-dashed border-slate-300 p-6 text-center">
                    <span class="text-xs font-medium uppercase tracking-wider mb-1">No Sessions Available</span>
                    <span class="text-[11px] text-slate-400 max-w-xs">Create a new named session above to get started with capturing specimens.</span>
                  </div>
                {/if}
              </div>
            </div>

          <!-- TAB 2.2: EXPORT MAPPINGS CONFIG -->
          {:else if activeTab === "settings"}
            <div class="space-y-6">
              <div>
                <h2 class="text-md font-bold text-slate-900 uppercase tracking-wide">CSV Field Mapping Settings</h2>
                <p class="text-xs text-slate-500 mt-1">Configure the output format to match mappings required by herbaria databases (e.g. BRAHMS).</p>
              </div>

              {#if settingsMessage}
                <div class="p-3 text-xs bg-emerald-50 border border-emerald-300 text-emerald-800 font-medium">
                  {settingsMessage}
                </div>
              {/if}

              <!-- Collection Code Setting -->
              <div class="space-y-2">
                <label for="settings-collectionCode" class="block text-xs font-bold text-slate-700 uppercase tracking-wider">Working Collection Code</label>
                <input
                  id="settings-collectionCode"
                  type="text"
                  placeholder="e.g. WIOI"
                  bind:value={workingCollectionCode}
                  class="w-full sm:w-64 bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
                />
              </div>

              <!-- Format Choice -->
              <div class="space-y-2">
                <span class="block text-xs font-bold text-slate-700 uppercase tracking-wider">Export Protocol Format</span>
                <div class="flex gap-4">
                  <label class="flex items-center gap-2 text-xs font-medium text-slate-700 cursor-pointer">
                    <input
                      type="radio"
                      name="export-format"
                      value="DwC"
                      bind:group={exportFormat}
                      class="text-slate-800"
                    />
                    <span>Darwin Core Standard (DwC Headers)</span>
                  </label>
                  <label class="flex items-center gap-2 text-xs font-medium text-slate-700 cursor-pointer">
                    <input
                      type="radio"
                      name="export-format"
                      value="BRAHMS"
                      bind:group={exportFormat}
                      class="text-slate-800"
                    />
                    <span>BRAHMS (Predefined Mappings)</span>
                  </label>
                </div>
              </div>

              <!-- Custom Mappings form -->
              {#if exportFormat === "DwC"}
                <div class="space-y-3 pt-2">
                  <div class="border-b border-slate-200 pb-2">
                    <h3 class="text-xs font-bold text-slate-800 uppercase tracking-wider">Custom Header Overrides</h3>
                    <p class="text-[10px] text-slate-400">Leave blank to output standard Darwin Core headers, or enter custom header strings.</p>
                  </div>

                  <div class="grid grid-cols-2 gap-3 max-h-80 overflow-y-auto pr-2">
                    {#each Object.keys(customMappings) as field}
                      <div class="flex flex-col">
                        <span class="text-[10px] font-bold text-slate-500">{field}</span>
                        <input
                          type="text"
                          placeholder={field}
                          bind:value={customMappings[field]}
                          class="bg-white border border-slate-300 text-slate-800 text-xs px-2 py-1.5 outline-none focus:border-slate-500 rounded-none transition-all mt-1"
                        />
                      </div>
                    {/each}
                  </div>
                </div>
              {:else if exportFormat === "BRAHMS"}
                <div class="bg-slate-50 border border-slate-200 p-4 space-y-2 text-xs text-slate-600 leading-relaxed">
                  <span class="font-bold text-slate-800 uppercase tracking-wider block mb-1">Predefined BRAHMS Mappings Active</span>
                  <span>When exporting your CSV, headers will automatically be transformed into the BRAHMS layout:</span>
                  <div class="grid grid-cols-2 gap-x-4 gap-y-1 text-[10px] mt-2 font-mono bg-white p-2 border border-slate-200">
                    <div>recordedBy → COLLECTOR</div>
                    <div>recordNumber → NUMBER</div>
                    <div>locality → LOCALITY</div>
                    <div>locationNotes → LOC_NOTES</div>
                    <div>scientificName → TAXON</div>
                    <div>stateProvince → PROVINCE</div>
                  </div>
                </div>
              {/if}

              <!-- Save settings button -->
              <div class="pt-4 border-t border-slate-100 flex justify-end">
                <button
                  type="button"
                  onclick={handleSaveSettings}
                  class="bg-slate-800 hover:bg-slate-900 text-white px-6 py-2 text-xs font-bold uppercase tracking-wider rounded-none transition-colors"
                >
                  Save Settings Mappings
                </button>
              </div>
            </div>
          {/if}
        </div>
      </div>

    <!-- VIEW 3: MAIN DUAL-PANE WORKSPACE -->
    {:else if view === "workspace"}
      <div class="flex-1 flex flex-col min-h-0 bg-slate-100">
        <!-- Workspace Header Sub-Bar -->
        <div class="px-6 py-3 bg-slate-800 text-white flex justify-between items-center shrink-0">
          <div class="flex items-center gap-3">
            <button
              onclick={async () => { view = "dashboard"; await loadSessions(); }}
              class="bg-slate-700 hover:bg-slate-600 text-xs font-bold uppercase px-3 py-1.5 tracking-wide rounded-none transition-colors"
            >
              ← Dashboard
            </button>
            <div>
              <span class="text-[9px] uppercase tracking-wider text-slate-400 block font-semibold">Active Session</span>
              <span class="text-xs font-bold text-slate-100">{activeSession.name}</span>
            </div>
          </div>

          <div class="flex items-center gap-4">
            <!-- Active stats count -->
            <div class="text-right">
              <span class="text-[9px] uppercase tracking-wider text-slate-400 block font-semibold">Specimens Captured</span>
              <span class="text-xs font-extrabold text-emerald-400">{activeSession.recordCount} records</span>
            </div>
            
            <!-- Export Session CSV bar -->
            <div class="flex items-center">
              <button
                onclick={handleExportCSV}
                class="bg-emerald-600 hover:bg-emerald-700 text-white px-5 py-2 text-xs font-bold uppercase tracking-wider rounded-none transition-colors"
              >
                Export CSV
              </button>
            </div>
          </div>
        </div>

        <!-- Export status notification -->
        {#if exportMessage}
          <div class="bg-emerald-50 border-b border-emerald-300 text-emerald-800 text-xs px-6 py-2 flex justify-between items-center">
            <span>✨ {exportMessage}</span>
            <button onclick={() => { exportMessage = ""; }} class="font-bold">✕</button>
          </div>
        {:else if exportError}
          <div class="bg-red-50 border-b border-red-300 text-red-800 text-xs px-6 py-2 flex justify-between items-center">
            <span>⚠️ Export failed: {exportError}</span>
            <button onclick={() => { exportError = ""; }} class="font-bold">✕</button>
          </div>
        {/if}

        <!-- Workspace Dual-Pane Layout -->
        <div class="flex-1 grid grid-cols-1 lg:grid-cols-2 gap-4 p-4 min-h-0 overflow-y-auto">
          <!-- Left Pane (Search) -->
          <div class="flex flex-col h-[650px] min-h-0">
            <SearchPane onSelectRecord={handleSelectSearchResult} />
          </div>

          <!-- Right Pane (Specimen Capture Form) -->
          <div class="flex flex-col h-[650px] min-h-0">
            <CaptureForm 
              sessionId={activeSession.id} 
              collectionCode={workingCollectionCode}
              bind:activeRecord={activeRecord} 
              onSaveSuccess={async () => { await loadCapturedRecords(); await loadSessions(); }} 
            />
          </div>
        </div>

        <!-- Bottom Panel: Captured Specimens in This Session -->
        <div class="border-t border-slate-300 bg-white p-4 shrink-0 max-h-60 flex flex-col">
          <div class="flex justify-between items-center mb-2">
            <h3 class="text-xs font-bold text-slate-800 uppercase tracking-wide">Specimens Saved in this Session</h3>
            <span class="text-[10px] text-slate-400 font-semibold uppercase">{capturedRecords.length} records</span>
          </div>

          <div class="flex-1 overflow-y-auto border border-slate-200">
            {#if capturedRecords.length > 0}
              <table class="w-full text-left text-xs border-collapse">
                <thead>
                  <tr class="bg-slate-50 border-b border-slate-350 text-slate-600 font-bold uppercase tracking-wider">
                    <th class="p-2">Collector</th>
                    <th class="p-2">Taxon Name</th>
                    <th class="p-2">Locality</th>
                    <th class="p-2">Geom</th>
                    <th class="p-2">Date</th>
                    <th class="p-2 text-right">Actions</th>
                  </tr>
                </thead>
                <tbody class="divide-y divide-slate-100">
                  {#each capturedRecords as rec}
                    <tr 
                      onclick={() => activeRecord = rec}
                      class="hover:bg-slate-50 cursor-pointer transition-colors border-b border-slate-100"
                    >
                      <td class="p-2 text-slate-700 font-medium">{rec.recordedBy || 'N/A'} {rec.recordNumber ? `#${rec.recordNumber}` : ''}</td>
                      <td class="p-2 text-slate-900 italic font-semibold">{rec.scientificName}</td>
                      <td class="p-2 text-slate-500 max-w-xs truncate" title={rec.locality}>{rec.locality || 'N/A'}</td>
                      <td class="p-2 text-slate-500">{rec.country || ''} {rec.stateProvince || ''}</td>
                      <td class="p-2 text-slate-650">{rec.year ? `${rec.year}-${rec.month || '?'}-${rec.day || '?'}` : 'N/A'}</td>
                      <td class="p-2 text-right">
                        <button
                          onclick={(e) => handleDeleteCapturedRecord(rec.id, e)}
                          class="bg-red-50 hover:bg-red-100 text-red-600 border border-red-200 px-2 py-0.5 text-[10px] uppercase font-bold tracking-wide transition-colors"
                        >
                          Delete
                        </button>
                      </td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            {:else}
              <div class="py-8 text-center text-slate-400 text-xs">
                No specimens captured yet in this session. Start by entering data in the form above!
              </div>
            {/if}
          </div>
        </div>
      </div>
    {/if}
  </main>
</div>
