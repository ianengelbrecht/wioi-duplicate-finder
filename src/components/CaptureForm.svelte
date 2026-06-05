<script>
  import { onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { convert } from 'geo-coordinates-parser';
  import { titleCase, SMALL_WORDS } from "title-case";
  import Autocomplete from "./Autocomplete.svelte";
  import MultiSelectAutocomplete from "./MultiSelectAutocomplete.svelte";

  let {
    sessionId = null,
    collectionCode = "WIOI",
    activeRecord = $bindable(null), // The selected record to edit (or empty for new)
    onSaveSuccess = () => {}
  } = $props();

  let form = $state({
    id: /** @type {number|null} */ (null),
    collectionCode: "",
    catalogNumber: "",
    duplicates: "",
    recordedBy: "",
    additionalCollectors: /** @type {string[]} */ ([]),
    recordNumber: "",
    verbatimEventDate: "",
    year: "",
    month: "",
    day: "",
    country: "",
    stateProvince: "", // Label Admin 2
    county: "",        // Label Admin 3
    municipality: "",  // Label Admin 4
    locality: "",
    verbatimCoordinates: "",
    decimalLatitude: "",
    decimalLongitude: "",
    locationNotes: "", // Mapped to locationRemarks
    verbatimLocality: "",
    verbatimElevation: "",
    habitat: "",
    identificationQualifier: "", // cf., aff., nr.
    scientificName: "",
    taxonID: "",
    typeStatus: "",
    identifiedBy: /** @type {string[]} */ ([]),
    yearIdentified: "",
    monthIdentified: "",
    dayIdentified: "",
    identificationRemarks: "",
    occurrenceRemarks: "",
    fieldNotes: ""
  });

  let coordinatesError = $state(false);
  let lastSavedRecord = $state(/** @type {any} */ (null));

  let isAnyGeoPopulated = $derived(
    !!((form.country && form.country.trim().length > 0) ||
       (form.stateProvince && form.stateProvince.trim().length > 0) ||
       (form.county && form.county.trim().length > 0) ||
       (form.municipality && form.municipality.trim().length > 0))
  );

  let saving = $state(false);
  let statusMessage = $state("");
  let statusType = $state(""); // "success" or "error"

  let verbatimLocalityRef = $state(/** @type {HTMLTextAreaElement|null} */ (null));
  let verbatimLocalityCopied = $state(false);
  /** @type {any} */
  let copyTimeoutId = null;

  function handleCopyVerbatimLocality() {
    if (!verbatimLocalityRef) return;
    
    const start = verbatimLocalityRef.selectionStart;
    const end = verbatimLocalityRef.selectionEnd;
    let textToCopy = "";
    
    if (start !== null && end !== null && start !== end) {
      textToCopy = verbatimLocalityRef.value.substring(start, end);
    } else {
      textToCopy = form.verbatimLocality || "";
    }
    
    if (textToCopy) {
      navigator.clipboard.writeText(textToCopy)
        .then(() => {
          verbatimLocalityCopied = true;
          if (copyTimeoutId) clearTimeout(copyTimeoutId);
          copyTimeoutId = setTimeout(() => {
            verbatimLocalityCopied = false;
          }, 2000);
        })
        .catch(err => {
          console.error("Failed to copy text:", err);
        });
    }
  }

  onDestroy(() => {
    if (copyTimeoutId) clearTimeout(copyTimeoutId);
    if (localityCopyTimeout) clearTimeout(localityCopyTimeout);
    if (locationNotesCopyTimeout) clearTimeout(locationNotesCopyTimeout);
  });

  let localityInputRef = $state(/** @type {HTMLInputElement|null} */ (null));
  let localityCopied = $state(false);
  /** @type {any} */
  let localityCopyTimeout = null;

  function handleCopyLocality() {
    if (!localityInputRef) return;
    const start = localityInputRef.selectionStart;
    const end = localityInputRef.selectionEnd;
    let textToCopy = "";
    if (start !== null && end !== null && start !== end) {
      textToCopy = localityInputRef.value.substring(start, end);
    } else {
      textToCopy = form.locality || "";
    }
    if (textToCopy) {
      navigator.clipboard.writeText(textToCopy)
        .then(() => {
          localityCopied = true;
          if (localityCopyTimeout) clearTimeout(localityCopyTimeout);
          localityCopyTimeout = setTimeout(() => {
            localityCopied = false;
          }, 2000);
        })
        .catch(err => console.error(err));
    }
  }

  async function handlePasteLocality() {
    if (!localityInputRef) return;
    try {
      const clipboardText = await navigator.clipboard.readText();
      if (!clipboardText) return;
      
      const start = localityInputRef.selectionStart || 0;
      const end = localityInputRef.selectionEnd || 0;
      const val = form.locality || "";
      
      form.locality = val.substring(0, start) + clipboardText + val.substring(end);
      
      setTimeout(() => {
        if (localityInputRef) {
          localityInputRef.focus();
          localityInputRef.setSelectionRange(start + clipboardText.length, start + clipboardText.length);
        }
      }, 0);
    } catch (err) {
      console.error("Failed to paste from clipboard:", err);
    }
  }

  let locationNotesRef = $state(/** @type {HTMLTextAreaElement|null} */ (null));
  let locationNotesCopied = $state(false);
  /** @type {any} */
  let locationNotesCopyTimeout = null;

  function handleCopyLocationNotes() {
    if (!locationNotesRef) return;
    const start = locationNotesRef.selectionStart;
    const end = locationNotesRef.selectionEnd;
    let textToCopy = "";
    if (start !== null && end !== null && start !== end) {
      textToCopy = locationNotesRef.value.substring(start, end);
    } else {
      textToCopy = form.locationNotes || "";
    }
    if (textToCopy) {
      navigator.clipboard.writeText(textToCopy)
        .then(() => {
          locationNotesCopied = true;
          if (locationNotesCopyTimeout) clearTimeout(locationNotesCopyTimeout);
          locationNotesCopyTimeout = setTimeout(() => {
            locationNotesCopied = false;
          }, 2000);
        })
        .catch(err => console.error(err));
    }
  }

  async function handlePasteLocationNotes() {
    if (!locationNotesRef) return;
    try {
      const clipboardText = await navigator.clipboard.readText();
      if (!clipboardText) return;
      
      const start = locationNotesRef.selectionStart || 0;
      const end = locationNotesRef.selectionEnd || 0;
      const val = form.locationNotes || "";
      
      form.locationNotes = val.substring(0, start) + clipboardText + val.substring(end);
      
      setTimeout(() => {
        if (locationNotesRef) {
          locationNotesRef.focus();
          locationNotesRef.setSelectionRange(start + clipboardText.length, start + clipboardText.length);
        }
      }, 0);
    } catch (err) {
      console.error("Failed to paste from clipboard:", err);
    }
  }

  // Dropdown suggestions lists
  let taxonSuggestions = $state(/** @type {any[]} */ ([]));
  let localitySuggestions = $state(/** @type {any[]} */ ([]));
  let collectorSuggestions = $state(/** @type {any[]} */ ([]));
  let additionalCollectorsSuggestions = $state(/** @type {any[]} */ ([]));
  let identifiedBySuggestions = $state(/** @type {any[]} */ ([]));
  let countrySuggestions = $state(/** @type {any[]} */ ([]));
  let stateProvinceSuggestions = $state(/** @type {any[]} */ ([]));
  let countySuggestions = $state(/** @type {any[]} */ ([]));
  let municipalitySuggestions = $state(/** @type {any[]} */ ([]));
  
  // Custom suggestion list for duplicates
  let duplicateSuggestions = $state(/** @type {any[]} */ ([]));
  const duplicateCodes = ["P", "K", "MO", "MAU"];

  let typeStatusSuggestions = $state(/** @type {string[]} */ ([]));
  const typeStatuses = [
    "type",  
    "isotype",
    "holotype",
    "syntype",
    "isosyntype",
    "isolectotype",
    "paratype",
    "lectotype",
    "paralectotype"
  ];

  // Helper to load collectionCode from active session owner's export settings
  async function loadCollectionCode() {
    form.collectionCode = collectionCode || "WIOI";
  }

  // Watch activeRecord changes (when a search result is clicked, copy it to the form!)
  $effect(() => {
    if (activeRecord) {
      form.id = activeRecord.id && activeRecord.sessionId ? activeRecord.id : null; // Only reuse id if it is a previously captured record, not a reference database record
      form.collectionCode = activeRecord.collectionCode || form.collectionCode || "WIOI";
      form.catalogNumber = activeRecord.catalogNumber || "";
      form.duplicates = activeRecord.duplicates ? String(activeRecord.duplicates) : "";
      
      if (activeRecord.recordedBy) {
        let collectors = splitNames(activeRecord.recordedBy);
        form.recordedBy = collectors[0] || "";
        form.additionalCollectors = collectors.slice(1);
      } else {
        form.recordedBy = "";
        form.additionalCollectors = [];
      }
      
      form.recordNumber = activeRecord.recordNumber || "";
      form.verbatimEventDate = activeRecord.verbatimEventDate || "";
      form.year = activeRecord.year !== null && activeRecord.year !== undefined ? activeRecord.year.toString() : "";
      form.month = activeRecord.month !== null && activeRecord.month !== undefined ? activeRecord.month.toString() : "";
      form.day = activeRecord.day !== null && activeRecord.day !== undefined ? activeRecord.day.toString() : "";
      form.country = activeRecord.country || "";
      form.stateProvince = activeRecord.stateProvince || "";
      form.county = activeRecord.county || "";
      form.municipality = activeRecord.municipality || "";
      form.locality = activeRecord.locality || "";
      
      // Populate verbatimCoordinates: use verbatimCoordinates if present, otherwise combine decimalLatitude and decimalLongitude
      if (activeRecord.verbatimCoordinates && activeRecord.verbatimCoordinates.trim().length > 0) {
        form.verbatimCoordinates = activeRecord.verbatimCoordinates;
      } else if (
        (activeRecord.decimalLatitude !== null && activeRecord.decimalLatitude !== undefined && activeRecord.decimalLatitude !== "") &&
        (activeRecord.decimalLongitude !== null && activeRecord.decimalLongitude !== undefined && activeRecord.decimalLongitude !== "")
      ) {
        form.verbatimCoordinates = `${activeRecord.decimalLatitude}, ${activeRecord.decimalLongitude}`;
      } else {
        form.verbatimCoordinates = "";
      }

      form.locationNotes = activeRecord.locationNotes || activeRecord.locationRemarks || "";
      form.verbatimLocality = activeRecord.verbatimLocality || "";

      // Populate verbatimElevation: use verbatimElevation if present, otherwise use elevation and add 'm'
      if (activeRecord.verbatimElevation && activeRecord.verbatimElevation.trim().length > 0) {
        form.verbatimElevation = activeRecord.verbatimElevation;
      } else if (activeRecord.elevation && String(activeRecord.elevation).trim().length > 0) {
        let elevStr = String(activeRecord.elevation).trim();
        form.verbatimElevation = elevStr.toLowerCase().endsWith("m") ? elevStr : `${elevStr}m`;
      } else {
        form.verbatimElevation = "";
      }

      form.habitat = activeRecord.habitat || "";
      form.identificationQualifier = activeRecord.identificationQualifier || "";
      form.scientificName = activeRecord.scientificName || "";
      form.taxonID = activeRecord.taxonID || "";
      form.typeStatus = activeRecord.typeStatus || "";
      
      if (activeRecord.identifiedBy) {
        form.identifiedBy = splitNames(activeRecord.identifiedBy);
      } else {
        form.identifiedBy = [];
      }
      
      form.yearIdentified = activeRecord.yearIdentified !== null && activeRecord.yearIdentified !== undefined ? activeRecord.yearIdentified.toString() : "";
      form.monthIdentified = activeRecord.monthIdentified !== null && activeRecord.monthIdentified !== undefined ? activeRecord.monthIdentified.toString() : "";
      form.dayIdentified = activeRecord.dayIdentified !== null && activeRecord.dayIdentified !== undefined ? activeRecord.dayIdentified.toString() : "";
      form.identificationRemarks = activeRecord.identificationRemarks || "";
      form.occurrenceRemarks = activeRecord.occurrenceRemarks || "";
      form.fieldNotes = activeRecord.fieldNotes || "";
      clearTitleCasedStates();
      
      statusMessage = "";
    }
  });

  $effect(() => {
    if (sessionId) {
      loadCollectionCode();
    }
  });

  // Autocomplete Query Methods
  async function handleTaxonInput(/** @type {any} */ val) {
    if (val.trim().length < 2) {
      taxonSuggestions = [];
      return;
    }
    try {
      taxonSuggestions = /** @type {any[]} */ (await invoke("autocomplete_scientific_name", { query: val }));
    } catch (e) {
      console.error(e);
    }
  }

  function handleTaxonSelect(/** @type {any} */ sug) {
    form.scientificName = sug.scientificName || "";
    form.taxonID = sug.taxonID || "";
  }

  function handleTypeStatusInput(/** @type {string} */ val) {
    if (!val) {
      typeStatusSuggestions = typeStatuses;
      return;
    }
    const lowerVal = val.toLowerCase();
    typeStatusSuggestions = typeStatuses.filter(t => t.toLowerCase().includes(lowerVal));
  }

  function handleTypeStatusFocus() {
    typeStatusSuggestions = typeStatuses;
  }

  async function handleLocalityInput(/** @type {any} */ val) {
    if (val.trim().length < 2) {
      localitySuggestions = [];
      return;
    }
    try {
      localitySuggestions = /** @type {any[]} */ (await invoke("autocomplete_locality", { query: val }));
    } catch (e) {
      console.error(e);
    }
  }

  async function handleCollectorInput(/** @type {any} */ val) {
    if (val.trim().length < 2) {
      collectorSuggestions = [];
      return;
    }
    try {
      const res = /** @type {any[]} */ (await invoke("autocomplete_agent", { query: val }));
      // Exclude already selected additional collectors
      collectorSuggestions = res.filter(name => !form.additionalCollectors.includes(name));
    } catch (e) {
      console.error(e);
    }
  }

  async function handleAdditionalCollectorsInput(/** @type {any} */ val) {
    if (val.trim().length < 2) {
      additionalCollectorsSuggestions = [];
      return;
    }
    try {
      const res = /** @type {any[]} */ (await invoke("autocomplete_agent", { query: val }));
      // Exclude primary collector and already selected additional collectors
      additionalCollectorsSuggestions = res.filter(name => 
        name !== form.recordedBy && 
        !form.additionalCollectors.includes(name)
      );
    } catch (e) {
      console.error(e);
    }
  }

  async function handleIdentifiedByInput(/** @type {any} */ val) {
    if (val.trim().length < 2) {
      identifiedBySuggestions = [];
      return;
    }
    try {
      const res = /** @type {any[]} */ (await invoke("autocomplete_agent", { query: val }));
      // Exclude already selected identifiedBy agents
      identifiedBySuggestions = res.filter(name => !form.identifiedBy.includes(name));
    } catch (e) {
      console.error(e);
    }
  }

  function onCountryChanged() {
    form.stateProvince = "";
    form.county = "";
    form.municipality = "";
    stateProvinceSuggestions = [];
    countySuggestions = [];
    municipalitySuggestions = [];
  }

  function onStateProvinceChanged() {
    form.county = "";
    form.municipality = "";
    countySuggestions = [];
    municipalitySuggestions = [];
  }

  function onCountyChanged() {
    form.municipality = "";
    municipalitySuggestions = [];
  }

  async function handleCountryInput(/** @type {string} */ val) {
    onCountryChanged();
    if (!val || val.trim().length === 0) {
      countrySuggestions = [];
      return;
    }
    try {
      countrySuggestions = /** @type {any[]} */ (await invoke("autocomplete_geography", {
        field: "country",
        query: val,
        country: "",
        stateProvince: "",
        county: ""
      }));
    } catch (e) {
      console.error(e);
    }
  }

  async function handleStateProvinceInput(/** @type {string} */ val) {
    onStateProvinceChanged();
    if (!val || val.trim().length === 0) {
      stateProvinceSuggestions = [];
      return;
    }
    try {
      stateProvinceSuggestions = /** @type {any[]} */ (await invoke("autocomplete_geography", {
        field: "stateProvince",
        query: val,
        country: form.country,
        stateProvince: "",
        county: ""
      }));
    } catch (e) {
      console.error(e);
    }
  }

  async function handleCountyInput(/** @type {string} */ val) {
    onCountyChanged();
    if (!val || val.trim().length === 0) {
      countySuggestions = [];
      return;
    }
    try {
      countySuggestions = /** @type {any[]} */ (await invoke("autocomplete_geography", {
        field: "county",
        query: val,
        country: form.country,
        stateProvince: form.stateProvince,
        county: ""
      }));
    } catch (e) {
      console.error(e);
    }
  }

  async function handleMunicipalityInput(/** @type {string} */ val) {
    if (!val || val.trim().length === 0) {
      municipalitySuggestions = [];
      return;
    }
    try {
      municipalitySuggestions = /** @type {any[]} */ (await invoke("autocomplete_geography", {
        field: "municipality",
        query: val,
        country: form.country,
        stateProvince: form.stateProvince,
        county: form.county
      }));
    } catch (e) {
      console.error(e);
    }
  }

  // Handle autocomplete input for duplicates (comma-separated select multiple)
  function handleDuplicateInput(/** @type {any} */ val) {
    let parts = val.split(",");
    let lastPart = parts[parts.length - 1].trim().toUpperCase();
    if (lastPart === "") {
      duplicateSuggestions = [];
      return;
    }
    duplicateSuggestions = duplicateCodes
      .filter(code => code.startsWith(lastPart) && !parts.slice(0, -1).map(/** @param {string} p */ p => p.trim().toUpperCase()).includes(code));
  }

  function handleDuplicateSelect(/** @type {any} */ sug) {
    let parts = form.duplicates.split(",");
    if (parts.length > 0) {
      parts[parts.length - 1] = sug;
    } else {
      parts = [sug];
    }
    form.duplicates = parts.map(p => p.trim()).join(", ") + ", ";
    duplicateSuggestions = [];
  }

  function handleDuplicateFocus() {
    let parts = form.duplicates.split(",");
    duplicateSuggestions = duplicateCodes
      .filter(code => !parts.map(p => p.trim().toUpperCase()).includes(code));
  }

  // Verbatim Event Date Parser
  function parseVerbatimDate() {
    let dateStr = form.verbatimEventDate.trim();
    if (!dateStr) return;

    // ISO Format: YYYY-MM-DD
    let isoRegex = /^(\d{4})-(\d{2})-(\d{2})$/;
    let isoMatch = dateStr.match(isoRegex);
    if (isoMatch) {
      form.year = String(parseInt(isoMatch[1]));
      form.month = String(parseInt(isoMatch[2]));
      form.day = String(parseInt(isoMatch[3]));
      return;
    }

    // ISO Format: YYYY-MM
    let isoMonthRegex = /^(\d{4})-(\d{2})$/;
    let isoMonthMatch = dateStr.match(isoMonthRegex);
    if (isoMonthMatch) {
      form.year = String(parseInt(isoMonthMatch[1]));
      form.month = String(parseInt(isoMonthMatch[2]));
      form.day = "";
      return;
    }

    // Format: DD Month YYYY or D Month YYYY (e.g. 20 May 2024, 20 May, 2024)
    const monthNames = [
      "january", "february", "march", "april", "may", "june",
      "july", "august", "september", "october", "november", "december"
    ];
    const monthShortNames = [
      "jan", "feb", "mar", "apr", "may", "jun",
      "jul", "aug", "sep", "oct", "nov", "dec"
    ];

    // DD Month YYYY
    let textDateRegex = /^(\d{1,2})\s+([a-zA-Z]+)\s+(\d{4})$/;
    let textDateMatch = dateStr.match(textDateRegex);
    if (textDateMatch) {
      form.day = String(parseInt(textDateMatch[1]));
      let monthStr = textDateMatch[2].toLowerCase();
      let mIdx = monthNames.indexOf(monthStr);
      if (mIdx === -1) mIdx = monthShortNames.indexOf(monthStr);
      form.month = mIdx !== -1 ? String(mIdx + 1) : "";
      form.year = textDateMatch[3];
      return;
    }

    // Month YYYY
    let monthYearRegex = /^([a-zA-Z]+)\s+(\d{4})$/;
    let monthYearMatch = dateStr.match(monthYearRegex);
    if (monthYearMatch) {
      form.day = "";
      let monthStr = monthYearMatch[1].toLowerCase();
      let mIdx = monthNames.indexOf(monthStr);
      if (mIdx === -1) mIdx = monthShortNames.indexOf(monthStr);
      form.month = mIdx !== -1 ? String(mIdx + 1) : "";
      form.year = monthYearMatch[2];
      return;
    }

    // YYYY
    let yearRegex = /^(\d{4})$/;
    let yearMatch = dateStr.match(yearRegex);
    if (yearMatch) {
      form.year = dateStr;
      form.month = "";
      form.day = "";
      return;
    }
  }

  // Proper Casing Helper Utility
  /**
   * @param {string} str
   * @returns {string}
   */
  function totitleCase(str) {
    if (!str) return "";
    const smallWords = new Set([
      ...SMALL_WORDS,
      "along", "from", "towards", "above", "below", "road", "km", "mi", "m", "ft", "side", "slope", "bank", "valley", "ridge", "peak", "mountain", "hill",
      "island", "peninsula", "cape", "bay", "gulf", "strait", "channel", "canyon", "cliff", "plateau", "desert", "forest", "wood", "swamp", "marsh",
        "river", "creek", "stream", "lake", "pond", "spring", "waterfall", "glacier", "volcano", "harbor", "fjord", "delta", "ocean", "sea", "beach", 
        "coast", "shore", "isle", "avenue", "street", "boulevard", "drive", "lane", "court", "square", "parkway", "trail", "terrace", "place",
        "le long de", "de", "vers", "au-dessus de", "au-dessous de",
  "route", "rd", "rd.",   "hwy", "hwy.",  "st", "st.",  "ave", "ave.",  "blvd", "blvd.",  "dr", "dr.",  "ln", "ln.",  "ct", "ct.",  "sq", "sq.",  "pkwy", "pkwy.",
  "trl", "trl.",  "ter", "ter.",  "pl", "pl.",  "mt", "mt.",  "mtn", "mtn.",  "mts", "mts.",  "pk", "pk.",  "pt", "pt.",  "isl", "isl.",  "is", "is.",  "pen", "pen.",
  "riv", "riv.",   "cr", "cr.",  "ck", "ck.",  "str", "str.",  "lk", "lk.",  "pd", "pd.",  "spr", "spr.",  "falls",  "val", "val.",  "vly", "vly.",  "rdg", "rdg.",
  "cl", "cl.",   "plt", "plt.",  "for", "for.",  "wd", "wd.",  "sw", "sw.",  "mar", "mar.",  "har", "har.",  "fj", "fj.",  "del", "del.",  "oc", "oc.",  "sea",
  "bch", "bch.",  "cst", "cst.",  "shr", "shr.",  "n", "s", "e", "w",  "ne", "nw", "se", "sw",  "nne", "ene", "ese", "sse",  "ssw", "wsw", "wnw", "nnw",
  "km", "mi", "m", "ft",   
  "côté", "pente", "rive", "vallée", "crête", "sommet", "montagne", "colline",  
  "île", "péninsule", "cap", "baie", "golfe", "détroit", "chenal", "canyon",
  "falaise", "plateau", "désert", "forêt", "bois", "marécage", "marais",
  "rivière", "ruisseau", "cours d'eau", "lac", "étang", "source", "cascade",
  "glacier", "volcan", "port", "fjord", "delta", "océan", "mer", "plage",
  "côte", "rivage", "îlot",
  "avenue", "rue", "boulevard", "allée", "voie", "cour", "place",
  "promenade", "sentier", "terrasse", "lieu",
  "riv.", "mt", "ste", "st", "rte", "av."
    ]);
    const directions = [
      'N', 'S', 'E', 'W',
      'NE', 'NW', 'SE', 'SW',
      'NNE', 'ENE', 'ESE', 'SSE',
      'SSW', 'WSW', 'WNW', 'NNW'
    ];
    const regex = new RegExp(`\\b(${directions.join('|')})\\b`, 'gi');

    return titleCase(str.toLowerCase(), { smallWords }).replace(regex, match => match.toUpperCase());

  }

  // Title Casing Undo state tracking
  let titleCasedStates = $state({
    country: { original: "", titleCased: "" },
    stateProvince: { original: "", titleCased: "" },
    county: { original: "", titleCased: "" },
    municipality: { original: "", titleCased: "" },
    locality: { original: "", titleCased: "" },
    locationNotes: { original: "", titleCased: "" },
    habitat: { original: "", titleCased: "" },
    fieldNotes: { original: "", titleCased: "" },
    occurrenceRemarks: { original: "", titleCased: "" }
  });

  function clearTitleCasedStates() {
    titleCasedStates = {
      country: { original: "", titleCased: "" },
      stateProvince: { original: "", titleCased: "" },
      county: { original: "", titleCased: "" },
      municipality: { original: "", titleCased: "" },
      locality: { original: "", titleCased: "" },
      locationNotes: { original: "", titleCased: "" },
      habitat: { original: "", titleCased: "" },
      fieldNotes: { original: "", titleCased: "" },
      occurrenceRemarks: { original: "", titleCased: "" }
    };
  }

  function undoTitleCaseField(/** @type {string} */ field) {
    let formObj = /** @type {any} */ (form);
    let stateObj = /** @type {any} */ (titleCasedStates)[field];
    if (stateObj && stateObj.original !== undefined) {
      formObj[field] = stateObj.original;
      /** @type {any} */ (titleCasedStates)[field] = { original: "", titleCased: "" };
    }
  }

  function titleCaseField(/** @type {string} */ field) {
    let formObj = /** @type {any} */ (form);
    let val = formObj[field];
    if (typeof val === "string") {
      const originalValue = val;
      const titleCasedValue = totitleCase(val);
      /** @type {any} */ (titleCasedStates)[field] = {
        original: originalValue,
        titleCased: titleCasedValue
      };
      formObj[field] = titleCasedValue;
    }
  }

  function isInitials(/** @type {string} */ str) {
    const tokens = str.split(/[\s\.]+/).filter(Boolean);
    if (tokens.length === 0) return false;
    return tokens.every(t => t.length === 1);
  }

  function splitNames(/** @type {string} */ rawStr) {
    if (!rawStr) return [];
    let trimmed = rawStr.trim();
    if (!trimmed) return [];

    let parts = [];
    if (trimmed.includes("|")) {
      parts = trimmed.split("|");
    } else if (trimmed.includes(";")) {
      parts = trimmed.split(";");
    } else if (trimmed.includes(",")) {
      const commaCount = (trimmed.match(/,/g) || []).length;
      if (commaCount === 1) {
        const tempParts = trimmed.split(",");
        const partAfter = tempParts[1].trim();
        if (isInitials(partAfter)) {
          parts = [trimmed];
        } else {
          parts = tempParts;
        }
      } else {
        parts = trimmed.split(",");
      }
    } else {
      parts = [trimmed];
    }

    return parts.map(p => p.trim()).filter(Boolean);
  }

  async function handleSave(/** @type {any} */ e) {
    if (e) e.preventDefault();
    if (!sessionId) {
      statusMessage = "Error: No active capture session selected.";
      statusType = "error";
      return;
    }
    
    saving = true;
    statusMessage = "";
    
    let primaryCollector = form.recordedBy.trim();
    let additionalCollectorsList = form.additionalCollectors.map(s => s.trim()).filter(Boolean);
    let combinedRecordedBy = primaryCollector;
    if (additionalCollectorsList.length > 0) {
      const joinedAdd = additionalCollectorsList.join("; ");
      if (combinedRecordedBy) {
        combinedRecordedBy += "; " + joinedAdd;
      } else {
        combinedRecordedBy = joinedAdd;
      }
    }

    let identifiedByList = form.identifiedBy.map(s => s.trim()).filter(Boolean);
    let combinedIdentifiedBy = identifiedByList.join("; ");

    let recordPayload = {
      ...form,
      recordedBy: combinedRecordedBy,
      identifiedBy: combinedIdentifiedBy,
      sessionId: sessionId,
      duplicates: form.duplicates.trim().replace(/,\s*$/, "").split(",").map(p => p.trim()).filter(Boolean).length || null, // Convert string duplicates list to number of duplicates for DB
      year: form.year !== "" ? parseInt(form.year) : null,
      month: form.month !== "" ? parseInt(form.month) : null,
      day: form.day !== "" ? parseInt(form.day) : null,
      yearIdentified: form.yearIdentified !== "" ? parseInt(form.yearIdentified) : null,
      monthIdentified: form.monthIdentified !== "" ? parseInt(form.monthIdentified) : null,
      dayIdentified: form.dayIdentified !== "" ? parseInt(form.dayIdentified) : null
    };
    
    try {
      let res = /** @type {any} */ (await invoke("save_captured_record", { record: recordPayload }));
      if (res.success) {
        statusMessage = form.id ? "Record updated successfully!" : "Specimen saved successfully!";
        statusType = "success";
        
        // If it was a new record, update the local ID so subsequent saves edit it instead of inserting duplicates!
        if (!form.id) {
          form.id = res.id;
        }
        lastSavedRecord = {
          ...recordPayload,
          id: res.id
        };
        
        onSaveSuccess();
        
        // Hide success message after 3 seconds
        setTimeout(() => {
          if (statusType === "success") {
            statusMessage = "";
          }
        }, 3000);
      }
    } catch (err) {
      statusMessage = `Error: ${(/** @type {any} */ (err)).toString()}`;
      statusType = "error";
    } finally {
      saving = false;
    }
  }

  function handleReset() {
    form = {
      id: null,
      collectionCode: collectionCode || "WIOI",
      catalogNumber: "",
      duplicates: "",
      recordedBy: "",
      additionalCollectors: [],
      recordNumber: "",
      verbatimEventDate: "",
      year: "",
      month: "",
      day: "",
      country: "",
      stateProvince: "",
      county: "",
      municipality: "",
      locality: "",
      verbatimCoordinates: "",
      decimalLatitude: "",
      decimalLongitude: "",
      locationNotes: "",
      verbatimLocality: "",
      verbatimElevation: "",
      habitat: "",
      identificationQualifier: "",
      scientificName: "",
      taxonID: "",
      typeStatus: "",
      identifiedBy: [],
      yearIdentified: "",
      monthIdentified: "",
      dayIdentified: "",
      identificationRemarks: "",
      occurrenceRemarks: "",
      fieldNotes: ""
    };
    activeRecord = null;
    statusMessage = "";
    taxonSuggestions = [];
    localitySuggestions = [];
    collectorSuggestions = [];
    duplicateSuggestions = [];
    typeStatusSuggestions = [];
    clearTitleCasedStates();
  }

  function handleShowPreviousRecord() {
    if (!lastSavedRecord) return;
    
    activeRecord = lastSavedRecord;
    lastSavedRecord = null;
    statusMessage = "Loaded last saved record.";
    statusType = "success";
    
    // Clear status message after 3 seconds
    setTimeout(() => {
      if (statusMessage === "Loaded last saved record.") {
        statusMessage = "";
      }
    }, 3000);
  }

  // Keyboard shortcut listener (Ctrl+S to save)
  function handleGlobalKeyDown(/** @type {any} */ e) {
    if (e.key === "s" && e.ctrlKey) {
      e.preventDefault();
      handleSave(null);
    }
  }

  function handleCoordinatesBlur() {
    if (form.verbatimCoordinates.trim() === "") {
      coordinatesError = false;
      form.decimalLatitude = "";
      form.decimalLongitude = "";
      return;
    }

    try {
      const result = convert(form.verbatimCoordinates);
      if (result && result.decimalLatitude !== undefined && result.decimalLongitude !== undefined) {
        coordinatesError = false;
        form.decimalLatitude = String(result.decimalLatitude);
        form.decimalLongitude = String(result.decimalLongitude);
      }
    } catch (e) {
      coordinatesError = true;
    }
  }

  $effect(() => {
    window.addEventListener("keydown", handleGlobalKeyDown);
    return () => {
      window.removeEventListener("keydown", handleGlobalKeyDown);
    };
  });
</script>

<div class="flex flex-col h-full bg-white border border-slate-300">
  <!-- Header Title -->
  <div class="px-4 py-3 bg-slate-100 border-b border-slate-300 flex justify-between items-center border-box">
    <div class="flex items-center gap-2">
      <h2 class="text-sm font-bold text-slate-800 uppercase tracking-wide">
        {form.id ? "Edit Captured Specimen" : "Capture New Specimen"}
      </h2>
      {#if form.id}
        <span class="text-[9px] bg-indigo-100 text-indigo-800 font-bold uppercase tracking-wider px-1.5 py-0.5">SAVED CAPTURE</span>
      {:else}
        <span class="text-[9px] bg-emerald-100 text-emerald-800 font-bold uppercase tracking-wider px-1.5 py-0.5">NEW FORM</span>
      {/if}
    </div>
    <span class="text-[10px] text-slate-400 font-semibold uppercase">Shortcut: Ctrl+S to save</span>
  </div>

  <!-- Form Fields -->
  <form onsubmit={handleSave} class="flex-1 overflow-y-auto p-4 space-y-4">
    {#if statusMessage}
      <div class="p-3 text-xs border font-medium {statusType === 'success' ? 'bg-emerald-50 border-emerald-300 text-emerald-800' : 'bg-red-50 border-red-300 text-red-800'}">
        {statusMessage}
      </div>
    {/if}

    <!-- Row 1: Home Herbarium (read-only), catalogNumber, duplicates -->
    <div class="grid grid-cols-12 gap-3">
      <div class="col-span-3">
        <label for="capture-collectionCode" class="block text-xs font-semibold text-slate-650 uppercase tracking-wider mb-1">Home Herbarium</label>
        <input
          id="capture-collectionCode"
          type="text"
          readonly
          bind:value={form.collectionCode}
          class="w-full bg-slate-100 border border-slate-300 text-slate-500 text-sm px-3 py-2 outline-none rounded-none font-semibold"
        />
      </div>
      <div class="col-span-4">
        <label for="capture-catalogNumber" class="block text-xs font-semibold text-slate-655 uppercase tracking-wider mb-1">Barcode Number</label>
        <input
          id="capture-catalogNumber"
          type="text"
          placeholder="eg TAN123456"
          bind:value={form.catalogNumber}
          class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
        />
      </div>
      <div class="col-span-5">
        <Autocomplete
          id="capture-duplicates"
          label="Duplicates (comma-separated)"
          placeholder="eg P, K, etc..."
          bind:value={form.duplicates}
          suggestions={duplicateSuggestions}
          oninput={handleDuplicateInput}
          onselect={handleDuplicateSelect}
          onfocus={handleDuplicateFocus}
          customSelect={true}
        />
      </div>
    </div>

    <!-- Row 2: Collector/s (recordedBy), Collector Number, Verbatim Date, Year, Month, Day -->
    <div class="grid grid-cols-12 gap-3">
      <div class="col-span-3">
        <Autocomplete
          id="capture-recordedBy"
          label="Primary Collector"
          placeholder="Partial eg 'Raza'"
          bind:value={form.recordedBy}
          suggestions={collectorSuggestions}
          oninput={handleCollectorInput}
          delay={300}
        />
      </div>
      <div class="col-span-2">
        <label for="capture-recordNumber" class="block text-xs font-semibold text-slate-650 uppercase tracking-wider mb-1">Collector No.</label>
        <input
          id="capture-recordNumber"
          type="text"
          placeholder="eg 1042"
          bind:value={form.recordNumber}
          class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
        />
      </div>
      
      <div class="col-span-3">
        <label for="capture-verbatimEventDate" class="block text-xs font-semibold text-slate-650 uppercase tracking-wider mb-1">Verbatim Date</label>
        <input
          id="capture-verbatimEventDate"
          type="text"
          placeholder="eg 'May 20, `84'"
          bind:value={form.verbatimEventDate}
          onblur={parseVerbatimDate}
          class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
        />
      </div>
      <div class="col-span-4 flex gap-2">
        <div class="flex-1">
          <label for="capture-year" class="block text-xs font-semibold text-slate-650 uppercase tracking-wider mb-1">Year</label>
          <input
            id="capture-year"
            type="number"
            bind:value={form.year}
            class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-2 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
          />
        </div>
        <div class="flex-1">
          <label for="capture-month" class="block text-xs font-semibold text-slate-650 uppercase tracking-wider mb-1">Month</label>
          <input
            id="capture-month"
            type="number"
            min="1"
            max="12"
            bind:value={form.month}
            class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-2 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
          />
        </div>
        <div class="flex-1">
          <label for="capture-day" class="block text-xs font-semibold text-slate-650 uppercase tracking-wider mb-1">Day</label>
          <input
            id="capture-day"
            type="number"
            min="1"
            max="31"
            bind:value={form.day}
            class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-2 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
          />
        </div>
      </div>
    </div>

    <!-- Row 2.5: Additional Collectors -->
    <div class="grid grid-cols-12 gap-3 pt-1">
      <div class="col-span-12">
        <MultiSelectAutocomplete
          id="capture-additionalCollectors"
          label="Additional Collectors"
          placeholder="Type name and press Enter..."
          bind:selectedValues={form.additionalCollectors}
          suggestions={additionalCollectorsSuggestions}
          oninput={handleAdditionalCollectorsInput}
          delay={300}
        />
      </div>
    </div>

    <!-- Row 3: Geography with Title-case buttons -->
    <div class="space-y-3 pt-2">
      <h3 class="text-[10px] font-bold text-slate-400 uppercase tracking-wider border-b border-slate-100 pb-1">Geographic Locality</h3>
      
      <div class="grid grid-cols-4 gap-3">
        <div>
          <label for="capture-country" class="block text-xs font-semibold text-slate-650 uppercase tracking-wider mb-1">Country</label>
          <div class="relative flex items-center">
            <Autocomplete
              id="capture-country"
              label=""
              placeholder={isAnyGeoPopulated ? "" : "eg Madagascar"}
              bind:value={form.country}
              suggestions={countrySuggestions}
              oninput={handleCountryInput}
              onselect={onCountryChanged}
              delay={300}
            />
            {#if form.country === titleCasedStates.country.titleCased && titleCasedStates.country.titleCased !== ""}
              <button
                type="button"
                onclick={() => undoTitleCaseField("country")}
                title="Undo Title case"
                class="absolute right-2 top-3 text-slate-400 hover:text-slate-600 z-10 cursor-pointer flex items-center justify-center"
                tabindex="-1"
              >
                <svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256" class="w-3.5 h-3.5">
                  <path d="M232,144a64.07,64.07,0,0,1-64,64H80a8,8,0,0,1,0-16h88a48,48,0,0,0,0-96H51.31l34.35,34.34a8,8,0,0,1-11.32,11.32l-48-48a8,8,0,0,1,0-11.32l48-48A8,8,0,0,1,85.66,45.66L51.31,80H168A64.07,64.07,0,0,1,232,144Z"></path>
                </svg>
              </button>
            {:else}
              <button
                type="button"
                onclick={() => titleCaseField("country")}
                title="Title case Country"
                class="absolute right-2 top-3 text-slate-400 hover:text-slate-600 z-10 cursor-pointer flex items-center justify-center"
                tabindex="-1"
              >
                <svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256" class="w-3.5 h-3.5">
                  <path d="M87.24,52.59a8,8,0,0,0-14.48,0l-64,136a8,8,0,1,0,14.48,6.81L39.9,160h80.2l16.66,35.4a8,8,0,1,0,14.48-6.81ZM47.43,144,80,74.79,112.57,144ZM200,96c-12.76,0-22.73,3.47-29.63,10.32a8,8,0,0,0,11.26,11.36c3.8-3.77,10-5.68,18.37-5.68,13.23,0,24,9,24,20v3.22A42.76,42.76,0,0,0,200,128c-22.06,0-40,16.15-40,36s17.94,36,40,36a42.73,42.73,0,0,0,24-7.25,8,8,0,0,0,16-.75V132C240,112.15,222.06,96,200,96Zm0,88c-13.23,0-24-9-24-20s10.77-20,24-20,24,9,24,20S213.23,184,200,184Z"></path>
                </svg>
              </button>
            {/if}
          </div>
        </div>

        <div>
          <label for="capture-stateProvince" class="block text-xs font-semibold text-slate-650 uppercase tracking-wider mb-1">Admin 2 <span class="text-[70%]">(state/prov/etc)</span></label>
          <div class="relative flex items-center">
            <Autocomplete
              id="capture-stateProvince"
              label=""
              placeholder={isAnyGeoPopulated ? "" : "eg 'Itasy'"}
              bind:value={form.stateProvince}
              suggestions={stateProvinceSuggestions}
              oninput={handleStateProvinceInput}
              onselect={onStateProvinceChanged}
              delay={300}
            />
            {#if form.stateProvince === titleCasedStates.stateProvince.titleCased && titleCasedStates.stateProvince.titleCased !== ""}
              <button
                type="button"
                onclick={() => undoTitleCaseField("stateProvince")}
                title="Undo Title case"
                class="absolute right-2 top-3 text-slate-400 hover:text-slate-600 z-10 cursor-pointer flex items-center justify-center"
                tabindex="-1"
              >
                <svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256" class="w-3.5 h-3.5">
                  <path d="M232,144a64.07,64.07,0,0,1-64,64H80a8,8,0,0,1,0-16h88a48,48,0,0,0,0-96H51.31l34.35,34.34a8,8,0,0,1-11.32,11.32l-48-48a8,8,0,0,1,0-11.32l48-48A8,8,0,0,1,85.66,45.66L51.31,80H168A64.07,64.07,0,0,1,232,144Z"></path>
                </svg>
              </button>
            {:else}
              <button
                type="button"
                onclick={() => titleCaseField("stateProvince")}
                title="Title case Admin 2"
                class="absolute right-2 top-3 text-slate-400 hover:text-slate-600 z-10 cursor-pointer flex items-center justify-center"
                tabindex="-1"
              >
                <svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256" class="w-3.5 h-3.5">
                  <path d="M87.24,52.59a8,8,0,0,0-14.48,0l-64,136a8,8,0,1,0,14.48,6.81L39.9,160h80.2l16.66,35.4a8,8,0,1,0,14.48-6.81ZM47.43,144,80,74.79,112.57,144ZM200,96c-12.76,0-22.73,3.47-29.63,10.32a8,8,0,0,0,11.26,11.36c3.8-3.77,10-5.68,18.37-5.68,13.23,0,24,9,24,20v3.22A42.76,42.76,0,0,0,200,128c-22.06,0-40,16.15-40,36s17.94,36,40,36a42.73,42.73,0,0,0,24-7.25,8,8,0,0,0,16-.75V132C240,112.15,222.06,96,200,96Zm0,88c-13.23,0-24-9-24-20s10.77-20,24-20,24,9,24,20S213.23,184,200,184Z"></path>
                </svg>
              </button>
            {/if}
          </div>
        </div>

        <div>
          <label for="capture-county" class="block text-xs font-semibold text-slate-650 uppercase tracking-wider mb-1">Admin 3</label>
          <div class="relative flex items-center">
            <Autocomplete
              id="capture-county"
              label=""
              placeholder={isAnyGeoPopulated ? "" : "eg 'Miarinarivo'"}
              bind:value={form.county}
              suggestions={countySuggestions}
              oninput={handleCountyInput}
              onselect={onCountyChanged}
              delay={300}
            />
            {#if form.county === titleCasedStates.county.titleCased && titleCasedStates.county.titleCased !== ""}
              <button
                type="button"
                onclick={() => undoTitleCaseField("county")}
                title="Undo Title case"
                class="absolute right-2 top-3 text-slate-400 hover:text-slate-600 z-10 cursor-pointer flex items-center justify-center"
                tabindex="-1"
              >
                <svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256" class="w-3.5 h-3.5">
                  <path d="M232,144a64.07,64.07,0,0,1-64,64H80a8,8,0,0,1,0-16h88a48,48,0,0,0,0-96H51.31l34.35,34.34a8,8,0,0,1-11.32,11.32l-48-48a8,8,0,0,1,0-11.32l48-48A8,8,0,0,1,85.66,45.66L51.31,80H168A64.07,64.07,0,0,1,232,144Z"></path>
                </svg>
              </button>
            {:else}
              <button
                type="button"
                onclick={() => titleCaseField("county")}
                title="Title case Admin 3"
                class="absolute right-2 top-3 text-slate-400 hover:text-slate-600 z-10 cursor-pointer flex items-center justify-center"
                tabindex="-1"
              >
                <svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256" class="w-3.5 h-3.5">
                  <path d="M87.24,52.59a8,8,0,0,0-14.48,0l-64,136a8,8,0,1,0,14.48,6.81L39.9,160h80.2l16.66,35.4a8,8,0,1,0,14.48-6.81ZM47.43,144,80,74.79,112.57,144ZM200,96c-12.76,0-22.73,3.47-29.63,10.32a8,8,0,0,0,11.26,11.36c3.8-3.77,10-5.68,18.37-5.68,13.23,0,24,9,24,20v3.22A42.76,42.76,0,0,0,200,128c-22.06,0-40,16.15-40,36s17.94,36,40,36a42.73,42.73,0,0,0,24-7.25,8,8,0,0,0,16-.75V132C240,112.15,222.06,96,200,96Zm0,88c-13.23,0-24-9-24-20s10.77-20,24-20,24,9,24,20S213.23,184,200,184Z"></path>
                </svg>
              </button>
            {/if}
          </div>
        </div>

        <div>
          <label for="capture-municipality" class="block text-xs font-semibold text-slate-655 uppercase tracking-wider mb-1">Admin 4</label>
          <div class="relative flex items-center">
            <Autocomplete
              id="capture-municipality"
              label=""
              placeholder={isAnyGeoPopulated ? "" : "eg 'Manazary'"}
              bind:value={form.municipality}
              suggestions={municipalitySuggestions}
              oninput={handleMunicipalityInput}
              delay={300}
            />
            {#if form.municipality === titleCasedStates.municipality.titleCased && titleCasedStates.municipality.titleCased !== ""}
              <button
                type="button"
                onclick={() => undoTitleCaseField("municipality")}
                title="Undo Title case"
                class="absolute right-2 top-3 text-slate-400 hover:text-slate-600 z-10 cursor-pointer flex items-center justify-center"
                tabindex="-1"
              >
                <svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256" class="w-3.5 h-3.5">
                  <path d="M232,144a64.07,64.07,0,0,1-64,64H80a8,8,0,0,1,0-16h88a48,48,0,0,0,0-96H51.31l34.35,34.34a8,8,0,0,1-11.32,11.32l-48-48a8,8,0,0,1,0-11.32l48-48A8,8,0,0,1,85.66,45.66L51.31,80H168A64.07,64.07,0,0,1,232,144Z"></path>
                </svg>
              </button>
            {:else}
              <button
                type="button"
                onclick={() => titleCaseField("municipality")}
                title="Title case Admin 4"
                class="absolute right-2 top-3 text-slate-400 hover:text-slate-600 z-10 cursor-pointer flex items-center justify-center"
                tabindex="-1"
              >
                <svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256" class="w-3.5 h-3.5">
                  <path d="M87.24,52.59a8,8,0,0,0-14.48,0l-64,136a8,8,0,1,0,14.48,6.81L39.9,160h80.2l16.66,35.4a8,8,0,1,0,14.48-6.81ZM47.43,144,80,74.79,112.57,144ZM200,96c-12.76,0-22.73,3.47-29.63,10.32a8,8,0,0,0,11.26,11.36c3.8-3.77,10-5.68,18.37-5.68,13.23,0,24,9,24,20v3.22A42.76,42.76,0,0,0,200,128c-22.06,0-40,16.15-40,36s17.94,36,40,36a42.73,42.73,0,0,0,24-7.25,8,8,0,0,0,16-.75V132C240,112.15,222.06,96,200,96Zm0,88c-13.23,0-24-9-24-20s10.77-20,24-20,24,9,24,20S213.23,184,200,184Z"></path>
                </svg>
              </button>
            {/if}
          </div>
        </div>
      </div>
    </div>

    <!-- Row 4: Locality (on its own row) & Verbatim Coordinates (on its own row) -->
    <div class="space-y-3">
      <div>
        <label for="capture-locality" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">Locality</label>
        <div class="relative flex items-center w-full">
          <Autocomplete
            id="capture-locality"
            label=""
            placeholder="eg 'Antakohandro' (use partial search eg 'Anta')"
            bind:value={form.locality}
            suggestions={localitySuggestions}
            oninput={handleLocalityInput}
            bind:inputRef={localityInputRef}
            extraInputClass="pr-24"
            delay={300}
          />
          <div class="absolute right-2 top-2 flex items-center gap-1 z-10 bg-white pl-1">
            {#if form.locality === titleCasedStates.locality.titleCased && titleCasedStates.locality.titleCased !== ""}
              <button
                type="button"
                onclick={() => undoTitleCaseField("locality")}
                title="Undo Title case"
                class="px-1.5 py-1 text-slate-400 hover:text-slate-600 transition-colors cursor-pointer rounded-none flex items-center justify-center"
                tabindex="-1"
              >
                <svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256" class="w-3.5 h-3.5">
                  <path d="M232,144a64.07,64.07,0,0,1-64,64H80a8,8,0,0,1,0-16h88a48,48,0,0,0,0-96H51.31l34.35,34.34a8,8,0,0,1-11.32,11.32l-48-48a8,8,0,0,1,0-11.32l48-48A8,8,0,0,1,85.66,45.66L51.31,80H168A64.07,64.07,0,0,1,232,144Z"></path>
                </svg>
              </button>
            {:else}
              <button
                type="button"
                onclick={() => titleCaseField("locality")}
                title="Title case Locality"
                class="px-1.5 py-1 text-slate-400 hover:text-slate-600 transition-colors cursor-pointer rounded-none flex items-center justify-center"
                tabindex="-1"
              >
                <svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256" class="w-3.5 h-3.5">
                  <path d="M87.24,52.59a8,8,0,0,0-14.48,0l-64,136a8,8,0,1,0,14.48,6.81L39.9,160h80.2l16.66,35.4a8,8,0,1,0,14.48-6.81ZM47.43,144,80,74.79,112.57,144ZM200,96c-12.76,0-22.73,3.47-29.63,10.32a8,8,0,0,0,11.26,11.36c3.8-3.77,10-5.68,18.37-5.68,13.23,0,24,9,24,20v3.22A42.76,42.76,0,0,0,200,128c-22.06,0-40,16.15-40,36s17.94,36,40,36a42.73,42.73,0,0,0,24-7.25,8,8,0,0,0,16-.75V132C240,112.15,222.06,96,200,96Zm0,88c-13.23,0-24-9-24-20s10.77-20,24-20,24,9,24,20S213.23,184,200,184Z"></path>
                </svg>
              </button>
            {/if}
            <button
              type="button"
              onclick={handleCopyLocality}
              title={localityCopied ? "Copied!" : "Copy selection or entire text"}
              class="p-1 transition-colors cursor-pointer rounded-none flex items-center justify-center {localityCopied ? 'text-green-600' : 'text-slate-400 hover:text-slate-600'}"
              tabindex="-1"
            >
              {#if localityCopied}
                <svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256" class="w-3.5 h-3.5">
                  <path d="M229.66,77.66l-128,128a8,8,0,0,1-11.32,0l-56-56a8,8,0,0,1,11.32-11.32L96,188.69,218.34,66.34a8,8,0,0,1,11.32,11.32Z"></path>
                </svg>
              {:else}
                <svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256" class="w-3.5 h-3.5">
                  <path d="M184,64H40a8,8,0,0,0-8,8V216a8,8,0,0,0,8,8H184a8,8,0,0,0,8-8V72A8,8,0,0,0,184,64Zm-8,144H48V80H176ZM224,40V184a8,8,0,0,1-16,0V48H72a8,8,0,0,1,0-16H216A8,8,0,0,1,224,40Z"></path>
                </svg>
              {/if}
            </button>
            <button
              type="button"
              onclick={handlePasteLocality}
              title="Paste clipboard contents"
              class="p-1 text-slate-400 hover:text-slate-600 transition-colors cursor-pointer rounded-none flex items-center justify-center"
              tabindex="-1"
            >
              <svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256" class="w-3.5 h-3.5">
                <path d="M168,152a8,8,0,0,1-8,8H96a8,8,0,0,1,0-16h64A8,8,0,0,1,168,152Zm-8-40H96a8,8,0,0,0,0,16h64a8,8,0,0,0,0-16Zm56-64V216a16,16,0,0,1-16,16H56a16,16,0,0,1-16-16V48A16,16,0,0,1,56,32H92.26a47.92,47.92,0,0,1,71.48,0H200A16,16,0,0,1,216,48ZM96,64h64a32,32,0,0,0-64,0ZM200,48H173.25A47.93,47.93,0,0,1,176,64v8a8,8,0,0,1-8,8H88a8,8,0,0,1-8-8V64a47.93,47.93,0,0,1,2.75-16H56V216H200Z"></path>
              </svg>
            </button>
          </div>
        </div>
      </div>

      <!-- Row 5: Locality Notes (locationNotes) -->
      <div>
        <label for="capture-locationNotes" class="block text-xs font-semibold text-slate-650 uppercase tracking-wider mb-1">Locality Notes</label>
        <div class="relative flex items-start w-full">
          <textarea
            bind:this={locationNotesRef}
            id="capture-locationNotes"
            rows="2"
            placeholder="eg '12 km south, main ravine'"
            bind:value={form.locationNotes}
            class="w-full bg-white border border-slate-300 text-slate-800 text-sm pl-3 pr-24 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all resize-none"
          ></textarea>
          <div class="absolute right-2 top-2 flex items-center gap-1 z-10 bg-white pl-1">
            {#if form.locationNotes === titleCasedStates.locationNotes.titleCased && titleCasedStates.locationNotes.titleCased !== ""}
              <button
                type="button"
                onclick={() => undoTitleCaseField("locationNotes")}
                title="Undo Title case"
                class="px-1.5 py-1 text-slate-400 hover:text-slate-600 transition-colors cursor-pointer rounded-none flex items-center justify-center"
                tabindex="-1"
              >
                <svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256" class="w-3.5 h-3.5">
                  <path d="M232,144a64.07,64.07,0,0,1-64,64H80a8,8,0,0,1,0-16h88a48,48,0,0,0,0-96H51.31l34.35,34.34a8,8,0,0,1-11.32,11.32l-48-48a8,8,0,0,1,0-11.32l48-48A8,8,0,0,1,85.66,45.66L51.31,80H168A64.07,64.07,0,0,1,232,144Z"></path>
                </svg>
              </button>
            {:else}
              <button
                type="button"
                onclick={() => titleCaseField("locationNotes")}
                title="Title case Locality Notes"
                class="px-1.5 py-1 text-slate-400 hover:text-slate-600 transition-colors cursor-pointer rounded-none flex items-center justify-center"
                tabindex="-1"
              >
                <svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256" class="w-3.5 h-3.5">
                  <path d="M87.24,52.59a8,8,0,0,0-14.48,0l-64,136a8,8,0,1,0,14.48,6.81L39.9,160h80.2l16.66,35.4a8,8,0,1,0,14.48-6.81ZM47.43,144,80,74.79,112.57,144ZM200,96c-12.76,0-22.73,3.47-29.63,10.32a8,8,0,0,0,11.26,11.36c3.8-3.77,10-5.68,18.37-5.68,13.23,0,24,9,24,20v3.22A42.76,42.76,0,0,0,200,128c-22.06,0-40,16.15-40,36s17.94,36,40,36a42.73,42.73,0,0,0,24-7.25,8,8,0,0,0,16-.75V132C240,112.15,222.06,96,200,96Zm0,88c-13.23,0-24-9-24-20s10.77-20,24-20,24,9,24,20S213.23,184,200,184Z"></path>
                </svg>
              </button>
            {/if}
            <button
              type="button"
              onclick={handleCopyLocationNotes}
              title={locationNotesCopied ? "Copied!" : "Copy selection or entire text"}
              class="p-1 transition-colors cursor-pointer rounded-none flex items-center justify-center {locationNotesCopied ? 'text-green-600' : 'text-slate-400 hover:text-slate-600'}"
              tabindex="-1"
            >
              {#if locationNotesCopied}
                <svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256" class="w-3.5 h-3.5">
                  <path d="M229.66,77.66l-128,128a8,8,0,0,1-11.32,0l-56-56a8,8,0,0,1,11.32-11.32L96,188.69,218.34,66.34a8,8,0,0,1,11.32,11.32Z"></path>
                </svg>
              {:else}
                <svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256" class="w-3.5 h-3.5">
                  <path d="M184,64H40a8,8,0,0,0-8,8V216a8,8,0,0,0,8,8H184a8,8,0,0,0,8-8V72A8,8,0,0,0,184,64Zm-8,144H48V80H176ZM224,40V184a8,8,0,0,1-16,0V48H72a8,8,0,0,1,0-16H216A8,8,0,0,1,224,40Z"></path>
                </svg>
              {/if}
            </button>
            <button
              type="button"
              onclick={handlePasteLocationNotes}
              title="Paste clipboard contents"
              class="p-1 text-slate-400 hover:text-slate-600 transition-colors cursor-pointer rounded-none flex items-center justify-center"
              tabindex="-1"
            >
              <svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256" class="w-3.5 h-3.5">
                <path d="M168,152a8,8,0,0,1-8,8H96a8,8,0,0,1,0-16h64A8,8,0,0,1,168,152Zm-8-40H96a8,8,0,0,0,0,16h64a8,8,0,0,0,0-16Zm56-64V216a16,16,0,0,1-16,16H56a16,16,0,0,1-16-16V48A16,16,0,0,1,56,32H92.26a47.92,47.92,0,0,1,71.48,0H200A16,16,0,0,1,216,48ZM96,64h64a32,32,0,0,0-64,0ZM200,48H173.25A47.93,47.93,0,0,1,176,64v8a8,8,0,0,1-8,8H88a8,8,0,0,1-8-8V64a47.93,47.93,0,0,1,2.75-16H56V216H200Z"></path>
              </svg>
            </button>
          </div>
        </div>
      </div>

      <div>
        <label for="capture-verbatimCoordinates" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">Verbatim Coordinates</label>
        <input
          id="capture-verbatimCoordinates"
          type="text"
          placeholder="eg 28°15'S, 28°39'E"
          bind:value={form.verbatimCoordinates}
          onblur={handleCoordinatesBlur}
          class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
        />
      </div>
      {#if coordinatesError}
        <div class="mt-3 text-xs bg-amber-50 border-l-2 border-amber-500 text-amber-700 px-3 py-2 font-medium">
          ⚠️ Unable to parse coordinates, please check they are correct
        </div>
      {/if}
    </div>

    

    <!-- Row 6: Verbatim Locality (grayed-out read-only) -->
    <div>
      <label for="capture-verbatimLocality" class="block text-xs font-semibold text-slate-650 uppercase tracking-wider mb-1">Verbatim Locality (copy data above)</label>
      <div class="relative">
        <textarea
          bind:this={verbatimLocalityRef}
          id="capture-verbatimLocality"
          rows="2"
          readonly
          placeholder="Read-only imported value"
          bind:value={form.verbatimLocality}
          class="w-full bg-slate-100 border border-slate-300 text-slate-500 text-sm px-3 py-2 outline-none rounded-none pr-10"      >
        </textarea>
        <button
          type="button"
          onclick={handleCopyVerbatimLocality}
          title={verbatimLocalityCopied ? "Copied!" : "Copy selection or entire text"}
          class="absolute right-2 top-2 p-1.5 bg-white border transition-colors cursor-pointer rounded-none flex items-center justify-center {verbatimLocalityCopied ? 'border-green-300 text-green-600 hover:text-green-600 hover:border-green-300 shadow-xs' : 'border-slate-200 text-slate-400 hover:text-slate-600 hover:border-slate-350 shadow-xs'}"
          tabindex="-1"
        >
          {#if verbatimLocalityCopied}
            <svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256" class="w-3.5 h-3.5">
              <path d="M229.66,77.66l-128,128a8,8,0,0,1-11.32,0l-56-56a8,8,0,0,1,11.32-11.32L96,188.69,218.34,66.34a8,8,0,0,1,11.32,11.32Z"></path>
            </svg>
          {:else}
            <svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256" class="w-3.5 h-3.5">
              <path d="M184,64H40a8,8,0,0,0-8,8V216a8,8,0,0,0,8,8H184a8,8,0,0,0,8-8V72A8,8,0,0,0,184,64Zm-8,144H48V80H176ZM224,40V184a8,8,0,0,1-16,0V48H72a8,8,0,0,1,0-16H216A8,8,0,0,1,224,40Z"></path>
            </svg>
          {/if}
        </button>
      </div>
    </div>

    <!-- Row 6b: Verbatim Elevation & Habitat -->
    <div class="flex gap-3">
      <div class="w-1/4">
        <label for="capture-verbatimElevation" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">Verbatim Elevation</label>
        <input
          id="capture-verbatimElevation"
          type="text"
          placeholder="eg 1200m"
          bind:value={form.verbatimElevation}
          class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
        />
      </div>
      <div class="flex-1">
        <label for="capture-habitat" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">Habitat</label>
        <div class="relative flex items-center">
          <input
            id="capture-habitat"
            type="text"
            placeholder="eg Oak woodland, sandy soil..."
            bind:value={form.habitat}
            class="w-full bg-white border border-slate-300 text-slate-800 text-sm pl-3 pr-8 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
          />
          {#if form.habitat === titleCasedStates.habitat.titleCased && titleCasedStates.habitat.titleCased !== ""}
            <button
              type="button"
              onclick={() => undoTitleCaseField("habitat")}
              title="Undo Title case"
              class="absolute right-2 top-3 text-slate-400 hover:text-slate-600 z-10 cursor-pointer flex items-center justify-center"
              tabindex="-1"
            >
              <svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256" class="w-3.5 h-3.5">
                <path d="M232,144a64.07,64.07,0,0,1-64,64H80a8,8,0,0,1,0-16h88a48,48,0,0,0,0-96H51.31l34.35,34.34a8,8,0,0,1-11.32,11.32l-48-48a8,8,0,0,1,0-11.32l48-48A8,8,0,0,1,85.66,45.66L51.31,80H168A64.07,64.07,0,0,1,232,144Z"></path>
              </svg>
            </button>
          {:else}
            <button
              type="button"
              onclick={() => titleCaseField("habitat")}
              title="Title case Habitat"
              class="absolute right-2 top-3 text-slate-400 hover:text-slate-600 z-10 cursor-pointer flex items-center justify-center"
              tabindex="-1"
            >
              <svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256" class="w-3.5 h-3.5">
                <path d="M87.24,52.59a8,8,0,0,0-14.48,0l-64,136a8,8,0,1,0,14.48,6.81L39.9,160h80.2l16.66,35.4a8,8,0,1,0,14.48-6.81ZM47.43,144,80,74.79,112.57,144ZM200,96c-12.76,0-22.73,3.47-29.63,10.32a8,8,0,0,0,11.26,11.36c3.8-3.77,10-5.68,18.37-5.68,13.23,0,24,9,24,20v3.22A42.76,42.76,0,0,0,200,128c-22.06,0-40,16.15-40,36s17.94,36,40,36a42.73,42.73,0,0,0,24-7.25,8,8,0,0,0,16-.75V132C240,112.15,222.06,96,200,96Zm0,88c-13.23,0-24-9-24-20s10.77-20,24-20,24,9,24,20S213.23,184,200,184Z"></path>
              </svg>
            </button>
          {/if}
        </div>
      </div>
    </div>

    <!-- Plant description Section -->
    <div class="space-y-3 pt-2">
      <h3 class="text-[10px] font-bold text-slate-400 uppercase tracking-wider border-b border-slate-100 pb-1">Plant description</h3>
      <div class="relative flex items-start">
        <textarea
          id="capture-fieldNotes"
          rows="2"
          placeholder="eg flower yellow, tree 5m"
          bind:value={form.fieldNotes}
          class="w-full bg-white border border-slate-300 text-slate-800 text-sm pl-3 pr-8 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all resize-none"
        ></textarea>
        {#if form.fieldNotes === titleCasedStates.fieldNotes.titleCased && titleCasedStates.fieldNotes.titleCased !== ""}
          <button
            type="button"
            onclick={() => undoTitleCaseField("fieldNotes")}
            title="Undo Title case"
            class="absolute right-2 bottom-3 text-slate-400 hover:text-slate-600 z-10 cursor-pointer flex items-center justify-center"
            tabindex="-1"
          >
            <svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256" class="w-3.5 h-3.5">
              <path d="M232,144a64.07,64.07,0,0,1-64,64H80a8,8,0,0,1,0-16h88a48,48,0,0,0,0-96H51.31l34.35,34.34a8,8,0,0,1-11.32,11.32l-48-48a8,8,0,0,1,0-11.32l48-48A8,8,0,0,1,85.66,45.66L51.31,80H168A64.07,64.07,0,0,1,232,144Z"></path>
            </svg>
          </button>
        {:else}
          <button
            type="button"
            onclick={() => titleCaseField("fieldNotes")}
            title="Title case Plant description"
            class="absolute right-2 bottom-3 text-slate-400 hover:text-slate-600 z-10 cursor-pointer flex items-center justify-center"
            tabindex="-1"
          >
            <svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256" class="w-3.5 h-3.5">
              <path d="M87.24,52.59a8,8,0,0,0-14.48,0l-64,136a8,8,0,1,0,14.48,6.81L39.9,160h80.2l16.66,35.4a8,8,0,1,0,14.48-6.81ZM47.43,144,80,74.79,112.57,144ZM200,96c-12.76,0-22.73,3.47-29.63,10.32a8,8,0,0,0,11.26,11.36c3.8-3.77,10-5.68,18.37-5.68,13.23,0,24,9,24,20v3.22A42.76,42.76,0,0,0,200,128c-22.06,0-40,16.15-40,36s17.94,36,40,36a42.73,42.73,0,0,0,24-7.25,8,8,0,0,0,16-.75V132C240,112.15,222.06,96,200,96Zm0,88c-13.23,0-24-9-24-20s10.77-20,24-20,24,9,24,20S213.23,184,200,184Z"></path>
            </svg>
          </button>
        {/if}
      </div>
    </div>

    <!-- General Notes Section -->
    <div class="space-y-3 pt-2">
      <h3 class="text-[10px] font-bold text-slate-400 uppercase tracking-wider border-b border-slate-100 pb-1">General Notes</h3>
      <div class="relative flex items-start">
        <textarea
          id="capture-occurrenceRemarks"
          rows="2"
          placeholder="Any other notes..."
          bind:value={form.occurrenceRemarks}
          class="w-full bg-white border border-slate-300 text-slate-800 text-sm pl-3 pr-8 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all resize-none"
        ></textarea>
        {#if form.occurrenceRemarks === titleCasedStates.occurrenceRemarks.titleCased && titleCasedStates.occurrenceRemarks.titleCased !== ""}
          <button
            type="button"
            onclick={() => undoTitleCaseField("occurrenceRemarks")}
            title="Undo Title case"
            class="absolute right-2 bottom-3 text-slate-400 hover:text-slate-600 z-10 cursor-pointer flex items-center justify-center"
            tabindex="-1"
          >
            <svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256" class="w-3.5 h-3.5">
              <path d="M232,144a64.07,64.07,0,0,1-64,64H80a8,8,0,0,1,0-16h88a48,48,0,0,0,0-96H51.31l34.35,34.34a8,8,0,0,1-11.32,11.32l-48-48a8,8,0,0,1,0-11.32l48-48A8,8,0,0,1,85.66,45.66L51.31,80H168A64.07,64.07,0,0,1,232,144Z"></path>
            </svg>
          </button>
        {:else}
          <button
            type="button"
            onclick={() => titleCaseField("occurrenceRemarks")}
            title="Title case General Notes"
            class="absolute right-2 bottom-3 text-slate-400 hover:text-slate-600 z-10 cursor-pointer flex items-center justify-center"
            tabindex="-1"
          >
            <svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256" class="w-3.5 h-3.5">
              <path d="M87.24,52.59a8,8,0,0,0-14.48,0l-64,136a8,8,0,1,0,14.48,6.81L39.9,160h80.2l16.66,35.4a8,8,0,1,0,14.48-6.81ZM47.43,144,80,74.79,112.57,144ZM200,96c-12.76,0-22.73,3.47-29.63,10.32a8,8,0,0,0,11.26,11.36c3.8-3.77,10-5.68,18.37-5.68,13.23,0,24,9,24,20v3.22A42.76,42.76,0,0,0,200,128c-22.06,0-40,16.15-40,36s17.94,36,40,36a42.73,42.73,0,0,0,24-7.25,8,8,0,0,0,16-.75V132C240,112.15,222.06,96,200,96Zm0,88c-13.23,0-24-9-24-20s10.77-20,24-20,24,9,24,20S213.23,184,200,184Z"></path>
            </svg>
          </button>
        {/if}
      </div>
    </div>

    <!-- Row 7: Identification Section -->
    <div class="space-y-3 pt-2">
      <h3 class="text-[10px] font-bold text-slate-400 uppercase tracking-wider border-b border-slate-100 pb-1">Identification</h3>
      
      <div class="grid grid-cols-12 gap-3">
        <!-- Qualifier dropdown -->
        <div class="col-span-3">
          <label for="capture-identificationQualifier" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">Qualifier</label>
          <select
            id="capture-identificationQualifier"
            bind:value={form.identificationQualifier}
            class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-2 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
          >
            <option value="">(None)</option>
            <option value="cf.">cf.</option>
            <option value="aff.">aff.</option>
            <option value="nr.">nr.</option>
          </select>
        </div>

        <!-- Scientific Name autocomplete targeting wcvp -->
        <div class="col-span-6">
          <Autocomplete
            id="capture-scientificName"
            label="Scientific Name"
            placeholder="Partial search eg 'ab man'"
            bind:value={form.scientificName}
            suggestions={taxonSuggestions}
            oninput={handleTaxonInput}
            onselect={handleTaxonSelect}
            delay={300}
          />
        </div>

        <!-- Type Status autocomplete -->
        <div class="col-span-3">
          <label for="capture-typeStatus" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">Type Status</label>
          <Autocomplete
            id="capture-typeStatus"
            label=""
            placeholder="eg holotype"
            bind:value={form.typeStatus}
            suggestions={typeStatusSuggestions}
            oninput={handleTypeStatusInput}
            onfocus={handleTypeStatusFocus}
            delay={0}
          />
        </div>
      </div>
    </div>

    <!-- Row 8: Identified By, Year, Month, Day Identified -->
    <div class="grid grid-cols-12 gap-3">
      <div class="col-span-6">
        <MultiSelectAutocomplete
          id="capture-identifiedBy"
          label="Identified By"
          placeholder="Type name and press Enter..."
          bind:selectedValues={form.identifiedBy}
          suggestions={identifiedBySuggestions}
          oninput={handleIdentifiedByInput}
          delay={300}
        />
      </div>
      <div class="col-span-2">
        <label for="capture-yearIdentified" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">Year Ident.</label>
        <input
          id="capture-yearIdentified"
          type="number"
          placeholder="YYYY"
          bind:value={form.yearIdentified}
          class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-2 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
        />
      </div>
      <div class="col-span-2">
        <label for="capture-monthIdentified" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">Month Ident.</label>
        <input
          id="capture-monthIdentified"
          type="number"
          placeholder="MM"
          min="1"
          max="12"
          bind:value={form.monthIdentified}
          class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-2 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
        />
      </div>
      <div class="col-span-2">
        <label for="capture-dayIdentified" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">Day Ident.</label>
        <input
          id="capture-dayIdentified"
          type="number"
          placeholder="DD"
          min="1"
          max="31"
          bind:value={form.dayIdentified}
          class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-2 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
        />
      </div>
    </div>

    <!-- Row 9: Identification Notes -->
    <div class="pb-6">
      <label for="capture-identificationRemarks" class="block text-xs font-semibold text-slate-650 uppercase tracking-wider mb-1">Identification Notes</label>
      <textarea
        id="capture-identificationRemarks"
        rows="2"
        placeholder="e.g. Similar to [species name] but has different bract structures"
        bind:value={form.identificationRemarks}
        class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all resize-none"
      ></textarea>
    </div>
  </form>

  <!-- Save Action Footer -->
  <div class="p-4 border-t flex justify-between  border-slate-300 bg-slate-50 ">
    <button
        type="button"
        onclick={handleShowPreviousRecord}
        disabled={!lastSavedRecord}
        class="bg-slate-200 hover:bg-slate-300 disabled:bg-slate-100 disabled:text-slate-400 disabled:cursor-not-allowed text-slate-700 px-4 py-2 text-xs font-bold uppercase tracking-wider rounded-none transition-colors"
      >
        Show Previous
      </button>
    <div class="w-1/2 flex justify-between gap-2" >
      <button
        type="button"
        onclick={handleReset}
        class="bg-slate-200 hover:bg-slate-300 text-slate-700 px-4 py-2 text-xs font-bold uppercase tracking-wider rounded-none transition-colors"
      >
        Reset Form
      </button>
      <button
        type="button"
        onclick={handleSave}
        disabled={saving || !sessionId}
        class="flex-1 bg-slate-800 hover:bg-slate-900 disabled:bg-slate-300 disabled:text-slate-500 disabled:cursor-not-allowed text-white px-6 py-2 text-xs font-bold uppercase tracking-wider rounded-none transition-colors flex justify-center items-center gap-2"
      >
        {#if saving}
          <span>Saving...</span>
        {:else}
          <span>{form.id ? "Update Specimen" : "Save Specimen"}</span>
        {/if}
      </button>
    </div>
  </div>
</div>
