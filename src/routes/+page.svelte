<script>
  import { setContext } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import Papa from "papaparse";
  import { friendlyDate } from "friendly-date";
  import SearchPane from "../components/SearchPane.svelte";
  import CaptureForm from "../components/CaptureForm.svelte";
  import {parseElevation} from "../utils/parseVerbatimElevation.js";
  import {coordsToQDS } from "../utils/coordsToQDS.js";

  // State Management
  let currentUser = $state(/** @type {any} */ (null)); // { id, username }
  let activeSession = $state(/** @type {any} */ (null)); // { id, name, recordCount }
  let view = $state("loading"); // "loading", "auth", "dashboard", "workspace"
  let dbLoadingMessage = $state("Checking database and indexing reference records...");

  async function checkDb() {
    try {
      await invoke("initialize_database");
      
      const storedUser = localStorage.getItem("currentUser");
      if (storedUser) {
        currentUser = JSON.parse(storedUser);
        await loadSessions();
        await loadExportSettings();
        
        const storedSession = localStorage.getItem("lastActiveSession");
        if (storedSession) {
          const parsedSession = JSON.parse(storedSession);
          const sessionExists = sessionList.find(s => s.id === parsedSession.id);
          if (sessionExists) {
            await selectSession(sessionExists);
            return;
          }
        }
        view = "dashboard";
      } else {
        view = "auth";
      }
    } catch (e) {
      console.error(e);
      dbLoadingMessage = "Error: " + (/** @type {any} */ (e)).toString();
    }
  }

  $effect(() => {
    checkDb();
  });

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
  let editingSessionId = $state(/** @type {number|null} */ (null));
  let editingName = $state("");
  
  // Export Settings state
  let exportFormat = $state("DwC"); // "DwC" or "BRAHMS"
  let workingCollectionCode = $state("WIOI");
  let includeGridReference = $state(false);
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
  let searchPaneRef = $state(/** @type {any} */ (null));
  
  // Custom delete record modal state
  let showDeleteRecordModal = $state(false);
  let pendingDeleteRecordId = $state(/** @type {number|null} */ (null));
  let pendingDeleteRecordDetails = $state("");

  // Custom delete session modal state
  let showDeleteSessionModal = $state(false);
  let pendingDeleteSessionId = $state(/** @type {number|null} */ (null));
  let pendingDeleteSessionName = $state("");

  // Localization state
  let currentLanguage = $state(localStorage.getItem("currentLanguage") || "EN");
  let translations = $state(/** @type {Record<string, string>} */ ({}));

  function t(/** @type {string} */ key, /** @type {string} */ defaultText) {
    return translations[key] || defaultText || key;
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
          localStorage.setItem("currentUser", JSON.stringify(user));
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
      let session = await invoke("create_session", { userId: currentUser.id, name: sessionName });
      sessionName = "";
      await loadSessions();
      // Auto-enter workspace
      selectSession(session);
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
    
    // Clear editing state first so UI updates immediately
    editingSessionId = null;
    
    if (newName.length === 0 || newName === oldName) {
      // Revert if empty or unchanged
      return;
    }
    
    try {
      await invoke("rename_session", { id: ses.id, name: newName });
      await loadSessions();
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

  async function selectSession(/** @type {any} */ session) {
    activeSession = session;
    view = "workspace";
    exportMessage = "";
    exportError = "";
    
    localStorage.setItem("lastActiveSession", JSON.stringify(session));
    
    await loadCapturedRecords();
  }

  async function returnToDashboard() {
    view = "dashboard";
    activeSession = null;
    localStorage.removeItem("lastActiveSession");
    await loadSessions();
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
        includeGridReference = maps.includeGridReference || false;
      }
    } catch (e) {
      console.error(e);
    }
  }

  async function handleSaveSettings() {
    if (!currentUser) return;
    settingsMessage = "";
    try {
      let mappingsObj = { ...customMappings, collectionCode: workingCollectionCode, includeGridReference };
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
    activeRecord = { ...rec };
  }

  function promptDeleteCapturedRecord(/** @type {any} */ rec, /** @type {any} */ e) {
    if (e) e.stopPropagation();
    pendingDeleteRecordId = rec.id;
    pendingDeleteRecordDetails = `${rec.recordedBy || 'N/A'} ${rec.recordNumber ? '#' + rec.recordNumber : ''} - ${rec.scientificName}`;
    showDeleteRecordModal = true;
  }

  async function confirmDeleteCapturedRecord() {
    if (pendingDeleteRecordId === null) return;
    
    const id = pendingDeleteRecordId;
    pendingDeleteRecordId = null;
    showDeleteRecordModal = false;
    
    try {
      await invoke("delete_captured_record", { id });
      await loadCapturedRecords();
      await loadSessions(); // Reload stats count in dashboard background
    } catch (err) {
      alert("Error deleting record: " + (/** @type {any} */ (err)).toString());
    }
  }

  function cancelDeleteCapturedRecord() {
    pendingDeleteRecordId = null;
    showDeleteRecordModal = false;
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
      await invoke("delete_session", { id });
      await loadSessions();
      
      const storedSession = localStorage.getItem("lastActiveSession");
      if (storedSession) {
        const parsed = JSON.parse(storedSession);
        if (parsed.id === id) {
          localStorage.removeItem("lastActiveSession");
        }
      }
      
      if (activeSession && activeSession.id === id) {
        activeSession = null;
        view = "dashboard";
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

  function formatISO8601Date(/** @type {number|string|null} */ year, /** @type {number|string|null} */ month, /** @type {number|string|null} */ day) {
    if (!year) return "";
    let dateStr = String(year);
    if (month) {
      let m = String(month).padStart(2, "0");
      dateStr += `-${m}`;
      if (day) {
        let d = String(day).padStart(2, "0");
        dateStr += `-${d}`;
      }
    }
    return dateStr;
  }

  function parseCollectorNumber(/** @type {string|null|undefined} */ recordNumber) {
    const result = {
      prefix: "",
      number: "",
      suffix: ""
    };
    if (!recordNumber) return result;
    const str = recordNumber.trim();
    
    const yearSlashRegex = /^(.*?\b(?:19|20)?\d{2}\s*\/\s*)(\d+)(.*)$/;
    const yearSlashMatch = str.match(yearSlashRegex);
    if (yearSlashMatch) {
      result.prefix = yearSlashMatch[1];
      result.number = yearSlashMatch[2];
      result.suffix = yearSlashMatch[3];
      return result;
    }
    
    const digitRegex = /^(.*?)(\d+)(.*)$/;
    const digitMatch = str.match(digitRegex);
    if (digitMatch) {
      result.prefix = digitMatch[1];
      result.number = digitMatch[2];
      result.suffix = digitMatch[3];
      return result;
    }
    
    result.prefix = str;
    return result;
  }

  function parseScientificName(/** @type {string|null|undefined} */ name) {
    const result = {
      genus: "",
      sp1: "",
      author1: "",
      rank1: "",
      sp2: "",
      author2: ""
    };
    if (!name) return result;
    const tokens = name.trim().split(/\s+/);
    if (tokens.length === 0) return result;
    
    result.genus = tokens[0];
    if (tokens.length === 1) return result;
    
    const isLowercase = (/** @type {string} */ str) => {
      if (!str) return false;
      const firstChar = str.charAt(0);
      return firstChar >= 'a' && firstChar <= 'z';
    };

    const rankIndicators = [
      "subsp.", "subsp", "var.", "var", "ssp.", "ssp", 
      "forma", "form", "subg.", "subgenus", "sect.", "section",
      "f.", "f"
    ];
    
    let sp1Index = -1;
    if (isLowercase(tokens[1]) || tokens[1] === "x" || tokens[1] === "×") {
      result.sp1 = tokens[1];
      sp1Index = 1;
    }
    
    if (sp1Index === -1) {
      result.author1 = tokens.slice(1).join(" ");
      return result;
    }
    
    let rankIndex = -1;
    for (let i = 2; i < tokens.length; i++) {
      if (rankIndicators.includes(tokens[i].toLowerCase())) {
        rankIndex = i;
        break;
      }
    }
    
    if (rankIndex === -1) {
      result.author1 = tokens.slice(2).join(" ");
    } else {
      result.author1 = tokens.slice(2, rankIndex).join(" ");
      result.rank1 = tokens[rankIndex];
      if (rankIndex + 1 < tokens.length) {
        result.sp2 = tokens[rankIndex + 1];
        result.author2 = tokens.slice(rankIndex + 2).join(" ");
      }
    }
    return result;
  }

  function coordinatesDiffer(/** @type {string|null|undefined} */ verbatim, /** @type {number|string|null|undefined} */ lat, /** @type {number|string|null|undefined} */ lon) {
    if (!verbatim) return false;
    if (lat === null || lat === undefined || lon === null || lon === undefined || lat === "" || lon === "") return true;
    
    const cleanVerbatim = verbatim.replace(/\s+/g, "");
    const cleanDec = `${lat},${lon}`;
    const cleanDecAbs = `${Math.abs(Number(lat))},${Math.abs(Number(lon))}`;
    
    if (cleanVerbatim === cleanDec || cleanVerbatim === cleanDecAbs) {
      return false;
    }
    
    const parts = verbatim.split(/[\s,]+/);
    if (parts.length === 2) {
      const vLat = parseFloat(parts[0]);
      const vLon = parseFloat(parts[1]);
      if (!isNaN(vLat) && !isNaN(vLon)) {
        if (Math.abs(vLat - Number(lat)) < 0.00001 && Math.abs(vLon - Number(lon)) < 0.00001) {
          return false;
        }
      }
    }
    
    return true;
  }

  function isSimpleElevation(/** @type {string|null|undefined} */ verbatim) {
    if (!verbatim) return true;
    return /^\d+(?:\.\d+)?\s*m?$/i.test(verbatim.trim());
  }

  function generateCSVContent(/** @type {any[]} */ records, /** @type {string} */ format, /** @type {any} */ familyMap = {}, /** @type {boolean} */ includeQDS = false) {
    if (format === "DwC") {
      const headers = [
        "dwc:collectionCode",
        "dwc:catalogNumber",
        "duplicates",
        "dwc:recordNumber",
        "dwc:recordedBy",
        "dwc:verbatimEventDate",
        "dwc:year",
        "dwc:month",
        "dwc:day",
        "dwc:country",
        "dwc:stateProvince",
        "dwc:county",
        "dwc:municipality",
        "dwc:locality",
        "dwc:locationRemarks",
        "dwc:verbatimCoordinates",
        "dwc:decimalLatitude",
        "dwc:decimalLongitude",
        "dwc:verbatimElevation",
        "minElevation",
        "maxElevation",
        "elevation",
        "elevationUncertainty",
        "dwc:habitat",
        "dwc:occurrenceRemarks",
        "dwc:fieldNotes",
        "dwc:typeStatus",
        "dwc:identificationQualifier",
        "dwc:scientificName",
        "dwc:identifiedBy",
        "dwc:dateIdentified",
        "dwc:identificationRemarks",
        "dwc:taxonID",
        "cultivated"
      ];

      const data = records.map(rec => {
        const elevParts = parseElevation(rec.verbatimElevation);
        let dwcLocationRemarks = rec.locationNotes || "";
        if (rec.cultivated) {
          dwcLocationRemarks = dwcLocationRemarks ? `${dwcLocationRemarks}; cultivated` : "cultivated";
        }
        return [
          rec.collectionCode || "",
          rec.catalogNumber || "",
          rec.duplicates !== null && rec.duplicates !== undefined ? rec.duplicates : "",
          rec.recordNumber || "",
          rec.recordedBy || "",
          rec.verbatimEventDate || "",
          rec.year !== null && rec.year !== undefined ? rec.year : "",
          rec.month !== null && rec.month !== undefined ? rec.month : "",
          rec.day !== null && rec.day !== undefined ? rec.day : "",
          rec.country || "",
          rec.stateProvince || "",
          rec.county || "",
          rec.municipality || "",
          rec.locality || "",
          dwcLocationRemarks, // locationRemarks maps to locationNotes in UI record
          rec.verbatimCoordinates || "",
          rec.decimalLatitude !== null && rec.decimalLatitude !== undefined ? rec.decimalLatitude : "",
          rec.decimalLongitude !== null && rec.decimalLongitude !== undefined ? rec.decimalLongitude : "",
          rec.verbatimElevation || "",
          elevParts.minElevation !== null && elevParts.minElevation !== undefined ? elevParts.minElevation : "",
          elevParts.maxElevation !== null && elevParts.maxElevation !== undefined ? elevParts.maxElevation : "",
          elevParts.elevation !== null && elevParts.elevation !== undefined ? elevParts.elevation : "",
          elevParts.elevationUncertainty !== null && elevParts.elevationUncertainty !== undefined ? elevParts.elevationUncertainty : "",
          rec.habitat || "",
          rec.occurrenceRemarks || "",
          rec.fieldNotes || "",
          rec.typeStatus || "",
          rec.identificationQualifier || "",
          rec.scientificName || "",
          rec.identifiedBy || "",
          formatISO8601Date(rec.yearIdentified, rec.monthIdentified, rec.dayIdentified),
          rec.identificationRemarks || "",
          rec.taxonID || "",
          rec.cultivated ? "true" : "false"
        ];
      });

      return Papa.unparse({ fields: headers, data });
    } else {
      // BRAHMS Export Format
      const headers = [
        "tag", "del", "barcode", "dups",
        "collector", "addcol", "prefix", "number", "suffix",
        "dd", "mm", "yy",
        "family",
        "type category",
        "genus", "sp1", "author1", "rank1", "sp2", "author2",
        "detdd", "detmm", "detyy", "detstatus",
        "country", "majorarea", "minorarea", "gazetteer",
        "lat", "long", "ns", "ew", "llunit"
      ];
      if (includeQDS) {
        headers.push("qds");
      }
      headers.push(
        "alt", "altunit",
        "locality notes",
        "habitat/site description",
        "plant description", "cultivated",
        "general notes"
      );

      const data = records.map(rec => {
        const collectors = rec.recordedBy ? rec.recordedBy.split(';').map((/** @type {string} */ s) => s.trim()).filter(Boolean) : [];
        const collector = collectors[0] || "";
        const addcol = collectors.slice(1).join("; ");

        const colNumParts = parseCollectorNumber(rec.recordNumber);

        const family = familyMap[rec.id] || "";

        const nameParts = parseScientificName(rec.scientificName);

        let gazetteer = rec.locality || "";
        if (rec.municipality && rec.municipality.trim()) {
          const muni = rec.municipality.trim();
          const locLower = (rec.locality || "").toLowerCase();
          const remarksLower = (rec.locationNotes || "").toLowerCase();
          if (!locLower.includes(muni.toLowerCase()) && !remarksLower.includes(muni.toLowerCase())) {
            gazetteer = rec.locality ? `${muni}, ${rec.locality}` : muni;
          }
        }

        const hasLat = rec.decimalLatitude !== null && rec.decimalLatitude !== undefined && rec.decimalLatitude !== "";
        const hasLon = rec.decimalLongitude !== null && rec.decimalLongitude !== undefined && rec.decimalLongitude !== "";
        const lat = hasLat ? Math.abs(Number(rec.decimalLatitude)) : "";
        const long = hasLon ? Math.abs(Number(rec.decimalLongitude)) : "";
        const ns = hasLat ? (Number(rec.decimalLatitude) >= 0 ? "N" : "S") : "";
        const ew = hasLon ? (Number(rec.decimalLongitude) >= 0 ? "E" : "W") : "";

        let qdsVal = "";
        if (includeQDS) {
          try {
            qdsVal = coordsToQDS(rec.decimalLatitude, rec.decimalLongitude) || "";
          } catch (e) {
            qdsVal = "";
          }
        }

        const elevParts = parseElevation(rec.verbatimElevation);
        const alt = elevParts.elevation !== null && elevParts.elevation !== undefined ? elevParts.elevation : "";

        let localityNotes = rec.locationNotes || "";
        if (rec.cultivated) {
          localityNotes = localityNotes ? `${localityNotes}; cultivated` : "cultivated";
        }
        if (coordinatesDiffer(rec.verbatimCoordinates, rec.decimalLatitude, rec.decimalLongitude)) {
          const coordNote = `verbatim coordinates: ${rec.verbatimCoordinates}`;
          localityNotes = localityNotes ? `${localityNotes}. ${coordNote}` : coordNote;
        }
        if (!isSimpleElevation(rec.verbatimElevation)) {
          const elevNote = `verbatim elevation: ${rec.verbatimElevation}`;
          localityNotes = localityNotes ? `${localityNotes}. ${elevNote}` : elevNote;
        }

        let generalNotes = rec.occurrenceRemarks || "";
        if (rec.identificationRemarks && rec.identificationRemarks.trim()) {
          const detNotes = `detnotes: ${rec.identificationRemarks.trim()}`;
          generalNotes = generalNotes ? `${generalNotes}. ${detNotes}` : detNotes;
        }

        const row = [
          "", // tag
          "", // del
          rec.catalogNumber || "", // barcode
          rec.duplicates !== null && rec.duplicates !== undefined ? rec.duplicates : "", // dups
          collector,
          addcol,
          colNumParts.prefix,
          colNumParts.number,
          colNumParts.suffix,
          rec.day !== null && rec.day !== undefined ? rec.day : "",
          rec.month !== null && rec.month !== undefined ? rec.month : "",
          rec.year !== null && rec.year !== undefined ? rec.year : "",
          family,
          rec.typeStatus || "", // type category
          nameParts.genus,
          nameParts.sp1,
          nameParts.author1,
          nameParts.rank1,
          nameParts.sp2,
          nameParts.author2,
          rec.dayIdentified !== null && rec.dayIdentified !== undefined ? rec.dayIdentified : "",
          rec.monthIdentified !== null && rec.monthIdentified !== undefined ? rec.monthIdentified : "",
          rec.yearIdentified !== null && rec.yearIdentified !== undefined ? rec.yearIdentified : "",
          rec.identificationQualifier || "", // detstatus
          rec.country || "",
          rec.stateProvince || "", // majorarea
          rec.county || "", // minorarea
          gazetteer,
          lat,
          long,
          ns,
          ew,
          "DD" // llunit
        ];

        if (includeQDS) {
          row.push(qdsVal);
        }

        row.push(
          alt,
          "", // altunit
          localityNotes,
          rec.habitat || "", // habitat/site description
          rec.fieldNotes || "", // plant description
          rec.cultivated ? "true" : "false", // cultivated
          generalNotes
        );

        return row;
      });

      return Papa.unparse({ fields: headers, data });
    }
  }

  async function handleExportCSV() {
    if (!activeSession) return;
    exportMessage = "";
    exportError = "";
    
    try {
      const records = /** @type {any[]} */ (await invoke("get_captured_records", { sessionId: activeSession.id }));
      if (!records || records.length === 0) {
        exportError = "No records to export in this session.";
        return;
      }

      const dt = new Date().toISOString().replace(/:/g, "-").split('.')[0] + 'Z';
      let defaultName = `${activeSession.name.replace(/[^a-zA-Z0-9]/g, "_")}_${dt}.csv`;
      let path = await invoke("select_export_path", { defaultName });
      if (!path) {
        // User cancelled the dialog
        return;
      }

      // Batch resolve families from WCVP database
      const queries = records.map(rec => ({
        id: rec.id,
        taxonID: rec.taxonID,
        scientificName: rec.scientificName
      }));
      const familyMap = await invoke("resolve_wcvp_families", { queries });
      
      const csvContent = generateCSVContent(records, exportFormat, familyMap, includeGridReference);
      
      let res = await invoke("export_session_csv", {
        sessionId: activeSession.id,
        filepath: path,
        csvContent
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
    
    localStorage.removeItem("currentUser");
    localStorage.removeItem("lastActiveSession");
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
        <h1 data-i18n-key="app-title" class="text-md font-bold tracking-tight text-slate-800">{t("app-title", "Herbarium Specimen Duplicate Finder")}</h1>
        <p data-i18n-key="app-subtitle" class="text-[10px] text-slate-500 font-semibold uppercase tracking-wider">{t("app-subtitle", "Offline Data Entry Accelerator")}</p>
      </div>
    </div>

    <!-- Active User & Session Control -->
    <div class="flex items-center gap-4 text-xs font-semibold">
      <!-- Language Selector -->
      <div class="flex items-center border border-slate-300 divide-x divide-slate-300 select-none">
        <button
          type="button"
          onclick={() => currentLanguage = "EN"}
          class="px-2.5 py-1.5 text-[10px] font-bold tracking-wider hover:bg-slate-50 transition-colors cursor-pointer {currentLanguage === 'EN' ? 'bg-slate-800 text-white hover:bg-slate-800' : 'bg-white text-slate-650'}"
        >
          EN
        </button>
        <button
          type="button"
          onclick={() => currentLanguage = "FR"}
          class="px-2.5 py-1.5 text-[10px] font-bold tracking-wider hover:bg-slate-50 transition-colors cursor-pointer {currentLanguage === 'FR' ? 'bg-slate-800 text-white hover:bg-slate-800' : 'bg-white text-slate-650'}"
        >
          FR
        </button>
        <button
          type="button"
          onclick={() => currentLanguage = "PT"}
          class="px-2.5 py-1.5 text-[10px] font-bold tracking-wider hover:bg-slate-50 transition-colors cursor-pointer {currentLanguage === 'PT' ? 'bg-slate-800 text-white hover:bg-slate-800' : 'bg-white text-slate-650'}"
        >
          PT
        </button>
        <button
          type="button"
          onclick={() => currentLanguage = "MG"}
          class="px-2.5 py-1.5 text-[10px] font-bold tracking-wider hover:bg-slate-50 transition-colors cursor-pointer {currentLanguage === 'MG' ? 'bg-slate-800 text-white hover:bg-slate-800' : 'bg-white text-slate-650'}"
        >
          MG
        </button>
      </div>

      {#if currentUser}
        <div class="text-right">
          <span data-i18n-key="logged-in-as" class="text-slate-400 block text-[9px] uppercase tracking-wider">{t("logged-in-as", "Logged In As")}</span>
          <span class="text-slate-800 font-bold">{currentUser.username}</span>
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
    {#if view === "loading"}
      <div class="flex-1 flex flex-col justify-center items-center p-6 bg-slate-50">
        <div class="w-full max-w-sm bg-white border border-slate-300 shadow-sm p-8 flex flex-col items-center text-center space-y-4">
          <!-- Spinner -->
          <div class="w-8 h-8 border-4 border-slate-200 border-t-slate-800 rounded-full animate-spin"></div>
          <div>
            <h2 data-i18n-key="preparing-db" class="text-sm font-bold text-slate-800 uppercase tracking-wider">{t("preparing-db", "Preparing Database")}</h2>
            <p class="text-xs text-slate-550 mt-2 leading-relaxed">
              {dbLoadingMessage}
            </p>
          </div>
        </div>
      </div>
    {:else if view === "auth"}
      <div class="flex-1 flex justify-center items-center p-6">
        <div class="w-full max-w-sm bg-white border border-slate-300 shadow-sm p-6 space-y-6">
          <div class="text-center">
            <h2 class="text-xl font-bold tracking-tight text-slate-900">
              {isRegister ? t("register-heading", "Register New Account") : t("sign-in-heading", "Sign In")}
            </h2>
            <p class="text-xs text-slate-500 mt-1 leading-relaxed">
              {isRegister 
                ? t("register-desc", "Configure login details to manage captured sessions locally.") 
                : t("sign-in-desc", "Enter credentials to unlock specimen databases.")}
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
              <label for="username" data-i18n-key="username-label" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">{t("username-label", "Username")}</label>
              <input
                id="username"
                type="text"
                placeholder={t("username-placeholder", "Enter username")}
                bind:value={authUsername}
                class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
              />
            </div>
            <div>
              <label for="password" data-i18n-key="password-label" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">{t("password-label", "Password")}</label>
              <input
                id="password"
                type="password"
                placeholder={t("password-placeholder", "Enter password")}
                bind:value={authPassword}
                class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
              />
            </div>

            <button
              type="submit"
              class="w-full bg-slate-800 hover:bg-slate-900 text-white py-2 text-xs font-bold uppercase tracking-wider rounded-none transition-colors"
            >
              {isRegister ? t("register-btn", "Create Account") : t("sign-in-btn", "Sign In")}
            </button>
          </form>

          <div class="text-center pt-2">
            <button
              type="button"
              onclick={() => { isRegister = !isRegister; authError = ""; authSuccess = ""; }}
              class="text-xs text-slate-500 hover:text-slate-800 underline font-medium"
            >
              {isRegister ? t("already-have-account", "Already have an account? Sign In") : t("need-account", "Need an account? Register")}
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
            {t("capture-sessions-heading", "Capture Sessions")}
          </button>
          <button
            onclick={() => activeTab = "settings"}
            class="w-full text-left px-4 py-3 text-xs font-bold uppercase tracking-wider border rounded-none transition-all {activeTab === 'settings' ? 'bg-slate-800 text-white border-slate-800' : 'bg-white text-slate-700 border-slate-300 hover:bg-slate-50'}"
          >
            {t("export-settings", "Export Settings")}
          </button>
        </div>

        <!-- Main Dashboard Tab Panels -->
        <div class="md:col-span-2 bg-white border border-slate-300 p-6 flex flex-col min-h-0 overflow-y-auto">
          <!-- TAB 2.1: SESSIONS MANAGER -->
          {#if activeTab === "sessions"}
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
                <h3 data-i18n-key="session-history-heading" class="text-[10px] font-bold text-slate-400 uppercase tracking-wider mb-2 border-b border-slate-100 pb-1">{t("session-history-heading", "Session History")}</h3>
                
                {#if sessionList.length > 0}
                  <ul class="border border-slate-200 divide-y divide-slate-200">
                    {#each sessionList as ses}
                      <li class="hover:bg-slate-50 transition-colors flex justify-between items-center pr-4">
                        <!-- svelte-ignore a11y_click_events_have_key_events -->
                        <!-- svelte-ignore a11y_no_static_element_interactions -->
                        <div
                          onclick={() => selectSession(ses)}
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
                                class="text-sm font-semibold text-slate-900 block cursor-pointer hover:text-slate-600 hover:underline"
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
                                    {friendlyDate(ses.lastRecordAt.replace(' ', 'T') + 'Z')}
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
                                      {friendlyDate(ses.lastExportedAt.replace(' ', 'T') + 'Z')}
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
                          class="bg-red-50 hover:bg-red-100 text-red-650 border border-red-200 px-3 py-1.5 text-xs font-bold uppercase tracking-wide transition-colors ml-2"
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

          <!-- TAB 2.2: APP SETTINGS -->
          {:else if activeTab === "settings"}
            <div class="space-y-6">
              <div>
                <h2 data-i18n-key="application-settings" class="text-md font-bold text-slate-900 uppercase tracking-wide">{t("application-settings", "Application settings")}</h2>
                <p data-i18n-key="settings-description" class="text-xs text-slate-500 mt-1">{t("settings-description", "Configure the collection code and export format for your herbarium.")}</p>
              </div>

              {#if settingsMessage}
                <div class="p-3 text-xs bg-emerald-50 border border-emerald-300 text-emerald-800 font-medium">
                  {settingsMessage}
                </div>
              {/if}

              <!-- Collection Code Setting -->
              <div class="space-y-2">
                <label for="settings-collectionCode" data-i18n-key="working-collection-code" class="block text-xs font-bold text-slate-700 uppercase tracking-wider">{t("working-collection-code", "Working Collection Code")}</label>
                <input
                  id="settings-collectionCode"
                  type="text"
                  placeholder="e.g. TAN"
                  bind:value={workingCollectionCode}
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
                      bind:group={exportFormat}
                      class="text-slate-800"
                    />
                    <span>Darwin Core</span>
                  </label>
                  <label class="flex items-center gap-2 text-xs font-medium text-slate-700 cursor-pointer">
                    <input
                      type="radio"
                      name="export-format"
                      value="BRAHMS"
                      bind:group={exportFormat}
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
                    bind:checked={includeGridReference}
                    class="w-4 h-4 text-slate-855 border-slate-300 rounded focus:ring-slate-500 focus:ring-1 cursor-pointer"
                  />
                  <span data-i18n-key="include-qds-label">{t("include-qds-label", "Include grid reference (QDS)")}</span>
                </label>
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
              <span class="text-xs font-bold text-slate-100">{activeSession.name}</span>
            </div>
          </div>

          <div class="flex items-center gap-4">
            <!-- Active stats count -->
            <div class="text-right">
              <span data-i18n-key="specimens-captured-heading" class="text-[9px] uppercase tracking-wider text-slate-400 block font-semibold">{t("specimens-captured-heading", "Specimens Captured")}</span>
              <span class="text-xs font-extrabold text-emerald-400">{activeSession.recordCount} {t("records-count", "records")}</span>
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
        {#if exportMessage}
          <div class="bg-emerald-50 border-b border-emerald-300 text-emerald-800 text-xs px-6 py-2 flex justify-between items-center">
            <span>{exportMessage}</span>
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
            <SearchPane bind:this={searchPaneRef} onSelectRecord={handleSelectSearchResult} />
          </div>

          <!-- Right Pane (Specimen Capture Form) -->
          <div class="flex flex-col h-[650px] min-h-0">
            <CaptureForm 
              sessionId={activeSession.id} 
              collectionCode={workingCollectionCode}
              bind:activeRecord={activeRecord} 
              onSaveSuccess={async () => {
                await loadCapturedRecords();
                await loadSessions();
                if (searchPaneRef) {
                  searchPaneRef.clearSearch();
                }
              }} 
            />
          </div>
        </div>

        <!-- Bottom Panel: Captured Specimens in This Session -->
        <div class="border-t border-slate-300 bg-white p-4 shrink-0 max-h-60 flex flex-col">
          <div class="flex justify-between items-center mb-2">
            <h3 data-i18n-key="specimens-saved-title" class="text-xs font-bold text-slate-800 uppercase tracking-wide">{t("specimens-saved-title", "Specimens Saved in this Session")}</h3>
            <span class="text-[10px] text-slate-400 font-semibold uppercase">{capturedRecords.length} {t("records-count", "records")}</span>
          </div>

          <div class="flex-1 overflow-y-auto border border-slate-200">
            {#if capturedRecords.length > 0}
              <table class="w-full text-left text-xs border-collapse">
                <thead>
                  <tr class="bg-slate-50 border-b border-slate-350 text-slate-600 font-bold uppercase tracking-wider">
                    <th data-i18n-key="collector-col" class="p-2">{t("collector-col", "Collector")}</th>
                    <th data-i18n-key="taxon-col" class="p-2">{t("taxon-col", "Taxon Name")}</th>
                    <th data-i18n-key="locality-col" class="p-2">{t("locality-col", "Locality")}</th>
                    <th data-i18n-key="geom-col" class="p-2">{t("geom-col", "Geom")}</th>
                    <th data-i18n-key="date-col" class="p-2">{t("date-col", "Date")}</th>
                    <th data-i18n-key="actions-col" class="p-2 text-right">{t("actions-col", "Actions")}</th>
                  </tr>
                </thead>
                <tbody class="divide-y divide-slate-100">
                  {#each capturedRecords as rec}
                    <tr 
                      onclick={() => activeRecord = { ...rec }}
                      class="hover:bg-slate-50 cursor-pointer transition-colors border-b border-slate-100"
                    >
                      <td class="p-2 text-slate-700 font-medium">{rec.recordedBy || 'N/A'} {rec.recordNumber ? `#${rec.recordNumber}` : ''}</td>
                      <td class="p-2 text-slate-900 italic font-semibold">{rec.scientificName}</td>
                      <td class="p-2 text-slate-500 max-w-xs truncate" title={rec.locality}>{rec.locality || 'N/A'}</td>
                      <td class="p-2 text-slate-500">{rec.country || ''} {rec.stateProvince || ''}</td>
                      <td class="p-2 text-slate-650">{rec.year ? `${rec.year}-${rec.month || '?'}-${rec.day || '?'}` : 'N/A'}</td>
                      <td class="p-2 text-right">
                        <button
                          data-i18n-key="delete-btn"
                          onclick={(e) => promptDeleteCapturedRecord(rec, e)}
                          class="bg-red-50 hover:bg-red-100 text-red-600 border border-red-200 px-2 py-0.5 text-[10px] uppercase font-bold tracking-wide transition-colors"
                        >
                          {t("delete-btn", "Delete")}
                        </button>
                      </td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            {:else}
              <div data-i18n-key="no-records-session" class="py-8 text-center text-slate-400 text-xs">
                {t("no-records-session", "No specimens captured yet in this session. Start by entering data in the form above!")}
              </div>
            {/if}
          </div>
        </div>
      </div>
    {/if}
  {#if showDeleteRecordModal}
    <div 
      class="fixed inset-0 z-[120] flex items-center justify-center bg-slate-900/40 backdrop-blur-xs p-4"
      role="dialog"
      aria-modal="true"
      tabindex="-1"
      onclick={(e) => { if (e.target === e.currentTarget) cancelDeleteCapturedRecord(); }}
      onkeydown={(e) => { 
        if (e.key === "Escape") {
          e.preventDefault();
          cancelDeleteCapturedRecord(); 
        } else if (e.key === "Enter") {
          e.preventDefault();
          confirmDeleteCapturedRecord();
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
          <div class="space-y-1">
            <h3 data-i18n-key="delete-record-heading" class="font-bold text-red-700">{t("delete-record-heading", "Delete Specimen Record")}</h3>
            <p data-i18n-key="delete-record-confirm" class="text-sm text-slate-500 leading-relaxed">
              {t("delete-record-confirm", "Are you sure you want to permanently delete this captured record?")}
            </p>
            <p class="text-xs font-semibold text-slate-700 bg-slate-50 p-2 border border-slate-150 break-all">{pendingDeleteRecordDetails}</p>
          </div>
        </div>
        
        <div class="flex justify-end gap-2 mt-2">
          <button
            type="button"
            data-i18n-key="cancel-btn"
            onclick={cancelDeleteCapturedRecord}
            class="px-3 py-1.5 text-xs font-semibold text-slate-500 hover:bg-slate-50 border border-slate-200 transition-colors cursor-pointer rounded-none"
          >
            {t("cancel-btn", "Cancel")}
          </button>
          <button
            type="button"
            data-i18n-key="delete-btn"
            onclick={confirmDeleteCapturedRecord}
            class="px-3.5 py-1.5 text-xs font-semibold text-white bg-red-650 bg-red-400 hover:bg-red-700 transition-colors cursor-pointer rounded-none"
          >
            {t("delete-btn", "Delete")}
          </button>
        </div>
      </div>
    </div>
  {/if}
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
  </main>
</div>
