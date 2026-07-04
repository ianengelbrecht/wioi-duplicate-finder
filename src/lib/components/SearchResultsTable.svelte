<script>
  import { getContext } from "svelte";
  import { formatISO8601Date } from "../utils/formatISO8601Date.js";

  const t = getContext("t");

  /**
   * @typedef {Object} SearchResultsTableProps
   * @property {any[]} results - List of reference records to display.
   * @property {boolean} searching - Whether search is currently running.
   * @property {string} searchMessage - Status message about the search result counts.
   * @property {string} errorMessage - Error message from the taxonomy search.
   * @property {(record: any) => void} onSelectRecord - Callback when a record is clicked.
   */

  /** @type {SearchResultsTableProps} */
  let {
    results = [],
    searching = false,
    searchMessage = "",
    errorMessage = "",
    onSelectRecord = () => {}
  } = $props();
</script>

<div class="flex-1 flex flex-col min-h-0">
  {#if errorMessage}
    <div class="p-4 text-sm text-red-600 bg-red-50 border-b border-red-200 shrink-0">
      Error: {errorMessage}
    </div>
  {/if}
  
  {#if searchMessage}
    <div class="px-4 py-2 text-[10px] uppercase font-bold tracking-wider text-slate-500 bg-slate-50 border-b border-slate-200 shrink-0">
      {searchMessage}
    </div>
  {/if}

  {#if results.length > 0}
    <div class="overflow-auto flex-1 min-h-0">
      <table class="w-full text-left text-xs border-collapse">
        <thead>
          <tr class="border-b border-slate-300 text-slate-600 font-bold uppercase tracking-wider">
            <th data-i18n-key="herb-col" class="p-3 sticky top-0 bg-slate-100 z-10">{t("herb-col", "HERB")}</th>
            <th data-i18n-key="collector-col" class="p-3 sticky top-0 bg-slate-100 z-10">{t("collector-col", "Collector")}</th>
            <th data-i18n-key="taxon-col" class="p-3 sticky top-0 bg-slate-100 z-10">{t("taxon-col", "Taxon Name")}</th>
            <th data-i18n-key="locality-col" class="p-3 sticky top-0 bg-slate-100 z-10">{t("locality-col", "Locality")}</th>
            <th data-i18n-key="geom-col" class="p-3 sticky top-0 bg-slate-100 z-10">{t("geom-col", "Geo")}</th>
            <th data-i18n-key="date-col" class="p-3 sticky top-0 bg-slate-100 z-10">{t("date-col", "Date")}</th>
            <th data-i18n-key="coords-col" class="p-3 sticky top-0 bg-slate-100 z-10">{t("coords-col", "Coords")}</th>
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
                {#if rec.fieldNumber}
                  <span class="text-[10px] text-slate-400 font-semibold block truncate">({rec.fieldNumber})</span>
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
                {rec.locality || rec.locationNotes || 'No locality information'}
                {#if rec.locality && rec.locationNotes}
                  <span class="text-[10px] text-slate-400 italic block">{rec.locationNotes}</span>
                {/if}
              </td>
              
              <!-- Country details -->
              <td class="p-3 text-slate-500">
                {rec.country || ''}
                {#if rec.stateProvince}
                  <span class="text-[10px] text-slate-400 block">{rec.stateProvince}</span>
                {/if}
                {#if rec.islandGroup || rec.island}
                  <span class="text-[10px] text-slate-400 block">
                    {rec.islandGroup || ''} {rec.island ? `(${rec.island})` : ''}
                  </span>
                {/if}
              </td>
              
              <!-- Date -->
              <td class="p-3 text-slate-600 whitespace-nowrap">
                {formatISO8601Date(rec.year, rec.month, rec.day) || 'N/A'}
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
      <span data-i18n-key="active-search-query" class="text-sm font-semibold uppercase tracking-wider text-slate-400 mb-1">{t("active-search-query", "No Search Query Active")}</span>
      <span data-i18n-key="search-query-instructions" class="text-xs text-slate-400 max-w-xs leading-relaxed">{t("search-query-instructions", "Enter search filters above and click Search. Use shortcuts like Ctrl+Enter.")}</span>
    </div>
  {/if}
</div>
