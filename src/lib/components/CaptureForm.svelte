<script>
  import { getContext } from "svelte";
  import Autocomplete from "./Autocomplete.svelte";
  import { specimenService } from "../services/specimenService.js";
  import { isValidPartialDate, comparePartialDates } from "../utils/isValidPartialDate.js";
  import { splitNames } from "../utils/splitNames.js";
  import { getDuplicateSuggestions } from "../utils/duplicates.js";
  import { titleCaseField, undoTitleCaseField, getInitialTrackingState } from "../utils/titleCaseHelper.js";

  import DateCollectorSection from "./forms/DateCollectorSection.svelte";
  import GeographySection from "./forms/GeographySection.svelte";
  import TaxonomySection from "./forms/TaxonomySection.svelte";
  import DeterminationSection from "./forms/DeterminationSection.svelte";

  const t = getContext("t");

  /**
   * @typedef {Object} CaptureFormProps
   * @property {number|null} [sessionId=null]
   * @property {string} collectionCode
   * @property {any} [activeRecord=null]
   * @property {() => void} [onSaveSuccess]
   */

  /** @type {CaptureFormProps} */
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
    stateProvince: "",
    county: "",
    municipality: "",
    locality: "",
    verbatimCoordinates: "",
    decimalLatitude: "",
    decimalLongitude: "",
    locationNotes: "", // Mapped to locationRemarks
    verbatimLocality: "",
    verbatimElevation: "",
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
    cultivated: false
  });

  /** @type {boolean} */
  let coordinatesError = $state(false);
  /** @type {any} */
  let lastSavedRecord = $state(/** @type {any} */ (null));
  /** @type {boolean} */
  let saving = $state(false);

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
      form.municipality = activeRecord.municipality || "";
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
      titleCasedStates = getInitialTrackingState();
      
      statusMessageKey = "";
      statusMessageDefault = "";
      statusType = "";
    }
  });

  /**
   * Handle form submission: validate required fields and data formats, then save the record using the specimenService. Displays appropriate success or error messages based on the outcome.  
   * @param {Event|null} e
   */
  async function handleSave(e) {
    if (e) e.preventDefault();
    if (saving) return;

    saving = true;
    statusMessageKey = "";
    statusMessageDefault = "";
    statusType = "";

    // Validate collector
    if (!form.recordedBy.trim()) {
      statusMessageKey = "recorded-by-required-error";
      statusMessageDefault = "Error: Primary Collector is required.";
      statusType = "error";
      saving = false;
      return;
    }

    // Validate collector number
    if (!form.recordNumber.trim()) {
      statusMessageKey = "record-number-required-error";
      statusMessageDefault = "Error: Collector Number is required.";
      statusType = "error";
      saving = false;
      return;
    }

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
      dayIdentified: form.dayIdentified !== "" ? parseInt(form.dayIdentified) : null
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
    duplicateSuggestions = [];
    titleCasedStates = getInitialTrackingState();
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
    form.municipality = lastSavedRecord.municipality;
    form.locality = lastSavedRecord.locality;
    form.verbatimCoordinates = lastSavedRecord.verbatimCoordinates;
    form.decimalLatitude = lastSavedRecord.decimalLatitude !== null ? lastSavedRecord.decimalLatitude.toString() : "";
    form.decimalLongitude = lastSavedRecord.decimalLongitude !== null ? lastSavedRecord.decimalLongitude.toString() : "";
    form.locationNotes = lastSavedRecord.locationNotes;
    form.verbatimLocality = lastSavedRecord.verbatimLocality;
    form.verbatimElevation = lastSavedRecord.verbatimElevation;
    form.habitat = lastSavedRecord.habitat;
    form.identificationQualifier = lastSavedRecord.identificationQualifier;
    form.scientificName = lastSavedRecord.scientificName;
    form.taxonID = lastSavedRecord.taxonID;
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
    <span data-i18n-key="save-shortcut-desc" class="text-[10px] text-slate-400 font-semibold uppercase">{t("save-shortcut-desc", "Shortcut: Ctrl+S to save")}</span>
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

    <!-- Section 1: Date and Collectors -->
    <DateCollectorSection
      bind:form
      bind:titleCasedStates
      bind:statusMessageKey
      bind:statusMessageDefault
      bind:statusType
      {undoTitleCaseField}
      {titleCaseField}
      {t}
    />

    <!-- Section 2: Geographic Locality -->
    <GeographySection
      bind:form
      bind:titleCasedStates
      bind:coordinatesError
      {undoTitleCaseField}
      {titleCaseField}
      {t}
    />

    <!-- Section 3: Taxonomy -->
    <TaxonomySection bind:form {t} />

    <!-- Section 4: Determination / Identification -->
    <DeterminationSection
      bind:form
      bind:statusMessageKey
      bind:statusMessageDefault
      bind:statusType
      {t}
    />

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
