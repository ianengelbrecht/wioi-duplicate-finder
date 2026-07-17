<script>
  import { getContext, onDestroy } from "svelte";
  import parser from "any-date-parser";
  import { convert } from "geo-coordinates-parser";
  import Autocomplete from "./Autocomplete.svelte";
  import MultiSelectAutocomplete from "./MultiSelectAutocomplete.svelte";
  import CopyIcon from "./icons/CopyIcon.svelte";
  import PasteIcon from "./icons/PasteIcon.svelte";
  import AaIcon from "./icons/AaIcon.svelte";
  import UndoIcon from "./icons/UndoIcon.svelte";
  import CheckIcon from "./icons/CheckIcon.svelte";
  import SearchIcon from "./icons/SearchIcon.svelte";
  import { specimenService } from "../services/specimenService.js";
  import { taxonomyService } from "../services/taxonomyService.js";
  import { agentService } from "../services/agentService.js";
  import { geographyService } from "../services/geographyService.js";
  import { isValidPartialDate, comparePartialDates } from "../utils/isValidPartialDate.js";
  import { splitNames } from "../utils/splitNames.js";
  import { coordsToQDS } from "../utils/coordsToQDS.js";
  import { getDuplicateSuggestions } from "../utils/duplicates.js";
  import { titleCaseField, undoTitleCaseField, getInitialTrackingState } from "../utils/titleCaseHelper.js";
  import { copySelectedOrValue, pasteAtCursor } from "../utils/clipboard.js";
  import { workspaceStore } from "../stores/workspaceStore.svelte.js";
  import { getPlaceholders } from "../utils/countryData.js";
  import { makeAgentFilter } from "../utils/makeAgentFilter.js";

  const agentFilter = $derived(makeAgentFilter({ initialsRequirePeriods: workspaceStore.initialsRequirePeriods }));

  const t = getContext("t");

  /**
   * @typedef {Object} CaptureFormProps
   * @property {number|null} [sessionId=null]
   * @property {string} collectionCode
   * @property {any} [activeRecord=null]
   * @property {() => void} [onSaveSuccess]
   * @property {string} [currentLanguage="EN"]
   */

  /** @type {CaptureFormProps} */
  let {
    sessionId = null,
    collectionCode, // no default, this must be set
    activeRecord = $bindable(null), // The selected record to edit (or empty for new)
    onSaveSuccess = () => {}, 
    currentLanguage = "EN" // "EN" or "FR"
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
    stateProvince: "",
    county: "",
    islandGroup: "",
    island: "",
    locality: "",
    verbatimCoordinates: "",
    decimalLatitude: "",
    decimalLongitude: "",
    locationNotes: "", // Mapped to locationRemarks
    verbatimLocality: "",
    verbatimElevation: "",
    gridReference: "",
    habitat: "",
    identificationQualifier: "",
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
    cultivated: false,
    recordSource: "manual"
  });

  let isGridCalculated = $state(false);

  let placeholders = $derived(getPlaceholders(workspaceStore.homeCountry, currentLanguage));

  /** @type {boolean} */
  let coordinatesError = $state(false);
  /** @type {any} */
  let lastSavedRecord = $state(/** @type {any} */ (null));
  /** @type {boolean} */
  let saving = $state(false);
  /** @type {boolean} */
  let showBarcodeWarningModal = $state(false);

  /** @type {string} */
  let statusMessageKey = $state("");
  /** @type {string} */
  let statusMessageDefault = $state("");
  let statusMessage = $derived(statusMessageKey ? t(statusMessageKey, statusMessageDefault) : statusMessageDefault);
  /** @type {string} */
  let statusType = $state(""); // "success" or "error"
  /** @type {HTMLFormElement|null} */
  let formRef = $state(/** @type {HTMLFormElement|null} */ (null));

  // Title Casing state tracking object
  let titleCasedStates = $state(getInitialTrackingState());

  /**
   * Duplicates handling logic: we maintain a list of suggestions based on the current input, and filter out any duplicates that have already been entered. The user can type comma-separated values, and the last value is used for suggestions. When a suggestion is selected, it replaces the last value in the input.
   * @type {string[]}
   */
  let duplicateSuggestions = $state([]);
  const duplicatesList = ["P", "K", "MO", "TAN", "TEF", "PRE", "NU", "NH", "E", "WAG", "BR", "L", "BM", "US", "NY", "G"];

  function handleDuplicateInput(/** @type {string} */ val) {
    duplicateSuggestions = getDuplicateSuggestions(val, duplicatesList);
  }

  function handleDuplicateSelect(/** @type {string} */ sug) {
    const parts = form.duplicates.split(",").map(s => s.trim());
    if (parts.length > 0) {
      parts[parts.length - 1] = sug;
      form.duplicates = parts.join(", ") + ", ";
    } else {
      form.duplicates = sug + ", ";
    }
  }

  function handleDuplicateFocus() {
    handleDuplicateInput(form.duplicates);
  }

  // Handle activeRecord sync
  $effect(() => {
    if (activeRecord) {
      form.id = activeRecord.id || null;
      form.catalogNumber = activeRecord.catalogNumber || "";
      form.duplicates = activeRecord.duplicates || "";
      
      if (activeRecord.recordedBy) {
        const parts = splitNames(activeRecord.recordedBy);
        form.recordedBy = parts[0] || "";
        form.additionalCollectors = parts.slice(1);
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
      form.islandGroup = activeRecord.islandGroup || "";
      form.island = activeRecord.island || "";
      form.locality = activeRecord.locality || "";
      
      // Populate verbatimCoordinates: use verbatimCoordinates if present, otherwise combine decimalLatitude and decimalLongitude
      if (activeRecord.verbatimCoordinates) {
        form.verbatimCoordinates = activeRecord.verbatimCoordinates;
      } else if (
        (activeRecord.decimalLatitude !== null && activeRecord.decimalLatitude !== undefined && activeRecord.decimalLatitude !== "") &&
        (activeRecord.decimalLongitude !== null && activeRecord.decimalLongitude !== undefined && activeRecord.decimalLongitude !== "")
      ) {
        form.verbatimCoordinates = `${activeRecord.decimalLatitude}, ${activeRecord.decimalLongitude}`;
      } else {
        form.verbatimCoordinates = "";
      }
      
      form.decimalLatitude = activeRecord.decimalLatitude !== null && activeRecord.decimalLatitude !== undefined ? activeRecord.decimalLatitude.toString() : "";
      form.decimalLongitude = activeRecord.decimalLongitude !== null && activeRecord.decimalLongitude !== undefined ? activeRecord.decimalLongitude.toString() : "";
      
      form.locationNotes = activeRecord.locationNotes || activeRecord.locationRemarks || "";
      form.verbatimLocality = activeRecord.verbatimLocality || "";
      form.verbatimElevation = activeRecord.verbatimElevation || "";
      form.gridReference = activeRecord.gridReference || "";
      isGridCalculated = false;
      form.habitat = activeRecord.habitat || "";
      form.identificationQualifier = activeRecord.identificationQualifier || "";
      form.scientificName = activeRecord.scientificName || "";
      form.taxonID = activeRecord.taxonID || "";
      const activeSciName = activeRecord.scientificName || "";
      if (activeSciName.trim()) {
        const trimmedName = activeSciName.trim();
        taxonomyService.lookupTaxonByName(trimmedName).then((plantNameId) => {
          if (plantNameId) {
            form.taxonID = plantNameId;
            nameValidationStatus = "valid";
          } else {
            form.taxonID = "";
            nameValidationStatus = "invalid";
          }
        }).catch((err) => {
          console.error("Error looking up taxon for activeRecord:", err);
          form.taxonID = "";
          nameValidationStatus = "invalid";
        });
      } else {
        nameValidationStatus = "unchecked";
      }
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
      form.recordSource = activeRecord.recordSource || null;
      titleCasedStates = getInitialTrackingState();
      
      statusMessageKey = "";
      statusMessageDefault = "";
      statusType = "";
    }
  });

  /**
   * Checks if catalogNumber is already present in session records (except the current record).
   * @returns {boolean}
   */
  function isCatalogNumberDuplicate() {
    if (!form.catalogNumber) return false;
    const barcode = form.catalogNumber.trim().toLowerCase();
    if (!barcode) return false;

    return workspaceStore.capturedRecords.some(r => {
      if (form.id && r.id === form.id) return false;
      return (r.catalogNumber || "").trim().toLowerCase() === barcode;
    });
  }

  /**
   * Handle form submission: validate required fields and data formats, then save the record using the specimenService. Displays appropriate success or error messages based on the outcome.  
   * @param {Event|null} e
   * @param {boolean} [bypassDuplicate=false]
   */
  async function handleSave(e, bypassDuplicate = false) {
    if (e) e.preventDefault();
    if (saving) return;

    if (!bypassDuplicate && isCatalogNumberDuplicate()) {
      showBarcodeWarningModal = true;
      return;
    }

    saving = true;
    statusMessageKey = "";
    statusMessageDefault = "";
    statusType = "";

    // Validate coordinates error
    if (coordinatesError) {
      statusMessageKey = "invalid-coordinates-error";
      statusMessageDefault = "Error: Verbatim coordinates could not be parsed successfully.";
      statusType = "error";
      saving = false;
      return;
    }

    // Validate collection date
    if (!isValidPartialDate(form.year, form.month, form.day)) {
      statusMessageKey = "invalid-date-error";
      statusMessageDefault = "Error: Invalid collection date.";
      statusType = "error";
      saving = false;
      return;
    }

    // Validate identification date
    if (!isValidPartialDate(form.yearIdentified, form.monthIdentified, form.dayIdentified)) {
      statusMessageKey = "invalid-id-date-error";
      statusMessageDefault = "Error: Invalid identification date.";
      statusType = "error";
      saving = false;
      return;
    }

    // Validate identification date is not before collection date
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
      duplicates: form.duplicates.replace(/,\s*$/, ""), 
      identifiedBy: combinedIdentifiedBy,
      sessionId: sessionId,
      year: form.year !== "" ? parseInt(form.year) : null,
      month: form.month !== "" ? parseInt(form.month) : null,
      day: form.day !== "" ? parseInt(form.day) : null,
      yearIdentified: form.yearIdentified !== "" ? parseInt(form.yearIdentified) : null,
      monthIdentified: form.monthIdentified !== "" ? parseInt(form.monthIdentified) : null,
      dayIdentified: form.dayIdentified !== "" ? parseInt(form.dayIdentified) : null,
      decimalLatitude: (form.decimalLatitude !== "" && form.decimalLatitude !== null && form.decimalLatitude !== undefined && !isNaN(parseFloat(form.decimalLatitude))) ? parseFloat(form.decimalLatitude) : null,
      decimalLongitude: (form.decimalLongitude !== "" && form.decimalLongitude !== null && form.decimalLongitude !== undefined && !isNaN(parseFloat(form.decimalLongitude))) ? parseFloat(form.decimalLongitude) : null
    };

    // Prevent saving blank record
    if (Object.values(recordPayload).every(value => value === "" || value === null || (Array.isArray(value) && value.length === 0))) {
      statusMessageKey = "empty-record-error";
      statusMessageDefault = "Error: Cannot save an empty record.";
      statusType = "error";
      saving = false;
      return;
    }

    try {
      let res = await specimenService.saveCapturedRecord(recordPayload);
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
        
        setTimeout(() => {
          if (statusType === "success") {
            statusMessageKey = "";
            statusMessageDefault = "";
          }
        }, 3000);
      }
    } catch (err) {
      statusMessageKey = "";
      statusMessageDefault = `Error: ${err instanceof Error ? err.message : String(err)}`;
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
      islandGroup: "",
      island: "",
      locality: "",
      verbatimCoordinates: "",
      decimalLatitude: "",
      decimalLongitude: "",
      locationNotes: "",
      verbatimLocality: "",
      verbatimElevation: "",
      gridReference: "",
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
      cultivated: false,
      recordSource: "manual"
    };
    isGridCalculated = false;
    activeRecord = null;
    statusMessageKey = "";
    statusMessageDefault = "";
    duplicateSuggestions = [];
    titleCasedStates = getInitialTrackingState();
    eventDateLanguage = "EN";
    nameValidationStatus = "unchecked";
  }

  function handleShowPreviousRecord() {
    if (!lastSavedRecord) return;
    
    form.id = lastSavedRecord.id;
    form.catalogNumber = lastSavedRecord.catalogNumber;
    form.duplicates = lastSavedRecord.duplicates;
    
    if (lastSavedRecord.recordedBy) {
      const parts = splitNames(lastSavedRecord.recordedBy);
      form.recordedBy = parts[0] || "";
      form.additionalCollectors = parts.slice(1);
    } else {
      form.recordedBy = "";
      form.additionalCollectors = [];
    }
    
    form.recordNumber = lastSavedRecord.recordNumber;
    form.verbatimEventDate = lastSavedRecord.verbatimEventDate;
    form.year = lastSavedRecord.year !== null ? lastSavedRecord.year.toString() : "";
    form.month = lastSavedRecord.month !== null ? lastSavedRecord.month.toString() : "";
    form.day = lastSavedRecord.day !== null ? lastSavedRecord.day.toString() : "";
    
    form.country = lastSavedRecord.country;
    form.stateProvince = lastSavedRecord.stateProvince;
    form.county = lastSavedRecord.county;
    form.islandGroup = lastSavedRecord.islandGroup || "";
    form.island = lastSavedRecord.island || "";
    form.locality = lastSavedRecord.locality;
    form.verbatimCoordinates = lastSavedRecord.verbatimCoordinates;
    form.decimalLatitude = lastSavedRecord.decimalLatitude !== null ? lastSavedRecord.decimalLatitude.toString() : "";
    form.decimalLongitude = lastSavedRecord.decimalLongitude !== null ? lastSavedRecord.decimalLongitude.toString() : "";
    form.locationNotes = lastSavedRecord.locationNotes;
    form.verbatimLocality = lastSavedRecord.verbatimLocality;
    form.verbatimElevation = lastSavedRecord.verbatimElevation;
    form.gridReference = lastSavedRecord.gridReference || "";
    isGridCalculated = false;
    form.habitat = lastSavedRecord.habitat;
    form.identificationQualifier = lastSavedRecord.identificationQualifier;
    form.scientificName = lastSavedRecord.scientificName;
    form.taxonID = lastSavedRecord.taxonID;
    if (form.scientificName) {
      nameValidationStatus = form.taxonID ? "valid" : "invalid";
    } else {
      nameValidationStatus = "unchecked";
    }
    form.typeStatus = lastSavedRecord.typeStatus;
    
    if (lastSavedRecord.identifiedBy) {
      form.identifiedBy = splitNames(lastSavedRecord.identifiedBy);
    } else {
      form.identifiedBy = [];
    }
    
    form.yearIdentified = lastSavedRecord.yearIdentified !== null ? lastSavedRecord.yearIdentified.toString() : "";
    form.monthIdentified = lastSavedRecord.monthIdentified !== null ? lastSavedRecord.monthIdentified.toString() : "";
    form.dayIdentified = lastSavedRecord.dayIdentified !== null ? lastSavedRecord.dayIdentified.toString() : "";
    form.identificationRemarks = lastSavedRecord.identificationRemarks;
    form.occurrenceRemarks = lastSavedRecord.occurrenceRemarks;
    form.fieldNotes = lastSavedRecord.fieldNotes;
    form.cultivated = !!lastSavedRecord.cultivated;
    
    statusMessageKey = "showing-previous-record-info";
    statusMessageDefault = "Showing previously saved record.";
    statusType = "success";
  }

  /* --- Section States & Functions --- */

  // --- Date and Collector Section ---
  /** @type {string} */
  let eventDateLanguage = $state("EN");
  /** @type {string[]} */
  let collectorSuggestions = $state([]);
  /** @type {string[]} */
  let additionalCollectorsSuggestions = $state([]);

  /**
   * Parses the verbatim date string using any-date-parser.
   * @returns {void}
   */
  function parseVerbatimDate() {
    let dateStr = form.verbatimEventDate.trim();
    if (!dateStr) return;
    
    // Remove apostrophe like chars so we can parse this more successfully
    dateStr = dateStr
      // normalize apostrophe-like chars to plain apostrophe
      .replace(/[\u2018\u2019\u02BC\u0060]/g, "'")
      // remove apostrophe before 2-digit year after start, space, slash, dash, comma
      .replace(/(^|[\s,/-])'(\d{2})(?=$|[\s,./-])/g, "$1$2");

    let defaultLocale = workspaceStore.preferredDateFormat || "en-US";
    let locale = eventDateLanguage === "FR" ? "fr-FR" : eventDateLanguage === "PT" ? "pt-PT" : defaultLocale;

    let { day, month, year } = parser.attempt(dateStr, locale);
    form.day = day ? String(day) : "";
    form.month = month ? String(month) : "";
    form.year = year ? String(year) : "";
  }

  $effect(() => {
    if (eventDateLanguage) {
      parseVerbatimDate();
    }
  });

  /**
   * Validates the collection date and clears date validation errors if valid.
   * @returns {void}
   */
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

  /**
   * Autocompletes the primary collector based on input string.
   * @param {string} val
   * @returns {Promise<void>}
   */
  async function handleCollectorInput(val) {
    if (val.trim().length < 2) {
      collectorSuggestions = [];
      return;
    }
    try {
      const res = await agentService.autocompleteAgent(val);
      collectorSuggestions = res.filter(name => agentFilter(name) && !form.additionalCollectors.includes(name));
    } catch (e) {
      console.error(e);
    }
  }

  /**
   * Autocompletes additional collectors based on input string.
   * @param {string} val
   * @returns {Promise<void>}
   */
  async function handleAdditionalCollectorsInput(val) {
    if (val.trim().length < 2) {
      additionalCollectorsSuggestions = [];
      return;
    }
    try {
      const res = await agentService.autocompleteAgent(val);
      additionalCollectorsSuggestions = res.filter(name => 
        agentFilter(name) &&
        name !== form.recordedBy && 
        !form.additionalCollectors.includes(name)
      );
    } catch (e) {
      console.error(e);
    }
  }

  // --- Geographic Locality Section ---
  /** @type {string[]} */
  let countrySuggestions = $state([]);
  /** @type {string[]} */
  let stateProvinceSuggestions = $state([]);
  /** @type {string[]} */
  let countySuggestions = $state([]);
  /** @type {string[]} */
  let municipalitySuggestions = $state([]);
  /** @type {string[]} */
  let islandGroupSuggestions = $state([]);
  /** @type {string[]} */
  let islandSuggestions = $state([]);
  let showLocalitySearchDialog = $state(false);
  /** @type {string} */
  let dialogSearchQuery = $state("");
  /** @type {any[]} */
  let dialogLocalitySuggestions = $state([]);
  /** @type {any} */
  let dialogSearchInputRef = $state(null);

  let isAnyGeoPopulated = $derived(
    !!((form.country && form.country.trim().length > 0) ||
       (form.stateProvince && form.stateProvince.trim().length > 0) ||
       (form.county && form.county.trim().length > 0) ||
       (form.islandGroup && form.islandGroup.trim().length > 0) ||
       (form.island && form.island.trim().length > 0))
  );

  /** @type {HTMLTextAreaElement|null} */
  let verbatimLocalityRef = $state(null);
  let verbatimLocalityCopied = $state(false);
  /** @type {any} */
  let copyTimeoutId = null;

  async function handleCopyVerbatimLocality() {
    const success = await copySelectedOrValue(verbatimLocalityRef, form.verbatimLocality);
    if (success) {
      verbatimLocalityCopied = true;
      if (copyTimeoutId) clearTimeout(copyTimeoutId);
      copyTimeoutId = setTimeout(() => {
        verbatimLocalityCopied = false;
      }, 2000);
    }
  }

  /** @type {HTMLInputElement|null} */
  let islandGroupInputRef = $state(null);
  /** @type {HTMLInputElement|null} */
  let islandInputRef = $state(null);

  async function handlePasteIslandGroup() {
    const res = await pasteAtCursor(islandGroupInputRef, form.islandGroup);
    if (res) {
      form.islandGroup = res.newValue;
      setTimeout(() => {
        if (islandGroupInputRef) {
          islandGroupInputRef.focus();
          islandGroupInputRef.setSelectionRange(res.newCursorPos, res.newCursorPos);
        }
      }, 0);
    }
  }

  async function handlePasteIsland() {
    const res = await pasteAtCursor(islandInputRef, form.island);
    if (res) {
      form.island = res.newValue;
      setTimeout(() => {
        if (islandInputRef) {
          islandInputRef.focus();
          islandInputRef.setSelectionRange(res.newCursorPos, res.newCursorPos);
        }
      }, 0);
    }
  }

  /** @type {HTMLTextAreaElement|null} */
  let localityInputRef = $state(null);
  let localityCopied = $state(false);
  /** @type {any} */
  let localityCopyTimeout = null;

  async function handleCopyLocality() {
    const success = await copySelectedOrValue(localityInputRef, form.locality);
    if (success) {
      localityCopied = true;
      if (localityCopyTimeout) clearTimeout(localityCopyTimeout);
      localityCopyTimeout = setTimeout(() => {
        localityCopied = false;
      }, 2000);
    }
  }

  async function handlePasteLocality() {
    const res = await pasteAtCursor(localityInputRef, form.locality);
    if (res) {
      form.locality = res.newValue;
      setTimeout(() => {
        if (localityInputRef) {
          localityInputRef.focus();
          localityInputRef.setSelectionRange(res.newCursorPos, res.newCursorPos);
        }
      }, 0);
    }
  }

  /** @type {HTMLTextAreaElement|null} */
  let locationNotesRef = $state(null);
  let locationNotesCopied = $state(false);
  /** @type {any} */
  let locationNotesCopyTimeout = null;

  async function handleCopyLocationNotes() {
    const success = await copySelectedOrValue(locationNotesRef, form.locationNotes);
    if (success) {
      locationNotesCopied = true;
      if (locationNotesCopyTimeout) clearTimeout(locationNotesCopyTimeout);
      locationNotesCopyTimeout = setTimeout(() => {
        locationNotesCopied = false;
      }, 2000);
    }
  }

  async function handlePasteLocationNotes() {
    const res = await pasteAtCursor(locationNotesRef, form.locationNotes);
    if (res) {
      form.locationNotes = res.newValue;
      setTimeout(() => {
        if (locationNotesRef) {
          locationNotesRef.focus();
          locationNotesRef.setSelectionRange(res.newCursorPos, res.newCursorPos);
        }
      }, 0);
    }
  }

  function handleCoordinatesBlur() {
    if (form.verbatimCoordinates.trim() === "") {
      coordinatesError = false;
      form.decimalLatitude = "";
      form.decimalLongitude = "";
      if (isGridCalculated) {
        form.gridReference = "";
        isGridCalculated = false;
      }
      return;
    }
    try {
      const result = convert(form.verbatimCoordinates);
      if (result && result.decimalLatitude !== undefined && result.decimalLongitude !== undefined) {
        coordinatesError = false;
        form.decimalLatitude = String(result.decimalLatitude);
        form.decimalLongitude = String(result.decimalLongitude);

        // Calculate QDS if not populated
        if (!form.gridReference || form.gridReference.trim() === "") {
          try {
            const calculated = coordsToQDS(form.decimalLatitude, form.decimalLongitude);
            if (calculated) {
              form.gridReference = calculated;
              isGridCalculated = true;
            }
          } catch (err) {
            console.error("Error calculating QDS from coordinates:", err);
          }
        }
      }
    } catch (e) {
      coordinatesError = true;
      if (isGridCalculated) {
        form.gridReference = "";
        isGridCalculated = false;
      }
    }
  }

  function onCountryChanged() {
    form.stateProvince = "";
    form.county = "";
    form.islandGroup = "";
    form.island = "";
    stateProvinceSuggestions = [];
    countySuggestions = [];
    municipalitySuggestions = [];
    islandGroupSuggestions = [];
    islandSuggestions = [];
  }

  function onStateProvinceChanged() {
    form.county = "";
    countySuggestions = [];
    municipalitySuggestions = [];
  }

  function onIslandGroupChanged() {
    form.island = "";
    islandSuggestions = [];
  }

  /**
   * @param {string} val
   */
  async function handleCountryInput(val) {
    onCountryChanged();
    if (!val || val.trim().length === 0) {
      countrySuggestions = [];
      return;
    }
    try {
      countrySuggestions = await geographyService.autocompleteGeography("country", val, "", "", "");
    } catch (e) {
      console.error(e);
    }
  }

  /**
   * @param {string} val
   */
  async function handleStateProvinceInput(val) {
    onStateProvinceChanged();
    if (!val || val.trim().length === 0) {
      stateProvinceSuggestions = [];
      return;
    }
    try {
      stateProvinceSuggestions = await geographyService.autocompleteGeography("stateProvince", val, form.country, "", "");
    } catch (e) {
      console.error(e);
    }
  }

  /**
   * @param {string} val
   */
  async function handleCountyInput(val) {
    if (!val || val.trim().length === 0) {
      countySuggestions = [];
      return;
    }
    try {
      countySuggestions = await geographyService.autocompleteGeography("county", val, form.country, form.stateProvince, "");
    } catch (e) {
      console.error(e);
    }
  }

  /**
   * @param {string} val
   */
  async function handleIslandGroupInput(val) {
    onIslandGroupChanged();
    if (!val || val.trim().length === 0) {
      islandGroupSuggestions = [];
      return;
    }
    try {
      islandGroupSuggestions = await geographyService.autocompleteGeography("islandGroup", val, form.country, "", "");
    } catch (e) {
      console.error(e);
    }
  }

  /**
   * @param {string} val
   */
  async function handleIslandInput(val) {
    if (!val || val.trim().length === 0) {
      islandSuggestions = [];
      return;
    }
    try {
      islandSuggestions = await geographyService.autocompleteGeography("island", val, form.country, form.islandGroup, "");
    } catch (e) {
      console.error(e);
    }
  }

  function handleSearchLocality() {
    showLocalitySearchDialog = true;
    dialogSearchQuery = "";
    dialogLocalitySuggestions = [];
    setTimeout(() => {
      if (dialogSearchInputRef) {
        dialogSearchInputRef.focus();
      }
    }, 50);
  }

  /**
   * @param {string} val
   */
  async function handleDialogLocalityInput(val) {
    if (val.trim().length < 2) {
      dialogLocalitySuggestions = [];
      return;
    }
    try {
      dialogLocalitySuggestions = await geographyService.autocompleteLocality(val);
    } catch (e) {
      console.error(e);
    }
  }

  /**
   * @param {any} suggestion
   */
  function handleDialogLocalitySelect(suggestion) {
    if (!suggestion || typeof suggestion === "string") return;

    if (suggestion.locality) {
      form.locality = suggestion.locality;
    }
    if (suggestion.country) {
      form.country = suggestion.country;
    }
    if (suggestion.stateProvince) {
      form.stateProvince = suggestion.stateProvince;
    }
    if (suggestion.county) {
      form.county = suggestion.county;
    }
    if (suggestion.locationNotes) {
      form.locationNotes = suggestion.locationNotes;
    }
    if (suggestion.verbatimCoordinates) {
      form.verbatimCoordinates = suggestion.verbatimCoordinates;
      form.decimalLatitude = suggestion.decimalLatitude !== null && suggestion.decimalLatitude !== undefined ? String(suggestion.decimalLatitude) : "";
      form.decimalLongitude = suggestion.decimalLongitude !== null && suggestion.decimalLongitude !== undefined ? String(suggestion.decimalLongitude) : "";
      coordinatesError = false;

      // Calculate QDS if not populated
      if (!form.gridReference || form.gridReference.trim() === "") {
        try {
          const calculated = coordsToQDS(form.decimalLatitude, form.decimalLongitude);
          if (calculated) {
            form.gridReference = calculated;
            isGridCalculated = true;
          }
        } catch (err) {
          console.error("Error calculating QDS from suggestion coordinates:", err);
        }
      }
    }

    showLocalitySearchDialog = false;
    dialogSearchQuery = "";
    dialogLocalitySuggestions = [];
  }

  /**
   * @param {string} field
   */
  async function triggerGeoAutocomplete(field) {
    try {
      let results;
      if (field === "islandGroup") {
        results = await geographyService.autocompleteGeography("islandGroup", "", form.country, "", "");
        islandGroupSuggestions = results;
      } else if (field === "island") {
        results = await geographyService.autocompleteGeography("island", "", form.country, form.islandGroup, "");
        islandSuggestions = results;
      } else {
        results = await geographyService.autocompleteGeography(field, "", form.country, form.stateProvince, form.county);
        if (field === "stateProvince") stateProvinceSuggestions = results;
        else if (field === "county") countySuggestions = results;
      }
    } catch (e) {
      console.error(e);
    }
  }

  function handleStateProvinceFocus() {
    if (form.stateProvince.trim() === "") {
      triggerGeoAutocomplete("stateProvince");
    }
  }

  function handleCountyFocus() {
    if (form.county.trim() === "") {
      triggerGeoAutocomplete("county");
    }
  }

  function handleIslandGroupFocus() {
    if (form.islandGroup.trim() === "") {
      triggerGeoAutocomplete("islandGroup");
    }
  }

  function handleIslandFocus() {
    if (form.island.trim() === "") {
      triggerGeoAutocomplete("island");
    }
  }

  onDestroy(() => {
    if (copyTimeoutId) clearTimeout(copyTimeoutId);
    if (localityCopyTimeout) clearTimeout(localityCopyTimeout);
    if (locationNotesCopyTimeout) clearTimeout(locationNotesCopyTimeout);
  });

  // --- Taxonomy Section ---
  /** @type {any[]} */
  let taxonSuggestions = $state([]);
  /** @type {string} */
  let nameValidationStatus = $state("unchecked"); // "unchecked", "valid", "invalid"
  /** @type {string[]} */
  let typeStatusSuggestions = $state([]);

  const typeStatuses = [
    "holotype", "isotype", "syntype", "lectotype", "neotype", "paratype", "paralectotype", "epitype", "isolectotype", "isosyntype", "isoneotype", "type", "topotype"
  ];

  /**
   * @param {string} val
   */
  async function handleTaxonInput(val) {
    form.taxonID = "";
    nameValidationStatus = "unchecked";

    if (val.trim().length < 2) {
      taxonSuggestions = [];
      return;
    }
    try {
      taxonSuggestions = await taxonomyService.autocompleteScientificName(val);
    } catch (e) {
      console.error(e);
    }
  }

  /**
   * @param {any} sug
   */
  function handleTaxonSelect(sug) {
    form.scientificName = sug.scientificName || "";
    form.taxonID = sug.taxonID || "";
    nameValidationStatus = sug.taxonID ? "valid" : "invalid";
  }

  async function handleTaxonBlur() {
    const trimmed = form.scientificName.trim();
    if (!trimmed) {
      form.taxonID = "";
      nameValidationStatus = "unchecked";
      return;
    }
    try {
      const plantNameId = await taxonomyService.lookupTaxonByName(trimmed);
      if (plantNameId) {
        form.taxonID = plantNameId;
        nameValidationStatus = "valid";
      } else {
        form.taxonID = "";
        nameValidationStatus = "invalid";
      }
    } catch (err) {
      console.error("Error looking up taxon by name:", err);
      form.taxonID = "";
      nameValidationStatus = "invalid";
    }
  }

  /**
   * @param {string} val
   */
  function handleTypeStatusInput(val) {
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

  // --- Determination Section ---
  /** @type {string[]} */
  let identifiedBySuggestions = $state([]);

  /**
   * @param {string} val
   */
  async function handleIdentifiedByInput(val) {
    if (val.trim().length < 2) {
      identifiedBySuggestions = [];
      return;
    }
    try {
      const res = await agentService.autocompleteAgent(val);
      identifiedBySuggestions = res.filter(name => agentFilter(name) && !form.identifiedBy.includes(name));
    } catch (e) {
      console.error(e);
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

  /**
   * Pastes a Darwin Core JSON object from the clipboard and maps it to the form fields.
   * Displays warning if there are any unmapped fields.
   * @returns {Promise<void>}
   */
  async function handlePasteJson() {
    statusMessageKey = "";
    statusMessageDefault = "";
    statusType = "";
    isGridCalculated = false;

    try {
      const text = await navigator.clipboard.readText();
      if (!text || !text.trim()) {
        statusMessageKey = "empty-clipboard-error";
        statusMessageDefault = "Error: Clipboard is empty.";
        statusType = "error";
        return;
      }

      let data;
      try {
        data = JSON.parse(text.trim());
      } catch (err) {
        statusMessageKey = "invalid-json-error";
        statusMessageDefault = "Error: Clipboard content is not valid JSON.";
        statusType = "error";
        return;
      }

      if (Array.isArray(data)) {
        if (data.length > 0) {
          data = data[0];
        } else {
          statusMessageKey = "empty-array-error";
          statusMessageDefault = "Error: Paste array is empty.";
          statusType = "error";
          return;
        }
      }

      if (typeof data !== "object" || data === null) {
        statusMessageKey = "invalid-json-object-error";
        statusMessageDefault = "Error: Paste content must be a JSON object.";
        statusType = "error";
        return;
      }

      const mappedKeys = new Set();
      form.recordSource = "json";

      // 1. catalogNumber
      if ("catalogNumber" in data) {
        form.catalogNumber = data.catalogNumber !== null && data.catalogNumber !== undefined ? String(data.catalogNumber) : "";
        mappedKeys.add("catalogNumber");
      }

      // 2. duplicates
      if ("duplicates" in data) {
        form.duplicates = data.duplicates !== null && data.duplicates !== undefined ? String(data.duplicates) : "";
        mappedKeys.add("duplicates");
      }

      // 3. recordedBy
      if ("recordedBy" in data) {
        const val = data.recordedBy;
        let names = [];
        if (Array.isArray(val)) {
          names = val;
        } else if (val !== null && val !== undefined) {
          names = splitNames(String(val));
        }
        form.recordedBy = names[0] || "";
        form.additionalCollectors = names.slice(1);
        mappedKeys.add("recordedBy");
      }

      // 4. additionalCollectors
      if ("additionalCollectors" in data) {
        const val = data.additionalCollectors;
        if (Array.isArray(val)) {
          form.additionalCollectors = val.map(String);
        } else if (val !== null && val !== undefined) {
          form.additionalCollectors = splitNames(String(val));
        }
        mappedKeys.add("additionalCollectors");
      }

      // 5. recordNumber
      if ("recordNumber" in data) {
        form.recordNumber = data.recordNumber !== null && data.recordNumber !== undefined ? String(data.recordNumber) : "";
        mappedKeys.add("recordNumber");
      }

      // 6. verbatimEventDate / eventDate
      if ("verbatimEventDate" in data) {
        form.verbatimEventDate = data.verbatimEventDate !== null && data.verbatimEventDate !== undefined ? String(data.verbatimEventDate) : "";
        mappedKeys.add("verbatimEventDate");
      } else if ("eventDate" in data) {
        form.verbatimEventDate = data.eventDate !== null && data.eventDate !== undefined ? String(data.eventDate) : "";
        mappedKeys.add("eventDate");
      }

      // 7. year, month, day
      let hasExplicitDateFields = false;
      if ("year" in data) {
        form.year = data.year !== null && data.year !== undefined ? String(data.year) : "";
        mappedKeys.add("year");
        hasExplicitDateFields = true;
      }
      if ("month" in data) {
        form.month = data.month !== null && data.month !== undefined ? String(data.month) : "";
        mappedKeys.add("month");
        hasExplicitDateFields = true;
      }
      if ("day" in data) {
        form.day = data.day !== null && data.day !== undefined ? String(data.day) : "";
        mappedKeys.add("day");
        hasExplicitDateFields = true;
      }
      if (!hasExplicitDateFields && form.verbatimEventDate) {
        parseVerbatimDate();
      }

      // 8. Geo fields
      if ("country" in data) {
        form.country = data.country !== null && data.country !== undefined ? String(data.country) : "";
        mappedKeys.add("country");
      }
      if ("stateProvince" in data) {
        form.stateProvince = data.stateProvince !== null && data.stateProvince !== undefined ? String(data.stateProvince) : "";
        mappedKeys.add("stateProvince");
      }
      if ("county" in data) {
        form.county = data.county !== null && data.county !== undefined ? String(data.county) : "";
        mappedKeys.add("county");
      }
      if ("islandGroup" in data) {
        form.islandGroup = data.islandGroup !== null && data.islandGroup !== undefined ? String(data.islandGroup) : "";
        mappedKeys.add("islandGroup");
      }
      if ("island" in data) {
        form.island = data.island !== null && data.island !== undefined ? String(data.island) : "";
        mappedKeys.add("island");
      }
      if ("locality" in data) {
        form.locality = data.locality !== null && data.locality !== undefined ? String(data.locality) : "";
        mappedKeys.add("locality");
      }
      if ("verbatimLocality" in data) {
        form.verbatimLocality = data.verbatimLocality !== null && data.verbatimLocality !== undefined ? String(data.verbatimLocality) : "";
        mappedKeys.add("verbatimLocality");
      }

      // 8.5 gridReference
      if ("gridReference" in data && data.gridReference !== null && data.gridReference !== undefined && String(data.gridReference).trim() !== "") {
        form.gridReference = String(data.gridReference);
        mappedKeys.add("gridReference");
      } else if ("grid" in data && data.grid !== null && data.grid !== undefined && String(data.grid).trim() !== "") {
        form.gridReference = String(data.grid);
        mappedKeys.add("grid");
      } else {
        form.gridReference = "";
        if ("gridReference" in data) mappedKeys.add("gridReference");
        if ("grid" in data) mappedKeys.add("grid");
      }

      // 9. verbatimCoordinates & decimalLatitude/decimalLongitude
      if ("verbatimCoordinates" in data) {
        form.verbatimCoordinates = data.verbatimCoordinates !== null && data.verbatimCoordinates !== undefined ? String(data.verbatimCoordinates) : "";
        mappedKeys.add("verbatimCoordinates");
        handleCoordinatesBlur();
      } else {
        let hasLat = "decimalLatitude" in data;
        let hasLon = "decimalLongitude" in data;
        if (hasLat) {
          form.decimalLatitude = data.decimalLatitude !== null && data.decimalLatitude !== undefined ? String(data.decimalLatitude) : "";
          mappedKeys.add("decimalLatitude");
        }
        if (hasLon) {
          form.decimalLongitude = data.decimalLongitude !== null && data.decimalLongitude !== undefined ? String(data.decimalLongitude) : "";
          mappedKeys.add("decimalLongitude");
        }
        if (hasLat && hasLon && form.decimalLatitude && form.decimalLongitude) {
          form.verbatimCoordinates = `${form.decimalLatitude}, ${form.decimalLongitude}`;
          coordinatesError = false;
          // Calculate QDS if not populated
          if (!form.gridReference || form.gridReference.trim() === "") {
            try {
              const calculated = coordsToQDS(form.decimalLatitude, form.decimalLongitude);
              if (calculated) {
                form.gridReference = calculated;
                isGridCalculated = true;
              }
            } catch (err) {
              console.error("Error calculating QDS from pasted coordinates:", err);
            }
          }
        }
      }

      // 10. locationRemarks / locationNotes
      if ("locationRemarks" in data) {
        form.locationNotes = data.locationRemarks !== null && data.locationRemarks !== undefined ? String(data.locationRemarks) : "";
        mappedKeys.add("locationRemarks");
      } else if ("locationNotes" in data) {
        form.locationNotes = data.locationNotes !== null && data.locationNotes !== undefined ? String(data.locationNotes) : "";
        mappedKeys.add("locationNotes");
      }

      // 11. verbatimElevation / elevation
      if ("verbatimElevation" in data) {
        form.verbatimElevation = data.verbatimElevation !== null && data.verbatimElevation !== undefined ? String(data.verbatimElevation) : "";
        mappedKeys.add("verbatimElevation");
      } else if ("elevation" in data) {
        form.verbatimElevation = data.elevation !== null && data.elevation !== undefined ? String(data.elevation) : "";
        mappedKeys.add("elevation");
      }

      // 12. habitat
      if ("habitat" in data) {
        form.habitat = data.habitat !== null && data.habitat !== undefined ? String(data.habitat) : "";
        mappedKeys.add("habitat");
      }

      // 13. identificationQualifier
      if ("identificationQualifier" in data) {
        form.identificationQualifier = data.identificationQualifier !== null && data.identificationQualifier !== undefined ? String(data.identificationQualifier) : "";
        mappedKeys.add("identificationQualifier");
      }

      // 14. scientificName & taxonomy lookup
      if ("scientificName" in data) {
        form.scientificName = data.scientificName !== null && data.scientificName !== undefined ? String(data.scientificName) : "";
        mappedKeys.add("scientificName");
        handleTaxonBlur();
      }

      // 15. taxonID
      if ("taxonID" in data) {
        form.taxonID = data.taxonID !== null && data.taxonID !== undefined ? String(data.taxonID) : "";
        mappedKeys.add("taxonID");
      }

      // 16. typeStatus
      if ("typeStatus" in data) {
        form.typeStatus = data.typeStatus !== null && data.typeStatus !== undefined ? String(data.typeStatus) : "";
        mappedKeys.add("typeStatus");
      }

      // 17. identifiedBy
      if ("identifiedBy" in data) {
        const val = data.identifiedBy;
        let names = [];
        if (Array.isArray(val)) {
          names = val;
        } else if (val !== null && val !== undefined) {
          names = splitNames(String(val));
        }
        form.identifiedBy = names;
        mappedKeys.add("identifiedBy");
      }

      // 18. yearIdentified, monthIdentified, dayIdentified, dateIdentified
      let hasExplicitIdDateFields = false;
      if ("yearIdentified" in data) {
        form.yearIdentified = data.yearIdentified !== null && data.yearIdentified !== undefined ? String(data.yearIdentified) : "";
        mappedKeys.add("yearIdentified");
        hasExplicitIdDateFields = true;
      }
      if ("monthIdentified" in data) {
        form.monthIdentified = data.monthIdentified !== null && data.monthIdentified !== undefined ? String(data.monthIdentified) : "";
        mappedKeys.add("monthIdentified");
        hasExplicitIdDateFields = true;
      }
      if ("dayIdentified" in data) {
        form.dayIdentified = data.dayIdentified !== null && data.dayIdentified !== undefined ? String(data.dayIdentified) : "";
        mappedKeys.add("dayIdentified");
        hasExplicitIdDateFields = true;
      }
      if (!hasExplicitIdDateFields && "dateIdentified" in data) {
        const val = data.dateIdentified;
        mappedKeys.add("dateIdentified");
        if (val !== null && val !== undefined && String(val).trim()) {
          const { day, month, year } = parser.attempt(String(val));
          form.dayIdentified = day ? String(day) : "";
          form.monthIdentified = month ? String(month) : "";
          form.yearIdentified = year ? String(year) : "";
        }
      }

      // 19. identificationRemarks
      if ("identificationRemarks" in data) {
        form.identificationRemarks = data.identificationRemarks !== null && data.identificationRemarks !== undefined ? String(data.identificationRemarks) : "";
        mappedKeys.add("identificationRemarks");
      }

      // 20. occurrenceRemarks
      if ("occurrenceRemarks" in data) {
        form.occurrenceRemarks = data.occurrenceRemarks !== null && data.occurrenceRemarks !== undefined ? String(data.occurrenceRemarks) : "";
        mappedKeys.add("occurrenceRemarks");
      }

      // 21. fieldNotes
      if ("fieldNotes" in data) {
        form.fieldNotes = data.fieldNotes !== null && data.fieldNotes !== undefined ? String(data.fieldNotes) : "";
        mappedKeys.add("fieldNotes");
      }

      // 22. cultivated
      if ("cultivated" in data) {
        const val = data.cultivated;
        if (typeof val === "boolean") {
          form.cultivated = val;
        } else if (typeof val === "string") {
          const lower = val.toLowerCase();
          form.cultivated = lower === "true" || lower === "yes" || lower === "cultivated";
        } else {
          form.cultivated = Boolean(val);
        }
        mappedKeys.add("cultivated");
      }

      // Determine unmapped keys
      const ignoredKeys = new Set(["id", "occurrenceID", "@context", "type", "modified"]);
      const allKeys = Object.keys(data);
      const unmapped = allKeys.filter(k => !mappedKeys.has(k) && !ignoredKeys.has(k));

      if (unmapped.length > 0) {
        statusMessageKey = "";
        statusMessageDefault = "Notice: The following JSON fields were not mapped: " + unmapped.join(", ");
        statusType = "success";
      } else {
        statusMessageKey = "";
        statusMessageDefault = "";
        statusType = "";
      }
    } catch (err) {
      statusMessageKey = "";
      statusMessageDefault = "Error pasting JSON: " + (err instanceof Error ? err.message : String(err));
      statusType = "error";
    }
  }

  function handleGlobalKeyDown(/** @type {KeyboardEvent} */ e) {
    if (e.ctrlKey && e.key.toLowerCase() === "s") {
      e.preventDefault();
      handleSave(null);
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
      <h2 data-i18n-key={form.id ? "edit-specimen-heading" : "capture-specimen-heading"} class="text-sm font-bold text-slate-800 uppercase tracking-wide">
        {form.id ? t("edit-specimen-heading", "Edit Captured Specimen") : t("capture-specimen-heading", "Capture New Specimen")}
      </h2>
      {#if form.id}
        <span data-i18n-key="saved-capture-badge" class="text-[9px] bg-indigo-100 text-indigo-800 font-bold uppercase tracking-wider px-1.5 py-0.5">{t("saved-capture-badge", "SAVED CAPTURE")}</span>
      {:else}
        <span data-i18n-key="new-form-badge" class="text-[9px] bg-emerald-100 text-emerald-800 font-bold uppercase tracking-wider px-1.5 py-0.5">{t("new-form-badge", "NEW FORM")}</span>
      {/if}
    </div>
    <div class="flex gap-2 items-center">
      <button
        id="paste-json-button"
        class="flex items-center gap-0 cursor-pointer disabled:cursor-default disabled:bg-slate-200 disabled:text-slate-400 px-2 py-2 text-[10px] font-bold tracking-wider bg-slate-700 text-white hover:bg-slate-800 transition-colors rounded-none"
        disabled={Boolean(form.id) || saving}
        onclick={handlePasteJson}
        title={t("paste-json-tooltip", "Paste DwC JSON")}
      >
        <span class="w-3 h-3 inline-block mr-1">
          <PasteIcon  />
        </span>
        JSON
      </button>
      <span data-i18n-key="save-shortcut-desc" class="text-[10px] text-slate-400 font-semibold uppercase">{t("save-shortcut-desc", "Shortcut: Ctrl+S to save")}</span>
    </div>
  </div>

  <!-- Form Fields -->
  <form bind:this={formRef} onsubmit={handleSave} class="flex-1 overflow-y-auto p-4 space-y-4">
    
    <!-- Row 1: Home Herbarium (read-only), catalogNumber, duplicates -->
    <div class="grid grid-cols-12 gap-3">
      <div class="col-span-3">
        <label for="capture-collectionCode" data-i18n-key="home-herbarium-label" class="block text-xs font-semibold text-slate-655 uppercase tracking-wider mb-1">{t("home-herbarium-label", "Home Herbarium")}</label>
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
          autocomplete="off"
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

    <!-- Section: Collection Details -->
    <div class="space-y-3 pt-2">
      <h3 data-i18n-key="collection-event-heading" class="text-[10px] font-bold text-slate-400 uppercase tracking-wider border-b border-slate-100 pb-1">{t("collection-event-heading", "Collection Details")}</h3>
      
      <!-- Primary Collector, Collector Number, Date -->
      <div class="grid grid-cols-12 gap-3">
        <!-- Primary Collector -->
        <div class="col-span-3">
          <Autocomplete
            id="capture-recordedBy"
            label="Primary Collector"
            labelKey="recorded-by-label"
            placeholder="Partial eg 'Raza'"
            placeholderKey="names-field-placeholder"
            bind:value={form.recordedBy}
            suggestions={collectorSuggestions}
            oninput={handleCollectorInput}
            delay={300}
            promptNewAgent={true}
          />
        </div>
  
        <!-- Collector Number -->
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
        
        <!-- Verbatim Collection Date -->
        <div class="col-span-3">
          <label for="capture-verbatimEventDate" data-i18n-key="verbatim-event-date-label" class="block text-xs font-semibold text-slate-655 uppercase tracking-wider mb-1">
            <div class="flex justify-between">
              <span>{t("verbatim-event-date-label", "Verbatim Date")}</span>
              <div class="flex items-center divide-x divide-slate-300 select-none">
                <button
                  type="button"
                  onclick={() => eventDateLanguage = "EN"}
                  class="px-1 py-0.5 text-[10px] font-bold tracking-wider hover:bg-slate-50 transition-colors cursor-pointer {eventDateLanguage === 'EN' ? 'bg-slate-800 text-white hover:bg-slate-800' : 'bg-white text-slate-400'}"
                >
                  EN
                </button>
                {#if currentLanguage === "EN" || currentLanguage === "FR" || currentLanguage === "MG"}
                  <button
                    type="button"
                    onclick={() => eventDateLanguage = "FR"}
                    class="px-1 py-0.5 text-[10px] font-bold tracking-wider hover:bg-slate-50 transition-colors cursor-pointer {eventDateLanguage === 'FR' ? 'bg-slate-800 text-white hover:bg-slate-800' : 'bg-white text-slate-400'}"
                  >
                    FR
                  </button>
                {/if}
                {#if currentLanguage === "PT"}
                  <button
                    type="button"
                    onclick={() => eventDateLanguage = "PT"}
                    class="px-1 py-0.5 text-[10px] font-bold tracking-wider hover:bg-slate-50 transition-colors cursor-pointer {eventDateLanguage === 'PT' ? 'bg-slate-800 text-white hover:bg-slate-800' : 'bg-white text-slate-400'}"
                  >
                    PT
                  </button>
                {/if}
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
  
        <!-- Numeric Collection Date Fields -->
        <div class="col-span-4 flex gap-2">
          <!-- Day -->
          <div class="flex-1">
            <label for="capture-day" data-i18n-key="day-label" class="block text-xs font-semibold text-slate-655 uppercase tracking-wider mb-1">{t("day-label", "Day")}</label>
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
          <!-- Month -->
          <div class="flex-1">
            <label for="capture-month" data-i18n-key="month-label" class="block text-xs font-semibold text-slate-655 uppercase tracking-wider mb-1">{t("month-label", "Month")}</label>
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
          <!-- Year -->
          <div class="flex-1">
            <label for="capture-year" data-i18n-key="year-label" class="block text-xs font-semibold text-slate-655 uppercase tracking-wider mb-1">{t("year-label", "Year")}</label>
            <input
              id="capture-year"
              type="number"
              bind:value={form.year}
              onblur={handleCollectionDateBlur}
              class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-2 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
            />
          </div>
        </div>
      </div>
  
      <!-- Additional Collectors -->
      <div class="grid grid-cols-12 gap-3 pt-1">
        <div class="col-span-12">
          <MultiSelectAutocomplete
            id="capture-additionalCollectors"
            label="Additional Collectors"
            labelKey="add-collectors-label"
            placeholder="Type name and press Enter..."
            placeholderKey="names-field-placeholder"
            bind:selectedValues={form.additionalCollectors}
            suggestions={additionalCollectorsSuggestions}
            oninput={handleAdditionalCollectorsInput}
            delay={300}
          />
        </div>
      </div>

      <!-- Plant Description (fieldNotes) -->
      <div>
        <label for="capture-fieldNotes" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">{t("field-notes-label", "Field Notes")}</label>
        <div class="relative flex items-center">
          <textarea
            id="capture-fieldNotes"
            placeholder={t("field-notes-placeholder", "eg 'Shrub 2m tall with yellow flowers'")}
            bind:value={form.fieldNotes}
            rows="2"
            class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all pr-12"
          ></textarea>
          <div class="absolute right-5 flex items-center gap-1 z-100">
            {#if form.fieldNotes === titleCasedStates.fieldNotes.titleCased && titleCasedStates.fieldNotes.titleCased !== ""}
              <button
                type="button"
                onclick={() => undoTitleCaseField(form, titleCasedStates, "fieldNotes")}
                data-i18n-key="undo-title-case"
                title={t("undo-title-case", "Undo Casing")}
                class="w-6 h-6 p-1 text-slate-400 hover:text-slate-600 transition-colors cursor-pointer rounded hover:bg-slate-100"
                tabindex="-1"
              >
                <UndoIcon />
              </button>
            {:else}
              <button
                type="button"
                onclick={() => titleCaseField(form, titleCasedStates, "fieldNotes")}
                data-i18n-key="title-case-field-notes"
                title={t("title-case-field-notes", "Title case Field Notes")}
                class="w-6 h-6 p-1.5 text-slate-400 hover:text-slate-600 transition-colors cursor-pointer rounded hover:bg-slate-100"
                tabindex="-1"
              >
                <AaIcon />
              </button>
            {/if}
          </div>  
        </div>
      </div>

      <!-- General Notes (occurrenceRemarks) -->
      <div>
        <label for="capture-occurrenceRemarks" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">{t("occurrence-remarks-label", "General Notes")}</label>
        <div class="relative flex items-center">
          <textarea
            id="capture-occurrenceRemarks"
            placeholder={t("occurrence-remarks-placeholder", "eg 'Common in tapia forest'")}
            bind:value={form.occurrenceRemarks}
            rows="2"
            class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all pr-12"
          ></textarea>
          <div class="absolute right-5 flex items-center gap-1 z-100">

            {#if form.occurrenceRemarks === titleCasedStates.occurrenceRemarks.titleCased && titleCasedStates.occurrenceRemarks.titleCased !== ""}
              <button
                type="button"
                onclick={() => undoTitleCaseField(form, titleCasedStates, "occurrenceRemarks")}
                data-i18n-key="undo-title-case"
                title={t("undo-title-case", "Undo Casing")}
                class="w-6 h-6 p-1 text-slate-400 hover:text-slate-600 transition-colors cursor-pointer rounded hover:bg-slate-100"
                tabindex="-1"
              >
                <UndoIcon />
              </button>
            {:else}
              <button
                type="button"
                onclick={() => titleCaseField(form, titleCasedStates, "occurrenceRemarks")}
                data-i18n-key="title-case-occurrence-remarks"
                title={t("title-case-occurrence-remarks", "Title case General Notes")}
                class="w-6 h-6 p-1.5 text-slate-400 hover:text-slate-600 transition-colors cursor-pointer rounded hover:bg-slate-100"
                tabindex="-1"
              >
                <AaIcon />
              </button>
            {/if}
          </div>
        </div>
      </div>
    </div>

    <!-- Section: Geographic Locality -->
    <div class="space-y-3 pt-2">
      <h3 data-i18n-key="geographic-locality-heading" class="text-[10px] font-bold text-slate-400 uppercase tracking-wider border-b border-slate-100 pb-1">{t("geographic-locality-heading", "Geographic Locality")}</h3>
      
      <!-- Country and Admins -->
      <div class="grid grid-cols-3 gap-3">
        <!-- Country -->
        <div>
          <label for="capture-country" data-i18n-key="country-label" class="block text-xs font-semibold text-slate-655 uppercase tracking-wider mb-1">{t("country-label", "Country")}</label>
          <div class="relative flex items-center">
            <Autocomplete
              id="capture-country"
              label=""
              placeholder={isAnyGeoPopulated ? "" : placeholders.country}
              placeholderKey={isAnyGeoPopulated ? undefined : ""}
              bind:value={form.country}
              suggestions={countrySuggestions}
              oninput={handleCountryInput}
              delay={300}
            />
            {#if form.country === titleCasedStates.country.titleCased && titleCasedStates.country.titleCased !== ""}
              <button
                type="button"
                onclick={() => undoTitleCaseField(form, titleCasedStates, "country")}
                data-i18n-key="undo-title-case"
                title={t("undo-title-case", "Undo Casing")}
                class="absolute right-2 bottom-1.5 w-6 h-6 p-1 text-slate-400 hover:text-slate-600 transition-colors cursor-pointer rounded hover:bg-slate-100"
                tabindex="-1"
              >
                <UndoIcon />
              </button>
            {:else}
              <button
                type="button"
                onclick={() => titleCaseField(form, titleCasedStates, "country")}
                data-i18n-key="title-case"
                title={t("title-case", "Proper Case")}
                class="absolute right-2 bottom-1.5 w-6 h-6 p-1.5 text-slate-400 hover:text-slate-600 transition-colors cursor-pointer rounded hover:bg-slate-100"
                tabindex="-1"
              >
                <AaIcon />
              </button>
            {/if}
          </div>
        </div>

        <!-- Admin 2 -->
        <div>
          <label for="capture-stateProvince" data-i18n-key="state-province-label" class="block text-xs font-semibold text-slate-655 uppercase tracking-wider mb-1">
            {form.country && form.country.toLowerCase() === "madagascar" ? t("state-province-label", "Province") : t("state-province-label", "Admin 2")}
            <span data-i18n-key="state-province-sub" class="text-[70%]">{t("state-province-sub", "(state/prov/etc)")}</span>
          </label>
          <div class="relative flex items-center">
            <Autocomplete
              id="capture-stateProvince"
              label=""
              placeholder={isAnyGeoPopulated ? "" : placeholders.stateProvince}
              placeholderKey={isAnyGeoPopulated ? undefined : ""}
              bind:value={form.stateProvince}
              suggestions={stateProvinceSuggestions}
              oninput={handleStateProvinceInput}
              onfocus={handleStateProvinceFocus}
              delay={300}
            />
            {#if form.stateProvince === titleCasedStates.stateProvince.titleCased && titleCasedStates.stateProvince.titleCased !== ""}
              <button
                type="button"
                onclick={() => undoTitleCaseField(form, titleCasedStates, "stateProvince")}
                data-i18n-key="undo-title-case"
                title={t("undo-title-case", "Undo Casing")}
                class="absolute right-2 bottom-1.5 w-6 h-6 p-1 text-slate-400 hover:text-slate-600 transition-colors cursor-pointer rounded hover:bg-slate-100"
                tabindex="-1"
              >
                <UndoIcon />
              </button>
            {:else}
              <button
                type="button"
                onclick={() => titleCaseField(form, titleCasedStates, "stateProvince")}
                data-i18n-key="title-case"
                title={t("title-case", "Proper Case")}
                class="absolute right-2 bottom-1.5 w-6 h-6 p-1.5 text-slate-400 hover:text-slate-600 transition-colors cursor-pointer rounded hover:bg-slate-100"
                tabindex="-1"
              >
                <AaIcon />
              </button>
            {/if}
          </div>
        </div>

        <!-- Admin 3 -->
        <div>
          <label for="capture-county" data-i18n-key="county-label" class="block text-xs font-semibold text-slate-655 uppercase tracking-wider mb-1">
            {form.country && form.country.toLowerCase() === "madagascar" ? t("county-label", "Region") : t("county-label", "Admin 3")}
            <span data-i18n-key="county-sub" class="text-[70%]">{t("county-sub", "(county/etc)")}</span>

          </label>
          <div class="relative flex items-center">
            <Autocomplete
              id="capture-county"
              label=""
              placeholder={isAnyGeoPopulated ? "" : placeholders.county}
              placeholderKey={isAnyGeoPopulated ? undefined : ""}
              bind:value={form.county}
              suggestions={countySuggestions}
              oninput={handleCountyInput}
              onfocus={handleCountyFocus}
              delay={300}
            />
            {#if form.county === titleCasedStates.county.titleCased && titleCasedStates.county.titleCased !== ""}
              <button
                type="button"
                onclick={() => undoTitleCaseField(form, titleCasedStates, "county")}
                data-i18n-key="undo-title-case"
                title={t("undo-title-case", "Undo Casing")}
                class="absolute right-2 bottom-1.5 w-6 h-6 p-1 text-slate-400 hover:text-slate-600 transition-colors cursor-pointer rounded hover:bg-slate-100"
                tabindex="-1"
              >
                <UndoIcon />
              </button>
            {:else}
              <button
                type="button"
                onclick={() => titleCaseField(form, titleCasedStates, "county")}
                data-i18n-key="title-case"
                title={t("title-case", "Proper Case")}
                class="absolute right-2 bottom-1.5 w-6 h-6 p-1.5 text-slate-400 hover:text-slate-600 transition-colors cursor-pointer rounded hover:bg-slate-100"
                tabindex="-1"
              >
                <AaIcon />
              </button>
            {/if}
          </div>
        </div>
      </div>

      <!-- Island Group and Island -->
      {#if workspaceStore.includeIslands}
      <div class="grid grid-cols-3 gap-3">
        <!-- Island Group -->
        <div class="col-span-2">
          <label for="capture-islandGroup" data-i18n-key="island-group-label" class="block text-xs font-semibold text-slate-655 uppercase tracking-wider mb-1">{t("island-group-label", "Island Group")}</label>
          <div class="relative flex items-center">
            <Autocomplete
              id="capture-islandGroup"
              label=""
              placeholder={isAnyGeoPopulated ? "" : "eg Mascarenes"}
              placeholderKey={isAnyGeoPopulated ? undefined : "island-group-placeholder"}
              bind:value={form.islandGroup}
              suggestions={islandGroupSuggestions}
              oninput={handleIslandGroupInput}
              onfocus={handleIslandGroupFocus}
              delay={300}
              bind:inputRef={islandGroupInputRef}
              extraInputClass="pr-20"
            />
            <div class="absolute right-2 flex items-center gap-1 z-100">
              <button
                type="button"
                onclick={handlePasteIslandGroup}
                title={t("paste-island-group-title", "Paste Island Group")}
                class="w-6 h-6 p-1 text-slate-400 hover:text-slate-600 transition-colors cursor-pointer rounded hover:bg-slate-100"
                tabindex="-1"
              >
                <PasteIcon />
              </button>
              {#if form.islandGroup === titleCasedStates.islandGroup.titleCased && titleCasedStates.islandGroup.titleCased !== ""}
                <button
                  type="button"
                  onclick={() => undoTitleCaseField(form, titleCasedStates, "islandGroup")}
                  data-i18n-key="undo-title-case"
                  title={t("undo-title-case", "Undo Casing")}
                  class="w-6 h-6 p-1 text-slate-400 hover:text-slate-600 transition-colors cursor-pointer rounded hover:bg-slate-100"
                  tabindex="-1"
                >
                  <UndoIcon />
                </button>
              {:else}
                <button
                  type="button"
                  onclick={() => titleCaseField(form, titleCasedStates, "islandGroup")}
                  data-i18n-key="title-case"
                  title={t("title-case", "Proper Case")}
                  class="w-6 h-6 p-1.5 text-slate-400 hover:text-slate-600 transition-colors cursor-pointer rounded hover:bg-slate-100"
                  tabindex="-1"
                >
                  <AaIcon />
                </button>
              {/if}
            </div>
          </div>
        </div>

        <!-- Island -->
        <div class="col-span-1">
          <label for="capture-island" data-i18n-key="island-label" class="block text-xs font-semibold text-slate-655 uppercase tracking-wider mb-1">{t("island-label", "Island")}</label>
          <div class="relative flex items-center">
            <Autocomplete
              id="capture-island"
              label=""
              placeholder={isAnyGeoPopulated ? "" : "eg Mauritius"}
              placeholderKey={isAnyGeoPopulated ? undefined : "island-placeholder"}
              bind:value={form.island}
              suggestions={islandSuggestions}
              oninput={handleIslandInput}
              onfocus={handleIslandFocus}
              delay={300}
              bind:inputRef={islandInputRef}
              extraInputClass="pr-20"
            />
            <div class="absolute right-2 flex items-center gap-1 z-100">
              <button
                type="button"
                onclick={handlePasteIsland}
                title={t("paste-island-title", "Paste Island")}
                class="w-6 h-6 p-1 text-slate-400 hover:text-slate-600 transition-colors cursor-pointer rounded hover:bg-slate-100"
                tabindex="-1"
              >
                <PasteIcon />
              </button>
              {#if form.island === titleCasedStates.island.titleCased && titleCasedStates.island.titleCased !== ""}
                <button
                  type="button"
                  onclick={() => undoTitleCaseField(form, titleCasedStates, "island")}
                  data-i18n-key="undo-title-case"
                  title={t("undo-title-case", "Undo Casing")}
                  class="w-6 h-6 p-1 text-slate-400 hover:text-slate-600 transition-colors cursor-pointer rounded hover:bg-slate-100"
                  tabindex="-1"
                >
                  <UndoIcon />
                </button>
              {:else}
                <button
                  type="button"
                  onclick={() => titleCaseField(form, titleCasedStates, "island")}
                  data-i18n-key="title-case"
                  title={t("title-case", "Proper Case")}
                  class="w-6 h-6 p-1.5 text-slate-400 hover:text-slate-600 transition-colors cursor-pointer rounded hover:bg-slate-100"
                  tabindex="-1"
                >
                  <AaIcon />
                </button>
              {/if}
            </div>
          </div>
        </div>
      </div>
      {/if}  

      <!-- Locality -->
      <div class="col-span-4">
        <label for="capture-locality" data-i18n-key="locality-label" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">{t("locality-label", "Locality (Gazetteer)")}</label>
        <div class="relative flex items-center ">
          <div class="w-full">
            <textarea
              id="capture-locality"
              placeholder={placeholders.locality}
              bind:value={form.locality}
              bind:this={localityInputRef}
              rows="2"
              class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all pr-24"
            ></textarea>
          </div>
          
          <!-- Buttons -->
          <div class="absolute right-5 flex items-center gap-1 z-100">
            <button
              type="button"
              onclick={handleCopyLocality}
              title={t("copy-locality-title", "Copy Locality")}
              class="w-6 h-6 p-1 text-slate-400 hover:text-slate-600 transition-colors cursor-pointer rounded hover:bg-slate-100"
              tabindex="-1"
            >
              {#if localityCopied}
                <CheckIcon />
              {:else}
                <CopyIcon />
              {/if}
            </button>
            <button
              type="button"
              onclick={handlePasteLocality}
              title={t("paste-locality-title", "Paste Locality")}
              class="w-6 h-6 p-1 text-slate-400 hover:text-slate-600 transition-colors cursor-pointer rounded hover:bg-slate-100"
              tabindex="-1"
            >
              <PasteIcon />
            </button>
            {#if form.locality === titleCasedStates.locality.titleCased && titleCasedStates.locality.titleCased !== ""}
              <button
                type="button"
                onclick={() => undoTitleCaseField(form, titleCasedStates, "locality")}
                data-i18n-key="undo-title-case"
                title={t("undo-title-case", "Undo Casing")}
                class="w-6 h-6 p-1 text-slate-400 hover:text-slate-600 transition-colors cursor-pointer rounded hover:bg-slate-100"
                tabindex="-1"
              >
                <UndoIcon />
              </button>
            {:else}
              <button
                type="button"
                onclick={() => titleCaseField(form, titleCasedStates, "locality")}
                data-i18n-key="title-case"
                title={t("title-case", "Proper Case")}
                class="w-6 h-6 p-1.5 text-slate-400 hover:text-slate-600 transition-colors cursor-pointer rounded hover:bg-slate-100"
                tabindex="-1"
              >
                <AaIcon />
              </button>
            {/if}
          </div>
          <div class="absolute right-0 -top-8 flex items-center justify-center" >
          <button id="search-locality-button" type="button" onclick={handleSearchLocality} title={t("search-locality-title", "Search Locality")} class="w-8 h-8 p-1 text-slate-400 hover:text-slate-600  transition-colors cursor-pointer rounded hover:bg-slate-100">
            <SearchIcon />
          </button>
          </div>
        </div>
      </div>

      

      <!-- Locality Notes -->
      <div class="grid grid-cols-12 gap-3">
        <div class="relative col-span-12">
          <label for="capture-locationNotes" data-i18n-key="location-notes-label" class="block text-xs font-semibold text-slate-655 uppercase tracking-wider mb-1">{t("location-notes-label", "Locality Notes")}</label>
          <textarea
            id="capture-locationNotes"
            placeholder={placeholders.localityNotes}
            bind:value={form.locationNotes}
            bind:this={locationNotesRef}
            rows="2"
            class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all pr-24"
          ></textarea>
          
          <!-- Clipboard Buttons -->
          <div class="absolute right-5 bottom-3.5 flex items-center gap-1 z-10">
            <button
              type="button"
              onclick={handleCopyLocationNotes}
              title={t("copy-location-notes-title", "Copy Notes")}
              class="w-6 h-6 p-1 text-slate-400 hover:text-slate-600 transition-colors cursor-pointer rounded hover:bg-slate-100"
              tabindex="-1"
            >
              {#if locationNotesCopied}
                <CheckIcon />
              {:else}
                <CopyIcon />
              {/if}
            </button>
            <button
              type="button"
              onclick={handlePasteLocationNotes}
              title={t("paste-location-notes-title", "Paste Notes")}
              class="w-6 h-6 p-1 text-slate-400 hover:text-slate-600 transition-colors cursor-pointer rounded hover:bg-slate-100"
              tabindex="-1"
            >
              <PasteIcon />
            </button>
            {#if form.locationNotes === titleCasedStates.locationNotes.titleCased && titleCasedStates.locationNotes.titleCased !== ""}
              <button
                type="button"
                onclick={() => undoTitleCaseField(form, titleCasedStates, "locationNotes")}
                data-i18n-key="undo-title-case"
                title={t("undo-title-case", "Undo Casing")}
                class="w-6 h-6 p-1 text-slate-400 hover:text-slate-600 transition-colors cursor-pointer rounded hover:bg-slate-100"
                tabindex="-1"
              >
                <UndoIcon />
              </button>
            {:else}
              <button
                type="button"
                onclick={() => titleCaseField(form, titleCasedStates, "locationNotes")}
                data-i18n-key="title-case-notes"
                title={t("title-case-notes", "Title case Locality Notes")}
                class="w-6 h-6 p-1.5 text-slate-400 hover:text-slate-600 transition-colors cursor-pointer rounded hover:bg-slate-100"
                tabindex="-1"
              >
                <AaIcon />
              </button>
            {/if}
          </div>
        </div>
      </div>

      <!-- Coordinates and Cultivated -->
      <div class="grid grid-cols-12 gap-3">
        <div class="relative col-span-9">
          <label for="capture-verbatimCoordinates" data-i18n-key="verbatim-coordinates-label" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">
            {t("verbatim-coordinates-label", "Coordinates")}
            {#if coordinatesError}
              <span data-i18n-key="coord-error-badge" class="text-[9px] text-red-500 font-bold ml-1 uppercase">{t("coord-error-badge", "(Invalid)")}</span>
            {/if}
          </label>
          <input
            id="capture-verbatimCoordinates"
            type="text"
            placeholder={placeholders.verbatimCoordinates}
            bind:value={form.verbatimCoordinates}
            onblur={handleCoordinatesBlur}
            class="w-full bg-white border text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all {coordinatesError ? 'border-red-500 bg-red-50 focus:border-red-500 focus:ring-red-500' : 'border-slate-300'}"
          />
          {#if form.decimalLatitude && form.decimalLongitude}
            <div class="absolute top-0 right-0 text-[10px] text-slate-400 mt-1">
              {(Number(form.decimalLatitude)).toFixed(6)}, {(Number(form.decimalLongitude)).toFixed(6)}
            </div>
          {/if}
        </div>
        <!-- Cultivated Specimen Flag -->
        <div class="col-span-3 flex items-center pt-5">
          <label class="flex items-center gap-2 cursor-pointer select-none">
            <input
              type="checkbox"
              bind:checked={form.cultivated}
              class="w-4 h-4 border border-slate-300 rounded-none accent-slate-800 outline-none cursor-pointer"
            />
            <span data-i18n-key="cultivated-label" class="text-xs font-semibold text-slate-600 uppercase tracking-wider">{t("cultivated-label", "Cultivated Specimen")}</span>
          </label>
        </div>
      </div>

      <!-- Verbatim Locality -->
      <div class="grid grid-cols-12 gap-3">
        <div class="col-span-12">
          <label for="capture-verbatimLocality" data-i18n-key="verbatim-locality-label" class="block text-xs font-semibold text-slate-655 uppercase tracking-wider mb-1">{t("verbatim-locality-label", "Verbatim Locality (copy data above)")}</label>
          <div class="relative flex items-center">
            <textarea
              id="capture-verbatimLocality"
              data-i18n-key="verbatim-locality-placeholder"
              placeholder={t("verbatim-locality-placeholder", "Select any portion above and click 'copy' to populate")}
              bind:value={form.verbatimLocality}
              bind:this={verbatimLocalityRef}
              rows="3"
              class="w-full bg-slate-100 border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all pr-12"
              readonly
            ></textarea>
            <button
              type="button"
              onclick={handleCopyVerbatimLocality}
              title={t("copy-verbatim-locality-title", "Copy selected text to clipboard")}
              class="absolute w-6 h-6 right-2 bottom-3.5 p-1 text-slate-400 hover:text-slate-600 transition-colors cursor-pointer rounded hover:bg-slate-100"
              tabindex="-1"
            >
              {#if verbatimLocalityCopied}
                <CheckIcon />
              {:else}
                <CopyIcon />
              {/if}
            </button>
          </div>
        </div>
      </div>

      <!-- Habitat -->
      <div class="w-full">
        <label for="capture-habitat" data-i18n-key="habitat-label" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">{t("habitat-label", "Habitat")}</label>
        <div class="relative flex items-center">
          <input
            id="capture-habitat"
            data-i18n-key="habitat-placeholder"
            type="text"
            placeholder={t("habitat-placeholder", "eg Tapia woodland")}
            bind:value={form.habitat}
            class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all pr-12"
          />
          <div class="absolute right-5 flex items-center gap-1 z-100">

            {#if form.habitat === titleCasedStates.habitat.titleCased && titleCasedStates.habitat.titleCased !== ""}
              <button
                type="button"
                onclick={() => undoTitleCaseField(form, titleCasedStates, "habitat")}
                data-i18n-key="undo-title-case"
                title={t("undo-title-case", "Undo Casing")}
                class="w-6 h-6 p-1 text-slate-400 hover:text-slate-600 transition-colors cursor-pointer rounded hover:bg-slate-100"
                tabindex="-1"
              >
                <UndoIcon />
              </button>
            {:else}
              <button
                type="button"
                onclick={() => titleCaseField(form, titleCasedStates, "habitat")}
                data-i18n-key="title-case-habitat"
                title={t("title-case-habitat", "Title case Habitat")}
                class="w-6 h-6 p-1.5 text-slate-400 hover:text-slate-600 transition-colors cursor-pointer rounded hover:bg-slate-100"
                tabindex="-1"
              >
                <AaIcon />
              </button>
            {/if}
          </div>
        </div>
      </div>

      <!-- Verbatim Elevation and grid reference -->
      <div class="grid grid-cols-12 gap-3">
        <div class="col-span-4">
          <label for="capture-verbatimElevation" data-i18n-key="verbatim-elevation-label" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">{t("verbatim-elevation-label", "Verbatim Elevation")}</label>
          <input
            id="capture-verbatimElevation"
            data-i18n-key="verbatim-elevation-placeholder"
            type="text"
            placeholder={t("verbatim-elevation-placeholder", "eg 1200m or 3400ft")}
            bind:value={form.verbatimElevation}
            class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
          />
        </div>
        <div class="relative col-span-4">
          <label for="capture-grid" data-i18n-key="grid-label" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">{t("grid-label", "Grid reference")}</label>
          <input
            id="capture-grid"
            data-i18n-key="grid-placeholder"
            type="text"
            placeholder={t("grid-placeholder", "eg 2432CD")}
            bind:value={form.gridReference}
            oninput={() => isGridCalculated = false}
            class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
          />
          {#if isGridCalculated && form.gridReference}
            <div class="absolute text-[10px] text-emerald-800 font-semibold uppercase px-1 py-px bg-emerald-100 rounded top-0 right-0">{t("grid-calculated-badge", "calc.")}</div>
          {/if}
        </div>
      </div>
    </div>
    
    <!-- Section: Determination / Identification -->
    <div class="space-y-3 pt-2">
      <h3 data-i18n-key="identification-heading" class="text-[10px] font-bold text-slate-400 uppercase tracking-wider border-b border-slate-100 pb-1">{t("identification-heading", "Identification")}</h3>
      
      <div class="grid grid-cols-12 gap-3">
        <!-- Qualifier -->
        <div class="col-span-2">
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

        <!-- Scientific Name -->
        <div class="relative col-span-7">
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
            onblur={handleTaxonBlur}
            delay={300}
          />
          {#if nameValidationStatus === "invalid" && form.scientificName}
            <div class="absolute text-xs text-slate-600 font-semibold uppercase px-1 py-px bg-amber-300 rounded top-0 right-0">{t("name-not-in-wcvp", "Not in WCVP")}</div>
          {/if}
        </div>

        <!-- Type Status -->
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
      <div class="grid grid-cols-12 gap-3 pt-2">
        <!-- Det By -->
        <div class="col-span-6">
          <MultiSelectAutocomplete
            id="capture-identifiedBy"
            label="Det By"
            labelKey="det-by-label"
            placeholder="Type name and press Enter..."
            placeholderKey="names-field-placeholder"
            bind:selectedValues={form.identifiedBy}
            suggestions={identifiedBySuggestions}
            oninput={handleIdentifiedByInput}
            delay={300}
          />
        </div>
  
        <!-- Identification Date Fields -->
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
          <label for="capture-yearIdentified" data-i18n-key="det-year-label" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">{t("det-year-label", "Year Ident.")}</label>
          <input
            id="capture-yearIdentified"
            type="number"
            bind:value={form.yearIdentified}
            onblur={handleIdentificationDateBlur}
            class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-2 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
          />
        </div>
      </div>
      <!-- Identification Notes -->
      <div class="">
        <label for="capture-identificationRemarks" data-i18n-key="det-remarks-label" class="block text-xs font-semibold text-slate-655 uppercase tracking-wider mb-1">{t("det-remarks-label", "Identification Notes")}</label>
        <input
          id="capture-identificationRemarks"
          data-i18n-key="det-remarks-placeholder"
          type="text"
          placeholder={t("det-remarks-placeholder", "eg 'cf. species A'")}
          bind:value={form.identificationRemarks}
          class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
        />
      </div>
    </div>

  </form>

  <!-- Form Actions Footer -->
  <div class="px-4 py-3 bg-slate-100 border-t border-slate-300 flex justify-between items-center border-box">
    <!-- Reset and Load Previous -->
    <div class="flex gap-2">
      <button
        type="button"
        onclick={handleReset}
        data-i18n-key="reset-button"
        class="bg-white border border-slate-300 text-slate-700 text-xs font-bold uppercase tracking-wider px-4 py-2 hover:bg-slate-50 focus:outline-none transition-all rounded-none cursor-pointer"
      >
        {t("reset-button", "Reset Form")}
      </button>
      {#if lastSavedRecord}
        <button
          type="button"
          onclick={handleShowPreviousRecord}
          data-i18n-key="previous-record-button"
          class="bg-white border border-slate-300 text-indigo-700 text-xs font-bold uppercase tracking-wider px-4 py-2 hover:bg-indigo-50 focus:outline-none transition-all rounded-none cursor-pointer"
        >
          {t("previous-record-button", "Load Last Saved")}
        </button>
      {/if}
    </div>

    <!-- Status Message Display -->
    {#if statusMessage}
      <div class="flex-1 px-4 text-center">
        <span class="text-xs font-bold tracking-wide {statusType === 'success' ? 'text-emerald-600' : 'text-red-500'}">
          {statusMessage}
        </span>
      </div>
    {/if}

    <!-- Submit Save Button -->
    <button
      type="button"
      onclick={handleSave}
      disabled={saving}
      class="bg-slate-800 text-white text-xs font-bold uppercase tracking-wider px-6 py-2.5 hover:bg-slate-900 focus:outline-none disabled:bg-slate-400 transition-all rounded-none cursor-pointer flex items-center gap-1.5"
    >
      {#if saving}
        <svg class="animate-spin h-3 w-3 text-white" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
          <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
          <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
        </svg>
        <span data-i18n-key="saving-button">{t("saving-button", "Saving...")}</span>
      {:else}
        <span data-i18n-key="save-button">{form.id ? t("update-button", "Update Record") : t("save-button", "Save Record")}</span>
      {/if}
    </button>
  </div>
</div>

{#if showLocalitySearchDialog}
  <div 
    class="fixed inset-0 z-[120] flex items-center justify-center bg-slate-900/40 backdrop-blur-xs p-4"
    role="dialog"
    aria-modal="true"
    tabindex="-1"
    onclick={(e) => { if (e.target === e.currentTarget) showLocalitySearchDialog = false; }}
    onkeydown={(e) => { 
      if (e.key === "Escape") {
        e.preventDefault();
        showLocalitySearchDialog = false; 
      }
    }}
  >
    <div class="bg-white border border-slate-200 shadow-2xl max-w-lg w-full p-5 flex flex-col gap-4 rounded-none min-h-[350px]">
      <div class="flex justify-between items-center border-b border-slate-100 pb-3">
        <h3 class="font-bold text-slate-800 text-sm uppercase tracking-wider">{t("search-locality-dialog-title", "Search Locality (Gazetteer)")}</h3>
        <button 
          type="button" 
          onclick={() => { showLocalitySearchDialog = false; }}
          title={t("close-btn", "Close")}
          class="text-slate-400 hover:text-slate-600 transition-colors cursor-pointer"
        >
          <svg xmlns="http://www.w3.org/2000/svg" class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>

      <div class="flex-1 relative pb-20">
        <Autocomplete
          id="dialog-locality-search"
          label={t("search-locality-input-label", "Search Locality")}
          placeholder={t("search-locality-placeholder", "Start typing to search...")}
          bind:value={dialogSearchQuery}
          suggestions={dialogLocalitySuggestions}
          oninput={handleDialogLocalityInput}
          onselect={handleDialogLocalitySelect}
          displayKey="locality"
          delay={300}
          bind:inputRef={dialogSearchInputRef}
          useTextArea={false}
        />
        <p class="text-xs text-slate-400 mt-2">
          {t("search-locality-help", "Type at least 2 characters to search the gazetteer database. Select a locality to populate form fields.")}
        </p>
      </div>

      <div class="flex justify-end gap-2 border-t border-slate-100 pt-3">
        <button
          type="button"
          onclick={() => { showLocalitySearchDialog = false; }}
          class="px-3 py-1.5 text-xs font-semibold text-slate-500 hover:bg-slate-50 border border-slate-200 transition-colors cursor-pointer rounded-none"
        >
          {t("close-btn", "Close")}
        </button>
      </div>
    </div>
  </div>
{/if}

{#if showBarcodeWarningModal}
  <div 
    class="fixed inset-0 z-[120] flex items-center justify-center bg-slate-900/40 backdrop-blur-xs p-4"
    role="dialog"
    aria-modal="true"
    tabindex="-1"
    onclick={(e) => { if (e.target === e.currentTarget) showBarcodeWarningModal = false; }}
    onkeydown={(e) => { 
      if (e.key === "Escape") {
        e.preventDefault();
        showBarcodeWarningModal = false; 
      } else if (e.key === "Enter") {
        e.preventDefault();
        showBarcodeWarningModal = false;
        handleSave(null, true);
      }
    }}
  >
    <div class="bg-white border border-slate-200 shadow-2xl max-w-sm w-full p-5 flex flex-col gap-4 rounded-none">
      <div class="flex items-start gap-3">
        <div class="p-2 bg-amber-50 text-amber-600 rounded-full shrink-0">
          <svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256" class="w-5 h-5 text-amber-600">
            <path d="M236.8,188.09,149.35,36.22a24.43,24.43,0,0,0-42.7,0L19.2,188.09a23.5,23.5,0,0,0,0,23.82,24,24,0,0,0,21.35,12.09H215.45a24,24,0,0,0,21.35-12.09A23.5,23.5,0,0,0,236.8,188.09ZM120,104a8,8,0,0,1,16,0v40a8,8,0,0,1-16,0Zm8,80a12,12,0,1,1,12-12A12,12,0,0,1,128,184Z"></path>
          </svg>
        </div>
        <div class="space-y-2">
          <h3 data-i18n-key="barcode-duplicate-warning-title" class="font-bold text-slate-800">{t("barcode-duplicate-warning-title", "Barcode Already Captured")}</h3>
          <p data-i18n-key="barcode-duplicate-warning-desc" class="text-sm text-slate-500 leading-relaxed">
            {t("barcode-duplicate-warning-desc", "This barcode number has already been captured in this session. Are you sure you want to save it again?")}
          </p>
        </div>
      </div>
      
      <div class="flex justify-end gap-2 mt-2">
        <button
          type="button"
          data-i18n-key="cancel-btn"
          onclick={() => { showBarcodeWarningModal = false; }}
          class="px-3 py-1.5 text-xs font-semibold text-slate-500 hover:bg-slate-50 border border-slate-200 transition-colors cursor-pointer rounded-none"
        >
          {t("cancel-btn", "Cancel")}
        </button>
        <button
          type="button"
          data-i18n-key="save-anyway-btn"
          onclick={() => {
            showBarcodeWarningModal = false;
            handleSave(null, true);
          }}
          class="px-3.5 py-1.5 text-xs font-semibold text-white bg-slate-800 hover:bg-slate-900 transition-colors cursor-pointer rounded-none"
        >
          {t("save-anyway-btn", "Save Anyway")}
        </button>
      </div>
    </div>
  </div>
{/if}
