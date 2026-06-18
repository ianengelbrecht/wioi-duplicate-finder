<script>
  import { getContext } from "svelte";
  import SearchFilters from "./SearchFilters.svelte";
  import SearchResultsTable from "./SearchResultsTable.svelte";
  import { taxonomyService } from "../services/taxonomyService.js";
  import { geographyService } from "../services/geographyService.js";

  const t = getContext("t");

  /**
   * @typedef {Object} SearchPaneProps
   * @property {(record: any) => void} onSelectRecord - Callback when a record is selected.
   */

  /** @type {SearchPaneProps} */
  let {
    onSelectRecord = () => {}
  } = $props();

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
  let searchIsValid = $state(false);

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
      let data = await taxonomyService.searchReference(searchFilters);
      results = data;
      if (results.length === 0) {
        searchMessage = t("search-no-results", "No matches found.");
      } else if (results.length >= 250) {
        searchMessage = t("search-results-capped", "Found 250+ matches (results capped at 250 to maintain performance).");
      } else {
        searchMessage = `${t("search-results-found-prefix", "Found")} ${results.length} ${t("search-results-found-suffix", "matching reference records.")}`;
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

  export function clearSearch() {
    handleClear();
  }

  let gbifCount = $state(/** @type {number|null} */ (null));
  let wcvpCount = $state(/** @type {number|null} */ (null));
  let formattedGbifCount = $derived(gbifCount !== null ? gbifCount.toLocaleString() : "...");
  let formattedWcvpCount = $derived(wcvpCount !== null ? wcvpCount.toLocaleString() : "...");

  async function fetchCounts() {
    try {
      const counts = await geographyService.getTableCounts();
      gbifCount = counts.gbif;
      wcvpCount = counts.wcvp;
    } catch (e) {
      console.error("Failed to fetch table counts:", e);
    }
  }

  $effect(() => {
    fetchCounts();
  });
</script>

<div class="flex flex-col h-full bg-white border border-slate-300">
  <!-- Header Title -->
  <div class="px-4 py-3 bg-slate-100 border-b border-slate-300 flex justify-between items-center">
    <h2 data-i18n-key="search-heading" class="text-sm font-bold text-slate-800 uppercase tracking-wide">{t("search-heading", "Search Existing Specimens")}</h2>
    <div class="flex gap-2">
      <span data-i18n-key="reference-data" class="text-[10px] text-slate-500 font-semibold bg-slate-200 px-2 py-0.5 uppercase">{t("reference-data", "Reference Data")} ({formattedGbifCount} {t("records-count", "Records")})</span>
      <span data-i18n-key="wcvp-version" class="text-[10px] text-slate-500 font-semibold bg-slate-200 px-2 py-0.5 uppercase">{t("wcvp-version", "WCVP v12")} ({formattedWcvpCount} {t("taxa-count", "Taxa")})</span>
    </div>
  </div>

  <!-- Search Filter Form -->
  <SearchFilters
    bind:filters={filters}
    bind:searchIsValid={searchIsValid}
    searching={searching}
    onSearch={handleSearch}
    onClear={handleClear}
  />

  <!-- Search Results Listing -->
  <SearchResultsTable
    results={results}
    searching={searching}
    searchMessage={searchMessage}
    errorMessage={errorMessage}
    onSelectRecord={onSelectRecord}
  />
</div>
