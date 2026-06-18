<script>
  import { getContext } from "svelte";
  import Autocomplete from "./Autocomplete.svelte";
  import { geographyService } from "../services/geographyService.js";

  const t = getContext("t");

  /**
   * @typedef {Object} FiltersType
   * @property {string} recordedBy
   * @property {string} recordNumber
   * @property {string} locality
   * @property {string} scientificName
   * @property {string} family
   * @property {string} country
   * @property {string} stateProvince
   * @property {number|string} year
   * @property {number|string} month
   * @property {number|string} day
   */

  /**
   * @typedef {Object} SearchFiltersProps
   * @property {FiltersType} filters - Bindable search filter values.
   * @property {boolean} [searchIsValid] - Bindable validation status of the form.
   * @property {boolean} searching - Indicates whether a search is currently in progress.
   * @property {() => void} onSearch - Callback triggered when search is submitted.
   * @property {() => void} onClear - Callback triggered when clear is requested.
   */

  /** @type {SearchFiltersProps} */
  let {
    filters = $bindable(),
    searchIsValid = $bindable(false),
    searching = false,
    onSearch = () => {},
    onClear = () => {}
  } = $props();

  let countrySuggestions = $state(/** @type {string[]} */ ([]));
  let stateProvinceSuggestions = $state(/** @type {string[]} */ ([]));

  // Search constraints validation (derived state)
  let hasRecordedBy = $derived(filters.recordedBy.trim().length > 0);
  let hasRecordNumber = $derived(filters.recordNumber.trim().length > 0);
  let hasLocality = $derived(filters.locality.trim().length > 0);
  let hasScientificName = $derived(filters.scientificName.trim().length > 0);
  let hasFamily = $derived(filters.family.trim().length > 0);
  let hasCountry = $derived(filters.country.trim().length > 0);
  let hasStateProvince = $derived(filters.stateProvince.trim().length > 0);
  
  let hasYear = $derived(filters.year !== "");
  let hasMonth = $derived(filters.month !== "");
  let hasDay = $derived(filters.day !== "");
  let hasDate = $derived(hasYear || hasMonth || hasDay);
  
  let hasOther = $derived(hasFamily || hasScientificName || hasCountry || hasStateProvince || hasLocality);
  
  let nonDateFieldsCount = $derived(
    [hasRecordedBy, hasRecordNumber, hasFamily, hasScientificName, hasCountry, hasStateProvince, hasLocality]
      .filter(Boolean).length
  );
  
  let totalFilledCount = $derived(
    [hasRecordedBy, hasRecordNumber, hasYear, hasMonth, hasDay, hasFamily, hasScientificName, hasCountry, hasStateProvince, hasLocality]
      .filter(Boolean).length
  );

  let collectorRuleOk = $derived(!hasRecordedBy || hasRecordNumber || (hasDate && hasOther));
  let recordNumberRuleOk = $derived(!hasRecordNumber || hasRecordedBy);
  let dateRuleOk = $derived(!hasDate || (nonDateFieldsCount >= 2));
  let tglRuleOk = $derived(!hasOther || (totalFilledCount >= 3));

  let calculatedIsValid = $derived(totalFilledCount > 0 && collectorRuleOk && recordNumberRuleOk && dateRuleOk && tglRuleOk);

  // Sync validity state back to parent
  $effect(() => {
    searchIsValid = calculatedIsValid;
  });

  function onCountryChanged() {
    filters.stateProvince = "";
    stateProvinceSuggestions = [];
  }

  async function handleCountryInput(/** @type {string} */ val) {
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

  async function handleStateProvinceInput(/** @type {string} */ val) {
    if (!val || val.trim().length === 0) {
      stateProvinceSuggestions = [];
      return;
    }
    try {
      stateProvinceSuggestions = await geographyService.autocompleteGeography("stateProvince", val, filters.country, "", "");
    } catch (e) {
      console.error(e);
    }
  }

  // Keyboard shortcut listener (Ctrl+Enter to trigger search)
  function handleGlobalKeyDown(/** @type {any} */ e) {
    if (e.key === "Enter" && e.ctrlKey) {
      e.preventDefault();
      if (calculatedIsValid) {
        onSearch();
      }
    }
  }

  $effect(() => {
    window.addEventListener("keydown", handleGlobalKeyDown);
    return () => {
      window.removeEventListener("keydown", handleGlobalKeyDown);
    };
  });
</script>

<div class="p-4 border-b border-slate-300 bg-slate-50">
  <div class="flex flex-col gap-3">
    <!-- Row 1: Collector, Collector No, Year, Month, Day (narrow date inputs) -->
    <div class="grid grid-cols-12 gap-3">
      <div class="col-span-4">
        <label for="search-recordedBy" data-i18n-key="search-collector-label" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">{t("search-collector-label", "Collector")}</label>
        <input
          id="search-recordedBy"
          data-i18n-key="search-collector-placeholder"
          type="text"
          placeholder={t("search-collector-placeholder", "Partial eg 'Raza' (no initials)")}
          bind:value={filters.recordedBy}
          class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
        />
      </div>
      <div class="col-span-2">
        <label for="search-recordNumber" data-i18n-key="record-number-label" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">{t("record-number-label", "Number")}</label>
        <input
          id="search-recordNumber"
          data-i18n-key="search-record-number-placeholder"
          type="text"
          placeholder={t("search-record-number-placeholder", "eg 1042")}
          bind:value={filters.recordNumber}
          class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
        />
      </div>
      <div class="col-span-5 flex gap-2">
        <div class="flex-1">
          <label for="search-year" data-i18n-key="year-label" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">{t("year-label", "Year")}</label>
          <input
            id="search-year"
            type="number"
            bind:value={filters.year}
            class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
          />
        </div>
        <div class="flex-1">
          <label for="search-month" data-i18n-key="month-label" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">{t("month-label", "Month")}</label>
          <input
            id="search-month"
            type="number"
            min="1"
            max="12"
            bind:value={filters.month}
            class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
          />
        </div>
        <div class="flex-1">
          <label for="search-day" data-i18n-key="day-label" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">{t("day-label", "Day")}</label>
          <input
            id="search-day"
            type="number"
            min="1"
            max="31"
            bind:value={filters.day}
            class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
          />
        </div>
      </div>
    </div>

    <!-- Row 2: Family, Scientific Name -->
    <div class="grid grid-cols-12 gap-3">
      <div class="col-span-12 sm:col-span-4">
        <label for="search-family" data-i18n-key="search-family-label" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">{t("search-family-label", "Family")}</label>
        <input
          id="search-family"
          data-i18n-key="search-family-placeholder"
          type="text"
          placeholder={t("search-family-placeholder", "eg 'mal' for Malvaceae")}
          bind:value={filters.family}
          class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
        />
      </div>
      <div class="col-span-12 sm:col-span-8">
        <label for="search-scientificName" data-i18n-key="scientific-name-label" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">{t("scientific-name-label", "Scientific Name")}</label>
        <input
          id="search-scientificName"
          data-i18n-key="scientific-name-placeholder"
          type="text"
          placeholder={t("scientific-name-placeholder", "Partial search eg 'ab man'")}
          bind:value={filters.scientificName}
          class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
        />
      </div>
    </div>

    <!-- Row 3: Country, State Province (Admin 2), Locality -->
    <div class="grid grid-cols-12 gap-3">
      <div class="col-span-12 sm:col-span-3">
        <label for="search-country" data-i18n-key="country-label" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">{t("country-label", "Country")}</label>
        <Autocomplete
          id="search-country"
          label=""
          placeholder="Partial ex. Mad"
          placeholderKey="search-country-placeholder"
          bind:value={filters.country}
          suggestions={countrySuggestions}
          oninput={handleCountryInput}
          onselect={onCountryChanged}
          delay={300}
        />
      </div>
      <div class="col-span-12 sm:col-span-3">
        <label for="search-stateProvince" data-i18n-key="state-province-label" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">
          {t("state-province-label", "Admin 2")} 
          <span data-i18n-key="state-province-sub" class="text-[70%]">{t("state-province-sub", "(state/prov/etc)")}</span>
        </label>
        <Autocomplete
          id="search-stateProvince"
          label=""
          placeholder="Partial eg 'Itas'"
          placeholderKey="search-admin2-placeholder"
          bind:value={filters.stateProvince}
          suggestions={stateProvinceSuggestions}
          oninput={handleStateProvinceInput}
          delay={300}
        />
      </div>
      <div class="col-span-12 sm:col-span-6">
        <label for="search-locality" data-i18n-key="locality-label" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">{t("locality-label", "Locality")}</label>
        <input
          id="search-locality"
          data-i18n-key="search-locality-placeholder"
          type="text"
          placeholder={t("search-locality-placeholder", "Partial search eg 'Anta ré'")}
          bind:value={filters.locality}
          class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
        />
      </div>
    </div>
  </div>

  <!-- Constraints Warning Flags -->
  {#if hasRecordedBy && !collectorRuleOk}
    <div data-i18n-key="search-warn-collector" class="mt-3 text-xs bg-amber-50 border-l-2 border-amber-500 text-amber-700 px-3 py-2 font-medium">
      {t("search-warn-collector", "⚠️ Collector search requires a collector number, or if just a collector and a date field, it also requires at least one of (family, scientific name, country, Admin 1, or locality).")}
    </div>
  {/if}
  {#if hasRecordNumber && !recordNumberRuleOk}
    <div data-i18n-key="search-warn-record-num" class="mt-3 text-xs bg-amber-50 border-l-2 border-amber-500 text-amber-700 px-3 py-2 font-medium">
      {t("search-warn-record-num", "⚠️ Collector number always requires a collector name, regardless of other fields.")}
    </div>
  {/if}
  {#if hasDate && !dateRuleOk}
    <div data-i18n-key="search-warn-date" class="mt-3 text-xs bg-amber-50 border-l-2 border-amber-500 text-amber-700 px-3 py-2 font-medium">
      {t("search-warn-date", "⚠️ Searches on year, month, or day require at least two other non-date fields.")}
    </div>
  {/if}
  {#if hasOther && !tglRuleOk}
    <div data-i18n-key="search-warn-other" class="mt-3 text-xs bg-amber-50 border-l-2 border-amber-500 text-amber-700 px-3 py-2 font-medium">
      {t("search-warn-other", "⚠️ Searching on family, scientific name, country, Admin 1, or locality requires at least two other fields (total of 3 or more fields).")}
    </div>
  {/if}

  <!-- Search Buttons -->
  <div class="mt-4 flex justify-between items-center">
    <span data-i18n-key="search-shortcut" class="text-[10px] text-slate-400 font-semibold uppercase">{t("search-shortcut", "Shortcut: Ctrl+Enter to search")}</span>
    <div class="flex gap-2">
      <button
        type="button"
        data-i18n-key="search-clear-btn"
        onclick={onClear}
        class="bg-slate-200 hover:bg-slate-300 text-slate-700 px-4 py-2 text-xs font-bold uppercase tracking-wider rounded-none transition-colors"
      >
        {t("search-clear-btn", "Clear")}
      </button>
      <button
        type="button"
        data-i18n-key={searching ? "search-searching" : "search-btn"}
        onclick={onSearch}
        disabled={!calculatedIsValid || searching}
        class="bg-slate-800 hover:bg-slate-900 text-white disabled:bg-slate-300 disabled:text-slate-500 disabled:cursor-not-allowed px-6 py-2 text-xs font-bold uppercase tracking-wider rounded-none transition-colors flex items-center gap-2"
      >
        {#if searching}
          <span>{t("search-searching", "Searching...")}</span>
        {:else}
          <span>{t("search-btn", "Search Database")}</span>
        {/if}
      </button>
    </div>
  </div>
</div>
