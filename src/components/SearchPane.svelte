<script>
  import { invoke } from "@tauri-apps/api/core";
  import Autocomplete from "./Autocomplete.svelte";

  let {
    onSelectRecord = () => {}
  } = $props();

  let countrySuggestions = $state(/** @type {string[]} */ ([]));
  let stateProvinceSuggestions = $state(/** @type {string[]} */ ([]));

  let filters = $state({
    recordedBy: "",
    recordNumber: "",
    locality: "",
    scientificName: "",
    family: "",
    country: "",
    stateProvince: "",
    year: "",
    month: "",
    day: ""
  });

  let results = $state(/** @type {any[]} */ ([]));
  let searching = $state(false);
  let errorMessage = $state("");
  let searchMessage = $state("");

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

  let searchIsValid = $derived(totalFilledCount > 0 && collectorRuleOk && recordNumberRuleOk && dateRuleOk && tglRuleOk);

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
    if (!val || val.trim().length === 0) {
      stateProvinceSuggestions = [];
      return;
    }
    try {
      stateProvinceSuggestions = /** @type {any[]} */ (await invoke("autocomplete_geography", {
        field: "stateProvince",
        query: val,
        country: filters.country,
        stateProvince: "",
        county: ""
      }));
    } catch (e) {
      console.error(e);
    }
  }

  async function handleSearch() {
    if (!searchIsValid) return;
    
    searching = true;
    errorMessage = "";
    searchMessage = "";
    
    // Package year, month, day safely
    let searchFilters = /** @type {any} */ ({ ...filters });
    searchFilters.year = filters.year !== "" ? parseInt(filters.year) : null;
    searchFilters.month = filters.month !== "" ? parseInt(filters.month) : null;
    searchFilters.day = filters.day !== "" ? parseInt(filters.day) : null;
    
    try {
      let data = await invoke("search_reference", { filters: searchFilters });
      results = /** @type {any[]} */ (data);
      if (results.length === 0) {
        searchMessage = "No matching records found.";
      } else if (results.length >= 250) {
        searchMessage = "Found 250+ matches (results capped at 250 to maintain performance).";
      } else {
        searchMessage = `Found ${results.length} matching reference records.`;
      }
    } catch (err) {
      errorMessage = (/** @type {any} */ (err)).toString();
      results = [];
    } finally {
      searching = false;
    }
  }

  function handleClear() {
    filters = {
      recordedBy: "",
      recordNumber: "",
      locality: "",
      scientificName: "",
      family: "",
      country: "",
      stateProvince: "",
      year: "",
      month: "",
      day: ""
    };
    results = [];
    errorMessage = "";
    searchMessage = "";
  }

  // ISO8601 formatting for partial dates (e.g. YYYY, YYYY-MM, YYYY-MM-DD)
  /**
   * @param {number|string|null} year
   * @param {number|string|null} month
   * @param {number|string|null} day
   */
  function formatISO8601Date(year, month, day) {
    if (!year) return "N/A";
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

  // Keyboard shortcut listener (Ctrl+Enter to trigger search)
  function handleGlobalKeyDown(/** @type {any} */ e) {
    if (e.key === "Enter" && e.ctrlKey) {
      e.preventDefault();
      if (searchIsValid) {
        handleSearch();
      }
    }
  }

  let gbifCount = $state(/** @type {number|null} */ (null));
  let wcvpCount = $state(/** @type {number|null} */ (null));
  let formattedGbifCount = $derived(gbifCount !== null ? gbifCount.toLocaleString() : "...");
  let formattedWcvpCount = $derived(wcvpCount !== null ? wcvpCount.toLocaleString() : "...");

  async function fetchCounts() {
    try {
      const counts = /** @type {any} */ (await invoke("get_table_counts"));
      gbifCount = counts.gbif;
      wcvpCount = counts.wcvp;
    } catch (e) {
      console.error("Failed to fetch table counts:", e);
    }
  }

  $effect(() => {
    fetchCounts();
  });

  $effect(() => {
    window.addEventListener("keydown", handleGlobalKeyDown);
    return () => {
      window.removeEventListener("keydown", handleGlobalKeyDown);
    };
  });
</script>

<div class="flex flex-col h-full bg-white border border-slate-300">
  <!-- Header Title -->
  <div class="px-4 py-3 bg-slate-100 border-b border-slate-300 flex justify-between items-center">
    <h2 class="text-sm font-bold text-slate-800 uppercase tracking-wide">Search Existing Specimens</h2>
    <div class="flex gap-2">
      <span class="text-[10px] text-slate-500 font-semibold bg-slate-200 px-2 py-0.5 uppercase">Reference Data ({formattedGbifCount} Records)</span>
      <span class="text-[10px] text-slate-500 font-semibold bg-slate-200 px-2 py-0.5 uppercase">WCVP v12 ({formattedWcvpCount} Taxa)</span>
    </div>
  </div>

  <!-- Search Filter Form -->
  <div class="p-4 border-b border-slate-300 bg-slate-50">
    <div class="flex flex-col gap-3">
      <!-- Row 1: Collector, Collector No, Year, Month, Day (narrow date inputs) -->
      <div class="grid grid-cols-12 gap-3">
        <div class="col-span-3">
          <label for="search-recordedBy" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">Collector</label>
          <input
            id="search-recordedBy"
            type="text"
            placeholder="Partial eg 'Raza'"
            bind:value={filters.recordedBy}
            class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
          />
        </div>
        <div class="col-span-2">
          <label for="search-recordNumber" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">Collector No.</label>
          <input
            id="search-recordNumber"
            type="text"
            placeholder="eg 1042"
            bind:value={filters.recordNumber}
            class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
          />
        </div>
        <div class="col-span-4 flex gap-2">
          <div class="flex-1">
            <label for="search-year" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">Year</label>
            <input
              id="search-year"
              type="number"
              bind:value={filters.year}
              class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
            />
          </div>
          <div class="flex-1">
            <label for="search-month" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">Month</label>
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
            <label for="search-day" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">Day</label>
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
          <label for="search-family" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">Family</label>
          <input
            id="search-family"
            type="text"
            placeholder="e.g. Malvaceae"
            bind:value={filters.family}
            class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
          />
        </div>
        <div class="col-span-12 sm:col-span-8">
          <label for="search-scientificName" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">Scientific Name</label>
          <input
            id="search-scientificName"
            type="text"
            placeholder="Partial search eg 'ab man'"
            bind:value={filters.scientificName}
            class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
          />
        </div>
      </div>

      <!-- Row 3: Country, State Province (Admin 2), Locality -->
      <div class="grid grid-cols-12 gap-3">
        <div class="col-span-12 sm:col-span-3">
          <label for="search-country" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">Country</label>
          <Autocomplete
            id="search-country"
            label=""
            placeholder="Partial ex. Mad"
            bind:value={filters.country}
            suggestions={countrySuggestions}
            oninput={handleCountryInput}
            onselect={onCountryChanged}
            delay={300}
          />
        </div>
        <div class="col-span-12 sm:col-span-3">
          <label for="search-stateProvince" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">Admin 2 <span class="text-[70%]">(state/prov/etc)</span></label>
          <Autocomplete
            id="search-stateProvince"
            label=""
            placeholder="Partial eg 'Itas'"
            bind:value={filters.stateProvince}
            suggestions={stateProvinceSuggestions}
            oninput={handleStateProvinceInput}
            delay={300}
          />
        </div>
        <div class="col-span-12 sm:col-span-6">
          <label for="search-locality" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">Locality</label>
          <input
            id="search-locality"
            type="text"
            placeholder="Partial search eg 'Anta ré'"
            bind:value={filters.locality}
            class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
          />
        </div>
      </div>
    </div>

    <!-- Constraints Warning Flags -->
    {#if hasRecordedBy && !collectorRuleOk}
      <div class="mt-3 text-xs bg-amber-50 border-l-2 border-amber-500 text-amber-700 px-3 py-2 font-medium">
        ⚠️ Collector search requires a collector number, or if just a collector and a date field, it also requires at least one of (family, scientific name, country, Admin 1, or locality).
      </div>
    {/if}
    {#if hasRecordNumber && !recordNumberRuleOk}
      <div class="mt-3 text-xs bg-amber-50 border-l-2 border-amber-500 text-amber-700 px-3 py-2 font-medium">
        ⚠️ Collector number always requires a collector name, regardless of other fields.
      </div>
    {/if}
    {#if hasDate && !dateRuleOk}
      <div class="mt-3 text-xs bg-amber-50 border-l-2 border-amber-500 text-amber-700 px-3 py-2 font-medium">
        ⚠️ Searches on year, month, or day require at least two other non-date fields.
      </div>
    {/if}
    {#if hasOther && !tglRuleOk}
      <div class="mt-3 text-xs bg-amber-50 border-l-2 border-amber-500 text-amber-700 px-3 py-2 font-medium">
        ⚠️ Searching on family, scientific name, country, Admin 1, or locality requires at least two other fields (total of 3 or more fields).
      </div>
    {/if}

    <!-- Search Buttons -->
    <div class="mt-4 flex justify-between items-center">
      <span class="text-[10px] text-slate-400 font-semibold uppercase">Shortcut: Ctrl+Enter to search</span>
      <div class="flex gap-2">
        <button
          type="button"
          onclick={handleClear}
          class="bg-slate-200 hover:bg-slate-300 text-slate-700 px-4 py-2 text-xs font-bold uppercase tracking-wider rounded-none transition-colors"
        >
          Clear
        </button>
        <button
          type="button"
          onclick={handleSearch}
          disabled={!searchIsValid || searching}
          class="bg-slate-800 hover:bg-slate-900 text-white disabled:bg-slate-300 disabled:text-slate-500 disabled:cursor-not-allowed px-6 py-2 text-xs font-bold uppercase tracking-wider rounded-none transition-colors flex items-center gap-2"
        >
          {#if searching}
            <span>Searching...</span>
          {:else}
            <span>Search Database</span>
          {/if}
        </button>
      </div>
    </div>
  </div>

  <!-- Search Results Listing -->
  <div class="flex-1 overflow-y-auto min-h-0">
    {#if errorMessage}
      <div class="p-4 text-sm text-red-600 bg-red-50 border-b border-red-200">
        Error: {errorMessage}
      </div>
    {/if}
    
    {#if searchMessage}
      <div class="px-4 py-2 text-[10px] uppercase font-bold tracking-wider text-slate-500 bg-slate-50 border-b border-slate-200">
        {searchMessage}
      </div>
    {/if}

    {#if results.length > 0}
      <div class="overflow-x-auto">
        <table class="w-full text-left text-xs border-collapse">
          <thead>
            <tr class="bg-slate-100 border-b border-slate-300 text-slate-600 font-bold uppercase tracking-wider">
              <th class="p-3">HERB</th>
              <th class="p-3">Collector</th>
              <th class="p-3">Taxon Name</th>
              <th class="p-3">Locality</th>
              <th class="p-3">Geo</th>
              <th class="p-3">Date</th>
              <th class="p-3">Coords</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-slate-200">
            {#each results as rec}
              <tr 
                onclick={() => onSelectRecord(rec)}
                class="hover:bg-slate-50 cursor-pointer transition-colors border-b border-slate-100 group"
              >
                <!-- HERB -->
                <td class="p-3 text-slate-700 font-medium">
                  {rec.collectionCode || 'N/A'}
                </td>

                <!-- Collector Details -->
                <td class="p-3 text-slate-700 font-medium">
                  {rec.recordedBy || 'N/A'}
                  {#if rec.recordNumber}
                    <span class="text-[10px] text-slate-400 font-semibold block">#{rec.recordNumber}</span>
                  {/if}
                </td>
                
                <!-- Scientific Name -->
                <td class="p-3 text-slate-900 italic font-semibold">
                  {#if rec.identificationQualifier && rec.identificationQualifier.trim()}
                    <span class="not-italic">{rec.identificationQualifier.trim()} </span>
                  {/if}
                  {rec.scientificName || 'N/A'}
                  {#if rec.family}
                    <span class="text-[9px] text-slate-400 not-italic uppercase tracking-wider font-bold block">{rec.family}</span>
                  {/if}
                </td>
                
                <!-- Locality -->
                <td class="p-3 text-slate-600 max-w-xs truncate" title={rec.locality}>
                  {rec.locality || 'No locality information'}
                  {#if rec.locationNotes}
                    <span class="text-[10px] text-slate-400 italic block">{rec.locationNotes}</span>
                  {/if}
                </td>
                
                <!-- Country details -->
                <td class="p-3 text-slate-500">
                  {rec.country || ''}
                  {#if rec.stateProvince}
                    <span class="text-[10px] text-slate-400 block">{rec.stateProvince}</span>
                  {/if}
                </td>
                
                <!-- Date -->
                <td class="p-3 text-slate-600 whitespace-nowrap">
                  {formatISO8601Date(rec.year, rec.month, rec.day)}
                </td>

                <!-- Coordinates -->
                <td class="p-3 text-slate-500 whitespace-nowrap">
                  {#if rec.verbatimCoordinates}
                    {rec.verbatimCoordinates}
                  {:else if rec.decimalLatitude != null && rec.decimalLongitude != null}
                    {rec.decimalLatitude}, {rec.decimalLongitude}
                  {:else}
                    N/A
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {:else if !searching && !searchMessage}
      <div class="h-64 flex flex-col justify-center items-center text-slate-400 p-8 text-center">
        <svg xmlns="http://www.w3.org/2000/svg" class="h-8 w-8 mb-2 text-slate-300" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
        </svg>
        <span class="text-sm font-semibold uppercase tracking-wider text-slate-400 mb-1">No Search Query Active</span>
        <span class="text-xs text-slate-400 max-w-xs leading-relaxed">Enter search filters above and click Search. Use shortcuts like Ctrl+Enter.</span>
      </div>
    {/if}
  </div>
</div>
