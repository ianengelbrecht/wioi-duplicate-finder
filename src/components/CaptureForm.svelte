<script>
  import { invoke } from "@tauri-apps/api/core";
  import { convert } from 'geo-coordinates-parser';
  import Autocomplete from "./Autocomplete.svelte";

  let {
    sessionId = null,
    collectionCode = "WIOI",
    activeRecord = $bindable(null), // The selected record to edit (or empty for new)
    onSaveSuccess = () => {}
  } = $props();

  let form = $state({
    id: null,
    collectionCode: "",
    catalogNumber: "",
    duplicates: "",
    recordedBy: "",
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
    identifiedBy: "",
    yearIdentified: "",
    monthIdentified: "",
    dayIdentified: "",
    identificationRemarks: "",
    occurrenceRemarks: "",
    fieldNotes: ""
  });

  let coordinatesError = $state(false);

  let isAnyGeoPopulated = $derived(
    !!((form.country && form.country.trim().length > 0) ||
       (form.stateProvince && form.stateProvince.trim().length > 0) ||
       (form.county && form.county.trim().length > 0) ||
       (form.municipality && form.municipality.trim().length > 0))
  );

  let saving = $state(false);
  let statusMessage = $state("");
  let statusType = $state(""); // "success" or "error"

  // Dropdown suggestions lists
  let taxonSuggestions = $state(/** @type {any[]} */ ([]));
  let localitySuggestions = $state(/** @type {any[]} */ ([]));
  let collectorSuggestions = $state(/** @type {any[]} */ ([]));
  let countrySuggestions = $state(/** @type {any[]} */ ([]));
  let stateProvinceSuggestions = $state(/** @type {any[]} */ ([]));
  let countySuggestions = $state(/** @type {any[]} */ ([]));
  let municipalitySuggestions = $state(/** @type {any[]} */ ([]));
  
  // Custom suggestion list for duplicates
  let duplicateSuggestions = $state(/** @type {any[]} */ ([]));
  const duplicateCodes = ["P", "K", "MO", "MAU"];

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
      form.recordedBy = activeRecord.recordedBy || "";
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
      form.identifiedBy = activeRecord.identifiedBy || "";
      form.yearIdentified = activeRecord.yearIdentified !== null && activeRecord.yearIdentified !== undefined ? activeRecord.yearIdentified.toString() : "";
      form.monthIdentified = activeRecord.monthIdentified !== null && activeRecord.monthIdentified !== undefined ? activeRecord.monthIdentified.toString() : "";
      form.dayIdentified = activeRecord.dayIdentified !== null && activeRecord.dayIdentified !== undefined ? activeRecord.dayIdentified.toString() : "";
      form.identificationRemarks = activeRecord.identificationRemarks || "";
      form.occurrenceRemarks = activeRecord.occurrenceRemarks || "";
      form.fieldNotes = activeRecord.fieldNotes || "";
      
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
      collectorSuggestions = /** @type {any[]} */ (await invoke("autocomplete_recorded_by", { query: val }));
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
  function toProperCase(str) {
    if (!str) return "";
    return str
      .toLowerCase()
      .replace(/\b([a-z])/g, (m) => m.toUpperCase());
  }

  function properCaseField(/** @type {string} */ field) {
    let formObj = /** @type {any} */ (form);
    let val = formObj[field];
    if (typeof val === "string") {
      formObj[field] = toProperCase(val);
    }
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
    
    // Validate required fields
    if (form.scientificName.trim().length === 0) {
      statusMessage = "Error: Scientific Name is required.";
      statusType = "error";
      saving = false;
      return;
    }
    
    let recordPayload = {
      ...form,
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
      identifiedBy: "",
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

    <!-- Row 1: Collection Code (read-only), catalogNumber, duplicates -->
    <div class="grid grid-cols-12 gap-3">
      <div class="col-span-3">
        <label for="capture-collectionCode" class="block text-xs font-semibold text-slate-650 uppercase tracking-wider mb-1">Collection Code</label>
        <input
          id="capture-collectionCode"
          type="text"
          readonly
          bind:value={form.collectionCode}
          class="w-full bg-slate-100 border border-slate-300 text-slate-500 text-sm px-3 py-2 cursor-not-allowed outline-none rounded-none font-semibold"
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
        />
      </div>
    </div>

    <!-- Row 2: Collector/s (recordedBy), Collector Number, Verbatim Date, Year, Month, Day -->
    <div class="grid grid-cols-12 gap-3">
      <div class="col-span-3">
        <Autocomplete
          id="capture-recordedBy"
          label="Collector/s"
          placeholder="Partial eg 'Raza'"
          bind:value={form.recordedBy}
          suggestions={collectorSuggestions}
          oninput={handleCollectorInput}
          delay={300}
        />
      </div>
      <div class="col-span-2">
        <label for="capture-recordNumber" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">Collector No.</label>
        <input
          id="capture-recordNumber"
          type="text"
          placeholder="eg 1042"
          bind:value={form.recordNumber}
          class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
        />
      </div>
      
      <div class="col-span-3">
        <label for="capture-verbatimEventDate" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">Verbatim Date</label>
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
          <label for="capture-year" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">Year</label>
          <input
            id="capture-year"
            type="number"
            bind:value={form.year}
            class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-2 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
          />
        </div>
        <div class="flex-1">
          <label for="capture-month" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">Month</label>
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
          <label for="capture-day" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">Day</label>
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

    <!-- Row 3: Geography with Proper-case buttons -->
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
            <button
              type="button"
              onclick={() => properCaseField("country")}
              title="Proper case Country"
              class="absolute right-2 top-3 text-slate-400 hover:text-slate-600 font-mono text-[10px] font-bold z-10"
            >
              Aa
            </button>
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
            <button
              type="button"
              onclick={() => properCaseField("stateProvince")}
              title="Proper case Admin 2"
              class="absolute right-2 top-3 text-slate-400 hover:text-slate-600 font-mono text-[10px] font-bold z-10"
            >
              Aa
            </button>
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
            <button
              type="button"
              onclick={() => properCaseField("county")}
              title="Proper case Admin 3"
              class="absolute right-2 top-3 text-slate-400 hover:text-slate-600 font-mono text-[10px] font-bold z-10"
            >
              Aa
            </button>
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
            <button
              type="button"
              onclick={() => properCaseField("municipality")}
              title="Proper case Admin 4"
              class="absolute right-2 top-3 text-slate-400 hover:text-slate-600 font-mono text-[10px] font-bold z-10"
            >
              Aa
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- Row 4: Locality (on its own row) & Verbatim Coordinates (on its own row) -->
    <div class="space-y-3">
      <div>
        <label for="capture-locality" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">Locality</label>
        <div class="relative flex items-center">
          <Autocomplete
            id="capture-locality"
            label=""
            placeholder="eg 'Antakohandro' (use partial search eg 'Anta')"
            bind:value={form.locality}
            suggestions={localitySuggestions}
            oninput={handleLocalityInput}
            delay={300}
          />
          <button
            type="button"
            onclick={() => properCaseField("locality")}
            title="Proper case Locality"
            class="absolute right-2 top-3 text-slate-400 hover:text-slate-600 font-mono text-[10px] font-bold z-10"
          >
            Aa
          </button>
        </div>
      </div>

      <!-- Row 5: Locality Notes (locationNotes) -->
      <div>
        <label for="capture-locationNotes" class="block text-xs font-semibold text-slate-650 uppercase tracking-wider mb-1">Locality Notes</label>
        <div class="relative flex items-start">
          <textarea
            id="capture-locationNotes"
            rows="2"
            placeholder="eg '12 km south, main ravine'"
            bind:value={form.locationNotes}
            class="w-full bg-white border border-slate-300 text-slate-800 text-sm pl-3 pr-8 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all resize-none"
          ></textarea>
          <button
            type="button"
            onclick={() => properCaseField("locationNotes")}
            title="Proper case Locality Notes"
            class="absolute right-2 bottom-3 text-slate-400 hover:text-slate-600 font-mono text-[10px] font-bold"
          >
            Aa
          </button>
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
      <label for="capture-verbatimLocality" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">Verbatim Locality (copy data above)</label>
      <textarea
        id="capture-verbatimLocality"
        rows="2"
        readonly
        placeholder="Read-only imported value"
        bind:value={form.verbatimLocality}
        class="w-full bg-slate-100 border border-slate-300 text-slate-500 text-sm px-3 py-2 cursor-not-allowed outline-none rounded-none"      >
      </textarea>
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
          <button
            type="button"
            onclick={() => properCaseField("habitat")}
            title="Proper case Habitat"
            class="absolute right-2 top-3 text-slate-400 hover:text-slate-600 font-mono text-[10px] font-bold z-10"
          >
            Aa
          </button>
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
        <button
          type="button"
          onclick={() => properCaseField("fieldNotes")}
          title="Proper case Plant description"
          class="absolute right-2 bottom-3 text-slate-400 hover:text-slate-600 font-mono text-[10px] font-bold"
        >
          Aa
        </button>
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
        <button
          type="button"
          onclick={() => properCaseField("occurrenceRemarks")}
          title="Proper case General Notes"
          class="absolute right-2 bottom-3 text-slate-400 hover:text-slate-600 font-mono text-[10px] font-bold"
        >
          Aa
        </button>
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

        <!-- Type Status single-select dropdown -->
        <div class="col-span-3">
          <label for="capture-typeStatus" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">Type Status</label>
          <select
            id="capture-typeStatus"
            bind:value={form.typeStatus}
            class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-2 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
          >
            <option value="">(None)</option>
            <option value="holotype">holotype</option>
            <option value="isotype">isotype</option>
            <option value="syntype">syntype</option>
            <option value="paratype">paratype</option>
            <option value="lectotype">lectotype</option>
            <option value="neotype">neotype</option>
            <option value="epitype">epitype</option>
          </select>
        </div>
      </div>
    </div>

    <!-- Row 8: Identified By, Year, Month, Day Identified -->
    <div class="grid grid-cols-12 gap-3">
      <div class="col-span-6">
        <label for="capture-identifiedBy" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">Identified By</label>
        <input
          id="capture-identifiedBy"
          type="text"
          placeholder="Partial search eg 'Raza'"
          bind:value={form.identifiedBy}
          class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
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
  <div class="p-4 border-t  border-slate-300 bg-slate-50 ">
    <div class="w-1/2 ml-auto flex justify-between gap-2" >
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
