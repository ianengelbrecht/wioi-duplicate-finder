<script>
  import { onDestroy } from "svelte";
  import { convert } from "geo-coordinates-parser";
  import Autocomplete from "../Autocomplete.svelte";
  import { geographyService } from "../../services/geographyService.js";
  import { copySelectedOrValue, pasteAtCursor } from "../../utils/clipboard.js";

  /**
   * @typedef {Object} GeographySectionProps
   * @property {any} form
   * @property {any} titleCasedStates
   * @property {boolean} [coordinatesError=false]
   * @property {function} undoTitleCaseField
   * @property {function} titleCaseField
   * @property {function} t
   */

  /** @type {GeographySectionProps} */
  let {
    form = $bindable(),
    titleCasedStates = $bindable(),
    coordinatesError = $bindable(false),
    undoTitleCaseField,
    titleCaseField,
    t
  } = $props();

  /** @type {string[]} */
  let countrySuggestions = $state([]);
  /** @type {string[]} */
  let stateProvinceSuggestions = $state([]);
  /** @type {string[]} */
  let countySuggestions = $state([]);
  /** @type {string[]} */
  let municipalitySuggestions = $state([]);
  /** @type {string[]} */
  let localitySuggestions = $state([]);

  let isAnyGeoPopulated = $derived(
    !!((form.country && form.country.trim().length > 0) ||
       (form.stateProvince && form.stateProvince.trim().length > 0) ||
       (form.county && form.county.trim().length > 0) ||
       (form.municipality && form.municipality.trim().length > 0))
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
    onCountyChanged();
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
  async function handleMunicipalityInput(val) {
    if (!val || val.trim().length === 0) {
      municipalitySuggestions = [];
      return;
    }
    try {
      municipalitySuggestions = await geographyService.autocompleteGeography("municipality", val, form.country, form.stateProvince, form.county);
    } catch (e) {
      console.error(e);
    }
  }

  /**
   * @param {string} val
   */
  async function handleLocalityInput(val) {
    if (val.trim().length < 2) {
      localitySuggestions = [];
      return;
    }
    try {
      localitySuggestions = await geographyService.autocompleteLocality(val);
    } catch (e) {
      console.error(e);
    }
  }

  /**
   * @param {string} field
   */
  async function triggerGeoAutocomplete(field) {
    try {
      const results = await geographyService.autocompleteGeography(field, "", form.country, form.stateProvince, form.county);
      if (field === "stateProvince") stateProvinceSuggestions = results;
      else if (field === "county") countySuggestions = results;
      else if (field === "municipality") municipalitySuggestions = results;
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

  function handleMunicipalityFocus() {
    if (form.municipality.trim() === "") {
      triggerGeoAutocomplete("municipality");
    }
  }

  onDestroy(() => {
    if (copyTimeoutId) clearTimeout(copyTimeoutId);
    if (localityCopyTimeout) clearTimeout(localityCopyTimeout);
    if (locationNotesCopyTimeout) clearTimeout(locationNotesCopyTimeout);
  });
</script>

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
          delay={300}
        />
        {#if form.country === titleCasedStates.country.titleCased && titleCasedStates.country.titleCased !== ""}
          <button
            type="button"
            onclick={() => undoTitleCaseField(form, titleCasedStates, "country")}
            data-i18n-key="undo-title-case"
            title={t("undo-title-case", "Undo Casing")}
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
            onclick={() => titleCaseField(form, titleCasedStates, "country")}
            data-i18n-key="title-case"
            title={t("title-case", "Proper Case")}
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

    <div>
      <label for="capture-stateProvince" data-i18n-key="state-province-label" class="block text-xs font-semibold text-slate-655 uppercase tracking-wider mb-1">
        {form.country && form.country.toLowerCase() === "madagascar" ? t("state-province-label", "Province") : t("state-province-label", "Admin Div 1")}
      </label>
      <div class="relative flex items-center">
        <Autocomplete
          id="capture-stateProvince"
          label=""
          placeholder={isAnyGeoPopulated ? "" : (form.country && form.country.toLowerCase() === "madagascar" ? "eg Fianarantsoa" : "eg Province/State")}
          placeholderKey={isAnyGeoPopulated ? undefined : (form.country && form.country.toLowerCase() === "madagascar" ? "state-province-placeholder-mada" : "state-province-placeholder")}
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
            onclick={() => titleCaseField(form, titleCasedStates, "stateProvince")}
            data-i18n-key="title-case"
            title={t("title-case", "Proper Case")}
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

    <div>
      <label for="capture-county" data-i18n-key="county-label" class="block text-xs font-semibold text-slate-650 uppercase tracking-wider mb-1">
        {form.country && form.country.toLowerCase() === "madagascar" ? t("county-label", "Region") : t("county-label", "Admin Div 2")}
      </label>
      <div class="relative flex items-center">
        <Autocomplete
          id="capture-county"
          label=""
          placeholder={isAnyGeoPopulated ? "" : (form.country && form.country.toLowerCase() === "madagascar" ? "eg Amoron'i Mania" : "eg Region/County")}
          placeholderKey={isAnyGeoPopulated ? undefined : (form.country && form.country.toLowerCase() === "madagascar" ? "county-placeholder-mada" : "county-placeholder")}
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
            onclick={() => titleCaseField(form, titleCasedStates, "county")}
            data-i18n-key="title-case"
            title={t("title-case", "Proper Case")}
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

    <div>
      <label for="capture-municipality" data-i18n-key="municipality-label" class="block text-xs font-semibold text-slate-655 uppercase tracking-wider mb-1">{t("municipality-label", "Municipality")}</label>
      <div class="relative flex items-center">
        <Autocomplete
          id="capture-municipality"
          label=""
          placeholder={isAnyGeoPopulated ? "" : (form.country && form.country.toLowerCase() === "madagascar" ? "eg Ambositra" : "eg Municipality")}
          placeholderKey={isAnyGeoPopulated ? undefined : (form.country && form.country.toLowerCase() === "madagascar" ? "municipality-placeholder-mada" : "municipality-placeholder")}
          bind:value={form.municipality}
          suggestions={municipalitySuggestions}
          oninput={handleMunicipalityInput}
          onfocus={handleMunicipalityFocus}
          delay={300}
        />
        {#if form.municipality === titleCasedStates.municipality.titleCased && titleCasedStates.municipality.titleCased !== ""}
          <button
            type="button"
            onclick={() => undoTitleCaseField(form, titleCasedStates, "municipality")}
            data-i18n-key="undo-title-case"
            title={t("undo-title-case", "Undo Casing")}
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
            onclick={() => titleCaseField(form, titleCasedStates, "municipality")}
            data-i18n-key="title-case"
            title={t("title-case", "Proper Case")}
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
  </div>

  <div class="grid grid-cols-12 gap-3">
    <!-- Locality Field -->
    <div class="col-span-8">
      <label for="capture-locality" data-i18n-key="locality-label" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">{t("locality-label", "Locality")}</label>
      <div class="relative flex items-center">
        <div class="w-full">
          <Autocomplete
            id="capture-locality"
            label=""
            placeholder="eg 10km north of Ambositra, on road to Antsirabe"
            placeholderKey="locality-placeholder"
            bind:value={form.locality}
            suggestions={localitySuggestions}
            oninput={handleLocalityInput}
            delay={300}
            inputRef={localityInputRef}
          />
        </div>
        
        <!-- Clipboard Buttons -->
        <div class="absolute right-10 bottom-3.5 flex items-center gap-1 z-10">
          <button
            type="button"
            onclick={handleCopyLocality}
            title={t("copy-locality-title", "Copy Locality")}
            class="p-1 text-slate-400 hover:text-slate-600 transition-colors cursor-pointer rounded hover:bg-slate-100"
            tabindex="-1"
          >
            {#if localityCopied}
              <svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256" class="w-3.5 h-3.5 text-emerald-600">
                <path d="M229.66,77.66l-128,128a8,8,0,0,1-11.32,0l-56-56a8,8,0,0,1,11.32-11.32L102,192.69,218.34,76.34a8,8,0,0,1,11.32,11.32Z"></path>
              </svg>
            {:else}
              <svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256" class="w-3.5 h-3.5">
                <path d="M200,32H163.74a47.92,47.92,0,0,0-71.48,0H56A16,16,0,0,0,40,48V208a16,16,0,0,0,16,16H200a16,16,0,0,0,16-16V48A16,16,0,0,0,200,32Zm-72,0a32,32,0,0,1,32,32H96A32,32,0,0,1,128,32Zm72,176H56V48H80v8a8,8,0,0,0,8,8h80a8,8,0,0,0,8-8V48h24V208Z"></path>
              </svg>
            {/if}
          </button>
          <button
            type="button"
            onclick={handlePasteLocality}
            title={t("paste-locality-title", "Paste Locality")}
            class="p-1 text-slate-400 hover:text-slate-600 transition-colors cursor-pointer rounded hover:bg-slate-100"
            tabindex="-1"
          >
            <svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256" class="w-3.5 h-3.5">
              <path d="M224,80V200a16,16,0,0,1-16,16H48a16,16,0,0,1-16-16V80A16,16,0,0,1,48,64H80V56A32,32,0,0,1,112,24h32a32,32,0,0,1,32,32v8h32A16,16,0,0,1,224,80ZM160,56a16,16,0,0,0-16-16H112A16,16,0,0,0,96,56v8h64ZM48,80V200H208V80H176V88a8,8,0,0,1-8,8H88a8,8,0,0,1-8-8V80Z"></path>
            </svg>
          </button>
        </div>

        {#if form.locality === titleCasedStates.locality.titleCased && titleCasedStates.locality.titleCased !== ""}
          <button
            type="button"
            onclick={() => undoTitleCaseField(form, titleCasedStates, "locality")}
            data-i18n-key="undo-title-case"
            title={t("undo-title-case", "Undo Casing")}
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
            onclick={() => titleCaseField(form, titleCasedStates, "locality")}
            data-i18n-key="title-case"
            title={t("title-case", "Proper Case")}
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

    <!-- Coordinates Field -->
    <div class="col-span-4">
      <label for="capture-verbatimCoordinates" data-i18n-key="verbatim-coordinates-label" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">
        {t("verbatim-coordinates-label", "Coordinates")}
        {#if coordinatesError}
          <span data-i18n-key="coord-error-badge" class="text-[9px] text-red-500 font-bold ml-1 uppercase">{t("coord-error-badge", "(Invalid)")}</span>
        {/if}
      </label>
      <input
        id="capture-verbatimCoordinates"
        data-i18n-key="verbatim-coordinates-placeholder"
        type="text"
        placeholder={t("verbatim-coordinates-placeholder", "eg 20°34'S, 47°12'E")}
        bind:value={form.verbatimCoordinates}
        onblur={handleCoordinatesBlur}
        class="w-full bg-white border text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all {coordinatesError ? 'border-red-500 bg-red-50 focus:border-red-500 focus:ring-red-500' : 'border-slate-300'}"
      />
      {#if form.decimalLatitude && form.decimalLongitude}
        <div class="text-[10px] text-slate-400 mt-1">
          {(Number(form.decimalLatitude)).toFixed(6)}, {(Number(form.decimalLongitude)).toFixed(6)}
        </div>
      {/if}
    </div>
  </div>

  <div class="grid grid-cols-12 gap-3">
    <!-- Locality Notes Field -->
    <div class="col-span-12">
      <label for="capture-locationNotes" data-i18n-key="location-notes-label" class="block text-xs font-semibold text-slate-655 uppercase tracking-wider mb-1">{t("location-notes-label", "Locality Notes")}</label>
      <div class="relative flex items-center">
        <textarea
          id="capture-locationNotes"
          data-i18n-key="location-notes-placeholder"
          placeholder={t("location-notes-placeholder", "eg 'Found in humid forest'")}
          bind:value={form.locationNotes}
          bind:this={locationNotesRef}
          rows="2"
          class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all pr-24"
        ></textarea>
        
        <!-- Clipboard Buttons -->
        <div class="absolute right-10 bottom-3.5 flex items-center gap-1 z-10">
          <button
            type="button"
            onclick={handleCopyLocationNotes}
            title={t("copy-location-notes-title", "Copy Notes")}
            class="p-1 text-slate-400 hover:text-slate-600 transition-colors cursor-pointer rounded hover:bg-slate-100"
            tabindex="-1"
          >
            {#if locationNotesCopied}
              <svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256" class="w-3.5 h-3.5 text-emerald-600">
                <path d="M229.66,77.66l-128,128a8,8,0,0,1-11.32,0l-56-56a8,8,0,0,1,11.32-11.32L102,192.69,218.34,76.34a8,8,0,0,1,11.32,11.32Z"></path>
              </svg>
            {:else}
              <svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256" class="w-3.5 h-3.5">
                <path d="M200,32H163.74a47.92,47.92,0,0,0-71.48,0H56A16,16,0,0,0,40,48V208a16,16,0,0,0,16,16H200a16,16,0,0,0,16-16V48A16,16,0,0,0,200,32Zm-72,0a32,32,0,0,1,32,32H96A32,32,0,0,1,128,32Zm72,176H56V48H80v8a8,8,0,0,0,8,8h80a8,8,0,0,0,8-8V48h24V208Z"></path>
              </svg>
            {/if}
          </button>
          <button
            type="button"
            onclick={handlePasteLocationNotes}
            title={t("paste-location-notes-title", "Paste Notes")}
            class="p-1 text-slate-400 hover:text-slate-600 transition-colors cursor-pointer rounded hover:bg-slate-100"
            tabindex="-1"
          >
            <svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256" class="w-3.5 h-3.5">
              <path d="M224,80V200a16,16,0,0,1-16,16H48a16,16,0,0,1-16-16V80A16,16,0,0,1,48,64H80V56A32,32,0,0,1,112,24h32a32,32,0,0,1,32,32v8h32A16,16,0,0,1,224,80ZM160,56a16,16,0,0,0-16-16H112A16,16,0,0,0,96,56v8h64ZM48,80V200H208V80H176V88a8,8,0,0,1-8,8H88a8,8,0,0,1-8,8V80Z"></path>
            </svg>
          </button>
        </div>

        {#if form.locationNotes === titleCasedStates.locationNotes.titleCased && titleCasedStates.locationNotes.titleCased !== ""}
          <button
            type="button"
            onclick={() => undoTitleCaseField(form, titleCasedStates, "locationNotes")}
            data-i18n-key="undo-title-case"
            title={t("undo-title-case", "Undo Casing")}
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
            onclick={() => titleCaseField(form, titleCasedStates, "locationNotes")}
            data-i18n-key="title-case-notes"
            title={t("title-case-notes", "Title case Locality Notes")}
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
  </div>

  <div class="grid grid-cols-12 gap-3">
    <!-- Verbatim Locality Field -->
    <div class="col-span-12">
      <label for="capture-verbatimLocality" data-i18n-key="verbatim-locality-label" class="block text-xs font-semibold text-slate-650 uppercase tracking-wider mb-1">{t("verbatim-locality-label", "Verbatim Locality (copy data above)")}</label>
      <div class="relative flex items-center">
        <textarea
          id="capture-verbatimLocality"
          data-i18n-key="verbatim-locality-placeholder"
          placeholder={t("verbatim-locality-placeholder", "Select any portion above and click 'copy' to populate")}
          bind:value={form.verbatimLocality}
          bind:this={verbatimLocalityRef}
          rows="3"
          class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all pr-12"
        ></textarea>
        <button
          type="button"
          onclick={handleCopyVerbatimLocality}
          title={t("copy-verbatim-locality-title", "Copy selected text to clipboard")}
          class="absolute right-2 bottom-3.5 p-1 text-slate-400 hover:text-slate-600 transition-colors cursor-pointer rounded hover:bg-slate-100"
          tabindex="-1"
        >
          {#if verbatimLocalityCopied}
            <svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256" class="w-4 h-4 text-emerald-600">
              <path d="M229.66,77.66l-128,128a8,8,0,0,1-11.32,0l-56-56a8,8,0,0,1,11.32-11.32L102,192.69,218.34,76.34a8,8,0,0,1,11.32,11.32Z"></path>
            </svg>
          {:else}
            <svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256" class="w-4 h-4">
              <path d="M200,32H163.74a47.92,47.92,0,0,0-71.48,0H56A16,16,0,0,0,40,48V208a16,16,0,0,0,16,16H200a16,16,0,0,0,16-16V48A16,16,0,0,0,200,32Zm-72,0a32,32,0,0,1,32,32H96A32,32,0,0,1,128,32Zm72,176H56V48H80v8a8,8,0,0,0,8,8h80a8,8,0,0,0,8-8V48h24V208Z"></path>
            </svg>
          {/if}
        </button>
      </div>
    </div>
  </div>

  <div class="grid grid-cols-12 gap-3">
    <!-- Verbatim Elevation Field -->
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

    <!-- Habitat Field -->
    <div class="col-span-8">
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
        {#if form.habitat === titleCasedStates.habitat.titleCased && titleCasedStates.habitat.titleCased !== ""}
          <button
            type="button"
            onclick={() => undoTitleCaseField(form, titleCasedStates, "habitat")}
            data-i18n-key="undo-title-case"
            title={t("undo-title-case", "Undo Casing")}
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
            onclick={() => titleCaseField(form, titleCasedStates, "habitat")}
            data-i18n-key="title-case-habitat"
            title={t("title-case-habitat", "Title case Habitat")}
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
  </div>
</div>
