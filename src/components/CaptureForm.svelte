<script>
  import { onDestroy, getContext } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { convert } from 'geo-coordinates-parser';
  import { titleCase, SMALL_WORDS } from "title-case";
  import parser from 'any-date-parser';
  import Autocomplete from "./Autocomplete.svelte";
  import MultiSelectAutocomplete from "./MultiSelectAutocomplete.svelte";
  import { isValidPartialDate, comparePartialDates } from "../utils/isValidPartialDate.js";

  const t = getContext("t");

  let {
    sessionId = null,
    collectionCode, // no default, this must be set
    activeRecord = $bindable(null), // The selected record to edit (or empty for new)
    onSaveSuccess = () => {}
  } = $props();

  let form = $state({
    id: /** @type {number|null} */ (null),
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
    fieldNotes: "",
    cultivated: false
  });

  let coordinatesError = $state(false);
  let lastSavedRecord = $state(/** @type {any} */ (null));

  let eventDateLanguage = $state("EN"); // Default to English for verbatimEventDate parsing, can be set to "FR" for French dates

  let isAnyGeoPopulated = $derived(
    !!((form.country && form.country.trim().length > 0) ||
       (form.stateProvince && form.stateProvince.trim().length > 0) ||
       (form.county && form.county.trim().length > 0) ||
       (form.municipality && form.municipality.trim().length > 0))
  );

  let saving = $state(false);
  let statusMessageKey = $state("");
  let statusMessageDefault = $state("");
  let statusMessage = $derived(statusMessageKey ? t(statusMessageKey, statusMessageDefault) : statusMessageDefault);
  let statusType = $state(""); // "success" or "error"
  let formRef = $state(/** @type {HTMLFormElement|null} */ (null));

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

  // Watch activeRecord changes (when a search result is clicked, copy it to the form!)
  $effect(() => {
    if (activeRecord) {
      form.id = activeRecord.id && activeRecord.sessionId ? activeRecord.id : null; // Only reuse id if it is a previously captured record, not a reference database record
      // do not copy the collection code, this must always be the value from the active session settings
      form.catalogNumber = activeRecord.catalogNumber || "";
      form.duplicates = activeRecord.duplicates ? activeRecord.duplicates + ", " : "";
      
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
      form.cultivated = !!activeRecord.cultivated;
      clearTitleCasedStates();
      
      statusMessage = "";
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

  function handleCollectionDateBlur() {
    const { day, month, year } = form;
    if (!isValidPartialDate(year, month, day)) {
      statusMessageKey = "invalid-date-error";
      statusMessageDefault = "Error: Invalid collection date.";
      statusType = "error";
    } else {
      if (statusMessageKey === "invalid-date-error") {
        statusMessageKey = "";
        statusMessageDefault = "";
        statusType = "";
      }
      
      // Clear comparison error if collection date was corrected to be before or equal to id date
      const { dayIdentified, monthIdentified, yearIdentified } = form;
      if (
        statusMessageKey === "id-date-before-collection-error" &&
        isValidPartialDate(yearIdentified, monthIdentified, dayIdentified) &&
        comparePartialDates(yearIdentified, monthIdentified, dayIdentified, year, month, day) >= 0
      ) {
        statusMessageKey = "";
        statusMessageDefault = "";
        statusType = "";
      }
    }
  }

  function handleIdentificationDateBlur() {
    const { dayIdentified, monthIdentified, yearIdentified } = form;
    if (!isValidPartialDate(yearIdentified, monthIdentified, dayIdentified)) {
      statusMessageKey = "invalid-id-date-error";
      statusMessageDefault = "Error: Invalid identification date.";
      statusType = "error";
    } else if (
      yearIdentified && form.year &&
      comparePartialDates(yearIdentified, monthIdentified, dayIdentified, form.year, form.month, form.day) < 0
    ) {
      statusMessageKey = "id-date-before-collection-error";
      statusMessageDefault = "Error: Identification date cannot be before collection date.";
      statusType = "error";
    } else {
      if (
        statusMessageKey === "invalid-id-date-error" ||
        statusMessageKey === "id-date-before-collection-error"
      ) {
        statusMessageKey = "";
        statusMessageDefault = "";
        statusType = "";
      }
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

  async function handleStateProvinceFocus() {
    if (form.country && form.country.trim().length > 0) {
      try {
        stateProvinceSuggestions = /** @type {any[]} */ (await invoke("autocomplete_geography", {
          field: "stateProvince",
          query: "",
          country: form.country,
          stateProvince: "",
          county: ""
        }));
      } catch (e) {
        console.error(e);
      }
    }
  }

  async function handleCountyFocus() {
    if (form.stateProvince && form.stateProvince.trim().length > 0) {
      try {
        countySuggestions = /** @type {any[]} */ (await invoke("autocomplete_geography", {
          field: "county",
          query: "",
          country: form.country,
          stateProvince: form.stateProvince,
          county: ""
        }));
      } catch (e) {
        console.error(e);
      }
    }
  }

  async function handleMunicipalityFocus() {
    if (form.county && form.county.trim().length > 0) {
      try {
        municipalitySuggestions = /** @type {any[]} */ (await invoke("autocomplete_geography", {
          field: "municipality",
          query: "",
          country: form.country,
          stateProvince: form.stateProvince,
          county: form.county
        }));
      } catch (e) {
        console.error(e);
      }
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

    let {day, month, year} = parser.attempt(dateStr, eventDateLanguage === "FR" ? 'fr-FR'  : 'en-US');

    form.day = day ? String(day) : "";
    form.month = month ? String(month) : "";
    form.year = year ? String(year) : "";
    
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
      statusMessageKey = "no-active-session-error";
      statusMessageDefault = "Error: No active capture session selected.";
      statusType = "error";
      return;
    }
    
    saving = true;
    statusMessageKey = "";
    statusMessageDefault = "";
    
    // Validate collection date on save
    if (!isValidPartialDate(form.year, form.month, form.day)) {
      statusMessageKey = "invalid-date-error";
      statusMessageDefault = "Error: Invalid collection date.";
      statusType = "error";
      saving = false;
      return;
    }

    // Validate identification date on save
    if (!isValidPartialDate(form.yearIdentified, form.monthIdentified, form.dayIdentified)) {
      statusMessageKey = "invalid-id-date-error";
      statusMessageDefault = "Error: Invalid identification date.";
      statusType = "error";
      saving = false;
      return;
    }

    // Validate identification date is not before collection date on save
    if (
      form.yearIdentified && form.year &&
      comparePartialDates(form.yearIdentified, form.monthIdentified, form.dayIdentified, form.year, form.month, form.day) < 0
    ) {
      statusMessageKey = "id-date-before-collection-error";
      statusMessageDefault = "Error: Identification date cannot be before collection date.";
      statusType = "error";
      saving = false;
      return;
    }
    
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
      collectionCode,
      recordedBy: combinedRecordedBy,
      duplicates: form.duplicates.replace(/,\s*$/, ""), // Remove trailing comma space if present
      identifiedBy: combinedIdentifiedBy,
      sessionId: sessionId,
      year: form.year !== "" ? parseInt(form.year) : null,
      month: form.month !== "" ? parseInt(form.month) : null,
      day: form.day !== "" ? parseInt(form.day) : null,
      yearIdentified: form.yearIdentified !== "" ? parseInt(form.yearIdentified) : null,
      monthIdentified: form.monthIdentified !== "" ? parseInt(form.monthIdentified) : null,
      dayIdentified: form.dayIdentified !== "" ? parseInt(form.dayIdentified) : null
    };

    // some validation, just so we don't save a blank record
    if (Object.values(recordPayload).every(value => value === "" || value === null || (Array.isArray(value) && value.length === 0))) {
      statusMessageKey = "empty-record-error";
      statusMessageDefault = "Error: Cannot save an empty record.";
      statusType = "error";
      saving = false;
      return;
    }

    
    try {
      let res = /** @type {any} */ (await invoke("save_captured_record", { record: recordPayload }));
      if (res.success) {
        const isUpdate = !!form.id;
        
        lastSavedRecord = {
          ...recordPayload,
          id: res.id
        };
        
        handleReset();
        
        statusMessageKey = isUpdate ? "record-updated-success" : "record-saved-success";
        statusMessageDefault = isUpdate ? "Record updated successfully!" : "Specimen saved successfully!";
        statusType = "success";
        
        onSaveSuccess();
        
        if (formRef) {
          formRef.scrollTop = 0;
        }
        
        // Hide success message after 3 seconds
        setTimeout(() => {
          if (statusType === "success") {
            statusMessageKey = "";
            statusMessageDefault = "";
          }
        }, 3000);
      }
    } catch (err) {
      statusMessageKey = "";
      statusMessageDefault = `Error: ${(/** @type {any} */ (err)).toString()}`;
      statusType = "error";
    } finally {
      saving = false;
    }
  }

  function handleReset() {
    form = {
      id: null,
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
      fieldNotes: "",
      cultivated: false
    };
    activeRecord = null;
    statusMessageKey = "";
    statusMessageDefault = "";
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
    statusMessageKey = "record-loaded-last";
    statusMessageDefault = "Loaded last saved record.";
    statusType = "success";
    
    // Clear status message after 3 seconds
    setTimeout(() => {
      if (statusMessageKey === "record-loaded-last") {
        statusMessageKey = "";
        statusMessageDefault = "";
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

  $effect(() => {
    if (eventDateLanguage) {
      parseVerbatimDate();
    }
  })
</script>

<div class="flex flex-col h-full bg-white border border-slate-300">
  <!-- Header Title -->
  <div class="px-4 py-3 bg-slate-100 border-b border-slate-300 flex justify-between items-center border-box">
    <div class="flex items-center gap-2">
      <h2 data-i18n-key={form.id ? "edit-specimen-heading" : "capture-specimen-heading"} class="text-sm font-bold text-slate-800 uppercase tracking-wide">
        {form.id ? t("edit-specimen-heading", "Edit Captured Specimen") : t("capture-specimen-heading", "Capture New Specimen")}
      </h2>
      {#if form.id}
        <span data-i18n-key="saved-capture-badge" class="text-[9px] bg-indigo-100 text-indigo-800 font-bold uppercase tracking-wider px-1.5 py-0.5">{t("saved-capture-badge", "SAVED CAPTURE")}</span>
      {:else}
        <span data-i18n-key="new-form-badge" class="text-[9px] bg-emerald-100 text-emerald-800 font-bold uppercase tracking-wider px-1.5 py-0.5">{t("new-form-badge", "NEW FORM")}</span>
      {/if}
    </div>
    <span data-i18n-key="save-shortcut-desc" class="text-[10px] text-slate-400 font-semibold uppercase">{t("save-shortcut-desc", "Shortcut: Ctrl+S to save")}</span>
  </div>

  <!-- Form Fields -->
  <form bind:this={formRef} onsubmit={handleSave} class="flex-1 overflow-y-auto p-4 space-y-4">
    

    <!-- Row 1: Home Herbarium (read-only), catalogNumber, duplicates -->
    <div class="grid grid-cols-12 gap-3">
      <div class="col-span-3">
        <label for="capture-collectionCode" data-i18n-key="home-herbarium-label" class="block text-xs font-semibold text-slate-650 uppercase tracking-wider mb-1">{t("home-herbarium-label", "Home Herbarium")}</label>
        <input
          id="capture-collectionCode"
          type="text"
          readonly
          bind:value={collectionCode}
          class="w-full bg-slate-100 border border-slate-300 text-slate-500 text-sm px-3 py-2 outline-none rounded-none font-semibold"
        />
      </div>
      <div class="col-span-4">
        <label for="capture-catalogNumber" data-i18n-key="catalog-number-label" class="block text-xs font-semibold text-slate-655 uppercase tracking-wider mb-1">{t("catalog-number-label", "Barcode Number")}</label>
        <input
          id="capture-catalogNumber"
          data-i18n-key="catalog-number-placeholder"
          type="text"
          placeholder={t("catalog-number-placeholder", "eg TAN123456")}
          bind:value={form.catalogNumber}
          class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
        />
      </div>
      <div class="col-span-5">
        <Autocomplete
          id="capture-duplicates"
          label="Duplicates (comma-separated)"
          labelKey="duplicates-label"
          placeholder="eg P, K, etc..."
          placeholderKey="duplicates-placeholder"
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
          labelKey="recorded-by-label"
          placeholder="Partial eg 'Raza'"
          placeholderKey="recorded-by-placeholder"
          bind:value={form.recordedBy}
          suggestions={collectorSuggestions}
          oninput={handleCollectorInput}
          delay={300}
        />
      </div>
      <div class="col-span-2">
        <label for="capture-recordNumber" data-i18n-key="record-number-label" class="block text-xs font-semibold text-slate-650 uppercase tracking-wider mb-1">{t("record-number-label", "Collector No.")}</label>
        <input
          id="capture-recordNumber"
          data-i18n-key="record-number-placeholder"
          type="text"
          placeholder={t("record-number-placeholder", "eg 1042")}
          bind:value={form.recordNumber}
          class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
        />
      </div>
      
      <div class="col-span-3">
        <label for="capture-verbatimEventDate" data-i18n-key="verbatim-event-date-label" class="block text-xs font-semibold text-slate-650 uppercase tracking-wider mb-1">
          <div class="flex justify-between">
            <span>
              {t("verbatim-event-date-label", "Verbatim Date")}
            </span>
            <div class="flex items-center  divide-x divide-slate-300 select-none">
              <button
                type="button"
                onclick={() => eventDateLanguage = "EN"}
                class="px-1 py-0.5 text-[10px] font-bold tracking-wider hover:bg-slate-50 transition-colors cursor-pointer {eventDateLanguage === 'EN' ? 'bg-slate-800 text-white hover:bg-slate-800' : 'bg-white text-slate-400'}"
              >
                EN
              </button>
              <button
                type="button"
                onclick={() => eventDateLanguage = "FR"}
                class="px-1 py-0.5 text-[10px] font-bold tracking-wider hover:bg-slate-50 transition-colors cursor-pointer {eventDateLanguage === 'FR' ? 'bg-slate-800 text-white hover:bg-slate-800' : 'bg-white text-slate-400'}"
              >
                FR
              </button>
            </div>
          </div>
        </label>
        <input
          id="capture-verbatimEventDate"
          data-i18n-key="verbatim-event-date-placeholder"
          type="text"
          placeholder={t("verbatim-event-date-placeholder", "eg 'May 20, `84'")}
          bind:value={form.verbatimEventDate}
          onblur={parseVerbatimDate}
          class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
        />
      </div>
      <div class="col-span-4 flex gap-2">
        <div class="flex-1">
          <label for="capture-year" data-i18n-key="year-label" class="block text-xs font-semibold text-slate-650 uppercase tracking-wider mb-1">{t("year-label", "Year")}</label>
          <input
            id="capture-year"
            type="number"
            bind:value={form.year}
            onblur={handleCollectionDateBlur}
            class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-2 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
          />
        </div>
        <div class="flex-1">
          <label for="capture-month" data-i18n-key="month-label" class="block text-xs font-semibold text-slate-650 uppercase tracking-wider mb-1">{t("month-label", "Month")}</label>
          <input
            id="capture-month"
            type="number"
            min="1"
            max="12"
            bind:value={form.month}
            onblur={handleCollectionDateBlur}
            class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-2 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
          />
        </div>
        <div class="flex-1">
          <label for="capture-day" data-i18n-key="day-label" class="block text-xs font-semibold text-slate-650 uppercase tracking-wider mb-1">{t("day-label", "Day")}</label>
          <input
            id="capture-day"
            type="number"
            min="1"
            max="31"
            bind:value={form.day}
            onblur={handleCollectionDateBlur}
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
          labelKey="add-collectors-label"
          placeholder="Type name and press Enter..."
          placeholderKey="add-collectors-placeholder"
          bind:selectedValues={form.additionalCollectors}
          suggestions={additionalCollectorsSuggestions}
          oninput={handleAdditionalCollectorsInput}
          delay={300}
        />
      </div>
    </div>

    <!-- Row 3: Geography with Title-case buttons -->
    <div class="space-y-3 pt-2">
      <h3 data-i18n-key="geographic-locality-heading" class="text-[10px] font-bold text-slate-400 uppercase tracking-wider border-b border-slate-100 pb-1">{t("geographic-locality-heading", "Geographic Locality")}</h3>
      
      <div class="grid grid-cols-4 gap-3">
        <div>
          <label for="capture-country" data-i18n-key="country-label" class="block text-xs font-semibold text-slate-650 uppercase tracking-wider mb-1">{t("country-label", "Country")}</label>
          <div class="relative flex items-center">
            <Autocomplete
              id="capture-country"
              label=""
              placeholder={isAnyGeoPopulated ? "" : "eg Madagascar"}
              placeholderKey={isAnyGeoPopulated ? undefined : "country-placeholder"}
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
                data-i18n-key="undo-title-case"
                title={t("undo-title-case", "Undo Title case")}
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
                data-i18n-key="title-case-country"
                title={t("title-case-country", "Title case Country")}
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
          <label for="capture-stateProvince" data-i18n-key="state-province-label" class="block text-xs font-semibold text-slate-655 uppercase tracking-wider mb-1">
            {t("state-province-label", "Admin 2")}
            <span data-i18n-key="state-province-sub" class="text-[70%]">{t("state-province-sub", "(state/prov/etc)")}</span>
          </label>
          <div class="relative flex items-center">
            <Autocomplete
              id="capture-stateProvince"
              label=""
              placeholder={isAnyGeoPopulated ? "" : "eg 'Itasy'"}
              placeholderKey={isAnyGeoPopulated ? undefined : "state-province-placeholder"}
              bind:value={form.stateProvince}
              suggestions={stateProvinceSuggestions}
              oninput={handleStateProvinceInput}
              onselect={onStateProvinceChanged}
              onfocus={handleStateProvinceFocus}
              delay={300}
            />
            {#if form.stateProvince === titleCasedStates.stateProvince.titleCased && titleCasedStates.stateProvince.titleCased !== ""}
              <button
                type="button"
                onclick={() => undoTitleCaseField("stateProvince")}
                data-i18n-key="undo-title-case"
                title={t("undo-title-case", "Undo Title case")}
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
                data-i18n-key="title-case-admin2"
                title={t("title-case-admin2", "Title case Admin 2")}
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
          <label for="capture-county" data-i18n-key="county-label" class="block text-xs font-semibold text-slate-650 uppercase tracking-wider mb-1">
            {t("county-label", "Admin 3")}
            <span data-i18n-key="county-sub" class="text-[70%]">{t("county-sub", "(County/Dist/etc)")}</span>
          </label>
          <div class="relative flex items-center">
            <Autocomplete
              id="capture-county"
              label=""
              placeholder={isAnyGeoPopulated ? "" : "eg 'Miarinarivo'"}
              placeholderKey={isAnyGeoPopulated ? undefined : "county-placeholder"}
              bind:value={form.county}
              suggestions={countySuggestions}
              oninput={handleCountyInput}
              onselect={onCountyChanged}
              onfocus={handleCountyFocus}
              delay={300}
            />
            {#if form.county === titleCasedStates.county.titleCased && titleCasedStates.county.titleCased !== ""}
              <button
                type="button"
                onclick={() => undoTitleCaseField("county")}
                data-i18n-key="undo-title-case"
                title={t("undo-title-case", "Undo Title case")}
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
                data-i18n-key="title-case-admin3"
                title={t("title-case-admin3", "Title case Admin 3")}
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
          <label for="capture-municipality" data-i18n-key="municipality-label" class="block text-xs font-semibold text-slate-655 uppercase tracking-wider mb-1">{t("municipality-label", "Municipality")}</label>
          <div class="relative flex items-center">
            <Autocomplete
              id="capture-municipality"
              label=""
              placeholder={isAnyGeoPopulated ? "" : "eg 'Manazary'"}
              placeholderKey={isAnyGeoPopulated ? undefined : "municipality-placeholder"}
              bind:value={form.municipality}
              suggestions={municipalitySuggestions}
              oninput={handleMunicipalityInput}
              onfocus={handleMunicipalityFocus}
              delay={300}
            />
            {#if form.municipality === titleCasedStates.municipality.titleCased && titleCasedStates.municipality.titleCased !== ""}
              <button
                type="button"
                onclick={() => undoTitleCaseField("municipality")}
                data-i18n-key="undo-title-case"
                title={t("undo-title-case", "Undo Title case")}
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
                data-i18n-key="title-case-admin4"
                title={t("title-case-admin4", "Title case Admin 4")}
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
      <div class="grid grid-cols-4 gap-3">
        <div class="col-span-3">
          <label for="capture-locality" data-i18n-key="locality-label" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">{t("locality-label", "Locality")}</label>
          <div class="relative flex items-center w-full">
            <Autocomplete
              id="capture-locality"
              label=""
              placeholder="eg 'Antakohandro' (use partial search eg 'Anta')"
              placeholderKey="locality-placeholder"
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
                  data-i18n-key="undo-title-case"
                  title={t("undo-title-case", "Undo Title case")}
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
                  data-i18n-key="title-case-locality"
                  title={t("title-case-locality", "Title case Locality")}
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
                data-i18n-key={localityCopied ? "copied-msg" : "copy-text-desc"}
                title={localityCopied ? t("copied-msg", "Copied!") : t("copy-text-desc", "Copy selection or entire text")}
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
                data-i18n-key="paste-text-desc"
                title={t("paste-text-desc", "Paste clipboard contents")}
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
          <!-- spacer matching the locality label -->
          <div class="block text-xs font-semibold uppercase tracking-wider mb-1 invisible">
            Cultivated
          </div>

          <label
            for="capture-cultivated"
            class="flex items-center h-[38px] gap-2 cursor-pointer"
          >
            <input
              id="capture-cultivated"
              type="checkbox"
              bind:checked={form.cultivated}
              class="w-4 h-4 text-slate-800 border-slate-300 rounded focus:ring-slate-500 focus:ring-1"
            />
            <span data-i18n-key="cultivated-label">
              {t("cultivated-label", "Cultivated")}
            </span>
          </label>
        </div>

      </div>

      <!-- Row 5: Locality Notes (locationNotes) -->
      <div>
        <label for="capture-locationNotes" data-i18n-key="location-notes-label" class="block text-xs font-semibold text-slate-655 uppercase tracking-wider mb-1">{t("location-notes-label", "Locality Notes")}</label>
        <div class="relative flex items-start w-full">
          <textarea
            bind:this={locationNotesRef}
            id="capture-locationNotes"
            rows="2"
            data-i18n-key="location-notes-placeholder"
            placeholder={t("location-notes-placeholder", "eg '12 km south, main ravine'")}
            bind:value={form.locationNotes}
            class="w-full bg-white border border-slate-300 text-slate-800 text-sm pl-3 pr-24 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all resize-none"
          ></textarea>
          <div class="absolute right-2 top-2 flex items-center gap-1 z-10 bg-white pl-1">
            {#if form.locationNotes === titleCasedStates.locationNotes.titleCased && titleCasedStates.locationNotes.titleCased !== ""}
              <button
                type="button"
                onclick={() => undoTitleCaseField("locationNotes")}
                data-i18n-key="undo-title-case"
                title={t("undo-title-case", "Undo Title case")}
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
                data-i18n-key="title-case-locality-notes"
                title={t("title-case-locality-notes", "Title case Locality Notes")}
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
              data-i18n-key={locationNotesCopied ? "copied-msg" : "copy-text-desc"}
              title={locationNotesCopied ? t("copied-msg", "Copied!") : t("copy-text-desc", "Copy selection or entire text")}
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
              data-i18n-key="paste-text-desc"
              title={t("paste-text-desc", "Paste clipboard contents")}
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
        <label for="capture-verbatimCoordinates" data-i18n-key="verbatim-coordinates-label" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">{t("verbatim-coordinates-label", "Verbatim Coordinates")}</label>
        <input
          id="capture-verbatimCoordinates"
          data-i18n-key="verbatim-coordinates-placeholder"
          type="text"
          placeholder={t("verbatim-coordinates-placeholder", "eg 28°15'S, 28°39'E")}
          bind:value={form.verbatimCoordinates}
          onblur={handleCoordinatesBlur}
          class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
        />
      </div>
      {#if coordinatesError}
        <div data-i18n-key="coordinates-error-warn" class="mt-3 text-xs bg-amber-50 border-l-2 border-amber-500 text-amber-700 px-3 py-2 font-medium">
          {t("coordinates-error-warn", "⚠️ Unable to parse coordinates, please check they are correct")}
        </div>
      {/if}
    </div>

    

    <!-- Row 6: Verbatim Locality (grayed-out read-only) -->
    <div>
      <label for="capture-verbatimLocality" data-i18n-key="verbatim-locality-label" class="block text-xs font-semibold text-slate-650 uppercase tracking-wider mb-1">{t("verbatim-locality-label", "Verbatim Locality (copy data above)")}</label>
      <div class="relative">
        <textarea
          bind:this={verbatimLocalityRef}
          id="capture-verbatimLocality"
          rows="2"
          readonly
          data-i18n-key="verbatim-locality-placeholder"
          placeholder={t("verbatim-locality-placeholder", "Read-only imported value")}
          bind:value={form.verbatimLocality}
          class="w-full bg-slate-100 border border-slate-300 text-slate-500 text-sm px-3 py-2 outline-none rounded-none pr-10"      >
        </textarea>
        <button
          type="button"
          onclick={handleCopyVerbatimLocality}
          data-i18n-key={verbatimLocalityCopied ? "copied-msg" : "copy-text-desc"}
          title={verbatimLocalityCopied ? t("copied-msg", "Copied!") : t("copy-text-desc", "Copy selection or entire text")}
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
        <label for="capture-verbatimElevation" data-i18n-key="verbatim-elevation-label" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">{t("verbatim-elevation-label", "Verbatim Elevation")}</label>
        <input
          id="capture-verbatimElevation"
          data-i18n-key="verbatim-elevation-placeholder"
          type="text"
          placeholder={t("verbatim-elevation-placeholder", "eg 1200m")}
          bind:value={form.verbatimElevation}
          class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
        />
      </div>
      <div class="flex-1">
        <label for="capture-habitat" data-i18n-key="habitat-label" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">{t("habitat-label", "Habitat")}</label>
        <div class="relative flex items-center">
          <input
            id="capture-habitat"
            data-i18n-key="habitat-placeholder"
            type="text"
            placeholder={t("habitat-placeholder", "eg Oak woodland, sandy soil...")}
            bind:value={form.habitat}
            class="w-full bg-white border border-slate-300 text-slate-800 text-sm pl-3 pr-8 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
          />
          {#if form.habitat === titleCasedStates.habitat.titleCased && titleCasedStates.habitat.titleCased !== ""}
            <button
              type="button"
              onclick={() => undoTitleCaseField("habitat")}
              data-i18n-key="undo-title-case"
              title={t("undo-title-case", "Undo Title case")}
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
              data-i18n-key="title-case-habitat"
              title={t("title-case-habitat", "Title case Habitat")}
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
      <h3 data-i18n-key="field-notes-label" class="text-[10px] font-bold text-slate-400 uppercase tracking-wider border-b border-slate-100 pb-1">{t("field-notes-label", "Plant description")}</h3>
      <div class="relative flex items-start">
        <textarea
          id="capture-fieldNotes"
          rows="2"
          data-i18n-key="field-notes-placeholder"
          placeholder={t("field-notes-placeholder", "eg flower yellow, tree 5m")}
          bind:value={form.fieldNotes}
          class="w-full bg-white border border-slate-300 text-slate-800 text-sm pl-3 pr-8 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all resize-none"
        ></textarea>
        {#if form.fieldNotes === titleCasedStates.fieldNotes.titleCased && titleCasedStates.fieldNotes.titleCased !== ""}
          <button
            type="button"
            onclick={() => undoTitleCaseField("fieldNotes")}
            data-i18n-key="undo-title-case"
            title={t("undo-title-case", "Undo Title case")}
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
            data-i18n-key="title-case-field-notes"
            title={t("title-case-field-notes", "Title case Plant description")}
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
      <h3 data-i18n-key="occurrence-remarks-label" class="text-[10px] font-bold text-slate-400 uppercase tracking-wider border-b border-slate-100 pb-1">{t("occurrence-remarks-label", "General Notes")}</h3>
      <div class="relative flex items-start">
        <textarea
          id="capture-occurrenceRemarks"
          rows="2"
          data-i18n-key="occurrence-remarks-placeholder"
          placeholder={t("occurrence-remarks-placeholder", "Any other notes...")}
          bind:value={form.occurrenceRemarks}
          class="w-full bg-white border border-slate-300 text-slate-800 text-sm pl-3 pr-8 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all resize-none"
        ></textarea>
        {#if form.occurrenceRemarks === titleCasedStates.occurrenceRemarks.titleCased && titleCasedStates.occurrenceRemarks.titleCased !== ""}
          <button
            type="button"
            onclick={() => undoTitleCaseField("occurrenceRemarks")}
            data-i18n-key="undo-title-case"
            title={t("undo-title-case", "Undo Title case")}
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
            data-i18n-key="title-case-occurrence-remarks"
            title={t("title-case-occurrence-remarks", "Title case General Notes")}
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
      <h3 data-i18n-key="identification-heading" class="text-[10px] font-bold text-slate-400 uppercase tracking-wider border-b border-slate-100 pb-1">{t("identification-heading", "Identification")}</h3>
      
      <div class="grid grid-cols-12 gap-3">
        <!-- Qualifier dropdown -->
        <div class="col-span-3">
          <label for="capture-identificationQualifier" data-i18n-key="id-qualifier-label" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">{t("id-qualifier-label", "Qualifier")}</label>
          <select
            id="capture-identificationQualifier"
            bind:value={form.identificationQualifier}
            class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-2 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
          >
            <option value="" data-i18n-key="qualifier-none-option">{t("qualifier-none-option", "(None)")}</option>
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
            labelKey="scientific-name-label"
            placeholder="Partial search eg 'ab man'"
            placeholderKey="scientific-name-placeholder"
            bind:value={form.scientificName}
            suggestions={taxonSuggestions}
            oninput={handleTaxonInput}
            onselect={handleTaxonSelect}
            delay={300}
          />
        </div>

        <!-- Type Status autocomplete -->
        <div class="col-span-3">
          <label for="capture-typeStatus" data-i18n-key="type-status-label" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">{t("type-status-label", "Type Status")}</label>
          <Autocomplete
            id="capture-typeStatus"
            label=""
            placeholder="eg holotype"
            placeholderKey="type-status-placeholder"
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
          label="Det By"
          labelKey="det-by-label"
          placeholder="Type name and press Enter..."
          placeholderKey="det-by-placeholder"
          bind:selectedValues={form.identifiedBy}
          suggestions={identifiedBySuggestions}
          oninput={handleIdentifiedByInput}
          delay={300}
        />
      </div>
      <div class="col-span-2">
        <label for="capture-yearIdentified" data-i18n-key="det-year-label" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">{t("det-year-label", "Year Ident.")}</label>
        <input
          id="capture-yearIdentified"
          type="number"
          bind:value={form.yearIdentified}
          onblur={handleIdentificationDateBlur}
          class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-2 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
        />
      </div>
      <div class="col-span-2">
        <label for="capture-monthIdentified" data-i18n-key="det-month-label" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">{t("det-month-label", "Month Ident.")}</label>
        <input
          id="capture-monthIdentified"
          type="number"
          min="1"
          max="12"
          bind:value={form.monthIdentified}
          onblur={handleIdentificationDateBlur}
          class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-2 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
        />
      </div>
      <div class="col-span-2">
        <label for="capture-dayIdentified" data-i18n-key="det-day-label" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">{t("det-day-label", "Day Ident.")}</label>
        <input
          id="capture-dayIdentified"
          type="number"
          min="1"
          max="31"
          bind:value={form.dayIdentified}
          onblur={handleIdentificationDateBlur}
          class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-2 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
        />
      </div>
    </div>

    <!-- Row 9: Identification Notes -->
    <div class="pb-6">
      <label for="capture-identificationRemarks" data-i18n-key="det-remarks-label" class="block text-xs font-semibold text-slate-655 uppercase tracking-wider mb-1">{t("det-remarks-label", "Identification Notes")}</label>
      <textarea
        id="capture-identificationRemarks"
        rows="2"
        data-i18n-key="det-remarks-placeholder"
        placeholder={t("det-remarks-placeholder", "e.g. Similar to [species name] but has different bract structures")}
        bind:value={form.identificationRemarks}
        class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all resize-none"
      ></textarea>
    </div>
  </form>

  <!-- Save Action Footer -->
  {#if statusMessage}
    <div data-i18n-key={statusMessageKey || null} class="p-3 text-xs border font-medium {statusType === 'success' ? 'bg-emerald-50 border-emerald-300 text-emerald-800' : 'bg-red-50 border-red-300 text-red-800'}">
      {statusMessage}
    </div>
  {/if}
  <div class="p-4 border-t flex justify-between  border-slate-300 bg-slate-50 ">
    <button
        type="button"
        onclick={handleShowPreviousRecord}
        disabled={!lastSavedRecord}
        data-i18n-key="show-previous-btn"
        class="bg-slate-200 hover:bg-slate-300 disabled:bg-slate-100 disabled:text-slate-400 disabled:cursor-not-allowed text-slate-700 px-4 py-2 text-xs font-bold uppercase tracking-wider rounded-none transition-colors"
      >
        {t("show-previous-btn", "Show Previous")}
      </button>
    <div class="w-1/2 flex justify-between gap-2" >
      <button
        type="button"
        onclick={handleReset}
        data-i18n-key="reset-form-btn"
        class="bg-slate-200 hover:bg-slate-300 text-slate-700 px-4 py-2 text-xs font-bold uppercase tracking-wider rounded-none transition-colors"
      >
        {t("reset-form-btn", "Reset Form")}
      </button>
      <button
        type="button"
        onclick={handleSave}
        disabled={saving || !sessionId}
        class="flex-1 bg-slate-800 hover:bg-slate-900 disabled:bg-slate-300 disabled:text-slate-500 disabled:cursor-not-allowed text-white px-6 py-2 text-xs font-bold uppercase tracking-wider rounded-none transition-colors flex justify-center items-center gap-2"
      >
        {#if saving}
          <span data-i18n-key="saving-status">{t("saving-status", "Saving...")}</span>
        {:else}
          {#if form.id}
            <span data-i18n-key="update-specimen-btn">{t("update-specimen-btn", "Update Specimen")}</span>
          {:else}
            <span data-i18n-key="save-specimen-btn">{t("save-specimen-btn", "Save Specimen")}</span>
          {/if}
        {/if}
      </button>
    </div>
  </div>
</div>
