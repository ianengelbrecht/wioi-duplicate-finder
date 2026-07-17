<script>
  import { getContext } from "svelte";
  import { workspaceStore } from "$lib/stores/workspaceStore.svelte.js";
  import { authStore } from "$lib/stores/authStore.svelte.js";
  import { specimenService } from "$lib/services/specimenService.js";
  import { sessionService } from "$lib/services/sessionService.js";
  import DeleteIcon from "$lib/components/icons/DeleteIcon.svelte";
  import CopyIcon from "$lib/components/icons/CopyIcon.svelte";
  import SortAscendingIcon from "$lib/components/icons/SortAscendingIcon.svelte";
  import SortDescendingIcon from "$lib/components/icons/SortDescendingIcon.svelte";
  import ListIcon from "$lib/components/icons/ListIcon.svelte";

  const t = getContext("t");

  // Custom delete record modal state (UI only)
  let showDeleteRecordModal = $state(false);
  let pendingDeleteRecordId = $state(/** @type {number|null} */ (null));
  let pendingDeleteRecordDetails = $state("");

  // Custom duplicate record modal state (UI only)
  let showDuplicateRecordModal = $state(false);
  let pendingDuplicateRecord = $state(/** @type {any} */ (null));
  let pendingDuplicateRecordDetails = $state("");

  // Sorting state (UI only)
  let sortField = $state(/** @type {"catalogNumber" | "recordedBy" | "scientificName" | "date" | null} */ (null));
  let sortDirection = $state(/** @type {"asc" | "desc" | "none"} */ ("none"));

  async function loadCapturedRecords() {
    if (!workspaceStore.activeSession) return;
    try {
      workspaceStore.capturedRecords = await specimenService.getCapturedRecords(workspaceStore.activeSession.id);
      workspaceStore.activeSession.recordCount = workspaceStore.capturedRecords.length;
    } catch (e) {
      console.error(e);
    }
  }

  async function loadSessions() {
    if (!authStore.currentUser) return;
    try {
      workspaceStore.sessionList = await sessionService.getSessions(authStore.currentUser.id);
    } catch (e) {
      console.error(e);
    }
  }

  function promptDeleteCapturedRecord(/** @type {any} */ rec, /** @type {any} */ e) {
    if (e) e.stopPropagation();
    pendingDeleteRecordId = rec.id;
    pendingDeleteRecordDetails = `${rec.recordedBy || 'N/A'} ${rec.recordNumber ? '#' + rec.recordNumber : ''} - ${rec.scientificName}`;
    showDeleteRecordModal = true;
  }

  async function confirmDeleteCapturedRecord() {
    if (pendingDeleteRecordId === null) return;
    
    const id = pendingDeleteRecordId;
    pendingDeleteRecordId = null;
    showDeleteRecordModal = false;
    
    try {
      await specimenService.deleteCapturedRecord(id);
      await loadCapturedRecords();
      await loadSessions(); // Reload stats count in dashboard background
    } catch (err) {
      alert("Error deleting record: " + (/** @type {any} */ (err)).toString());
    }
  }

  function cancelDeleteCapturedRecord() {
    pendingDeleteRecordId = null;
    showDeleteRecordModal = false;
  }

  /**
   * Prompts the user to confirm duplicating a captured record.
   * @param {any} rec
   * @param {any} e
   */
  function promptDuplicateCapturedRecord(rec, e) {
    if (e) e.stopPropagation();
    pendingDuplicateRecord = rec;
    pendingDuplicateRecordDetails = `${rec.recordedBy || 'N/A'} ${rec.recordNumber ? '#' + rec.recordNumber : ''} - ${rec.scientificName}`;
    showDuplicateRecordModal = true;
  }

  /**
   * Confirms duplication of the pending record.
   * Copies all fields to activeRecord, except catalogNumber which is cleared, and set id to null.
   */
  function confirmDuplicateCapturedRecord() {
    if (!pendingDuplicateRecord) return;
    
    // Create copy, clear catalogNumber, set id to null
    const duplicated = {
      ...pendingDuplicateRecord,
      id: null,
      catalogNumber: ""
    };
    
    workspaceStore.activeRecord = duplicated;
    
    cancelDuplicateCapturedRecord();
  }

  /**
   * Cancels the duplicate prompt.
   */
  function cancelDuplicateCapturedRecord() {
    pendingDuplicateRecord = null;
    showDuplicateRecordModal = false;
  }

  /**
   * Helper to format part of a date with zero padding if present.
   * @param {number|string|null|undefined} val
   * @returns {string}
   */
  function formatZeroPad(val) {
    if (val === null || val === undefined || val === "") return "?";
    const num = typeof val === "string" ? parseInt(val, 10) : val;
    if (isNaN(num)) return "?";
    return num.toString().padStart(2, "0");
  }

  /**
   * Toggles the sort direction for a given field and clears other active sorts.
   * @param {"catalogNumber" | "recordedBy" | "scientificName" | "date"} field
   */
  function toggleSort(field) {
    if (sortField !== field) {
      sortField = field;
      sortDirection = "asc";
    } else {
      if (sortDirection === "asc") {
        sortDirection = "desc";
      } else if (sortDirection === "desc") {
        sortDirection = "none";
        sortField = null;
      } else {
        sortDirection = "asc";
      }
    }
  }

  // Reactive derived array of records sorted by current state
  let sortedRecords = $derived.by(() => {
    const records = [...workspaceStore.capturedRecords];
    if (!sortField || sortDirection === "none") {
      return records;
    }

    records.sort((a, b) => {
      let comparison = 0;
      if (sortField === "catalogNumber") {
        const valA = a.catalogNumber || "";
        const valB = b.catalogNumber || "";
        comparison = valA.localeCompare(valB, undefined, { numeric: true, sensitivity: 'base' });
      } else if (sortField === "recordedBy") {
        const valA = a.recordedBy || "";
        const valB = b.recordedBy || "";
        comparison = valA.localeCompare(valB, undefined, { sensitivity: 'base' });
        // Secondary sort by recordNumber if collector names are identical
        if (comparison === 0) {
          const numA = parseInt(a.recordNumber, 10) || 0;
          const numB = parseInt(b.recordNumber, 10) || 0;
          comparison = numA - numB;
        }
      } else if (sortField === "scientificName") {
        const valA = a.scientificName || "";
        const valB = b.scientificName || "";
        comparison = valA.localeCompare(valB, undefined, { sensitivity: 'base' });
      } else if (sortField === "date") {
        const yrA = a.year !== null && a.year !== undefined ? parseInt(a.year, 10) : 0;
        const yrB = b.year !== null && b.year !== undefined ? parseInt(b.year, 10) : 0;
        comparison = yrA - yrB;
        if (comparison === 0) {
          const moA = a.month !== null && a.month !== undefined ? parseInt(a.month, 10) : 0;
          const moB = b.month !== null && b.month !== undefined ? parseInt(b.month, 10) : 0;
          comparison = moA - moB;
          if (comparison === 0) {
            const dyA = a.day !== null && a.day !== undefined ? parseInt(a.day, 10) : 0;
            const dyB = b.day !== null && b.day !== undefined ? parseInt(b.day, 10) : 0;
            comparison = dyA - dyB;
          }
        }
      }
      return sortDirection === "asc" ? comparison : -comparison;
    });

    return records;
  });
</script>

<div class="border-t border-slate-300 bg-white p-4 shrink-0 max-h-60 flex flex-col">
  <div class="flex justify-between items-center mb-2">
    <h3 data-i18n-key="specimens-saved-title" class="text-xs font-bold text-slate-800 uppercase tracking-wide">{t("specimens-saved-title", "Specimens Saved in this Session")}</h3>
    <span class="text-[10px] text-slate-400 font-semibold uppercase">{workspaceStore.capturedRecords.length} {t("records-count", "records")}</span>
  </div>

  <div class="flex-1 overflow-y-auto border border-slate-200">
    {#if workspaceStore.capturedRecords.length > 0}
      <table class="w-full text-left text-xs border-collapse">
        <thead>
          <tr class="bg-slate-50 border-b border-slate-350 text-slate-600 font-bold uppercase tracking-wider">
            <th data-i18n-key="barcode-col" class="p-2">
              <span class="flex justify-between items-center gap-1">
                {t("barcode-col", "Barcode")}
                <button
                  onclick={() => toggleSort("catalogNumber")}
                  title={sortField === "catalogNumber" ? (sortDirection === "asc" ? t("sort-desc-title", "Sort Descending") : t("sort-none-title", "Clear Sort")) : t("sort-asc-title", "Sort Ascending")}
                  class="w-4 h-4 transition-colors cursor-pointer {sortField === 'catalogNumber' ? 'text-slate-800' : 'text-slate-300 hover:text-slate-500'}"
                >
                  {#if sortField === "catalogNumber" && sortDirection === "asc"}
                    <SortAscendingIcon />
                  {:else if sortField === "catalogNumber" && sortDirection === "desc"}
                    <SortDescendingIcon />
                  {:else}
                    <ListIcon />
                  {/if}
                </button>
              </span>
            </th>
            <th data-i18n-key="collector-col" class="p-2">
              <span class="flex justify-between items-center gap-1">
                {t("collector-col", "Collector")}
                <button
                  onclick={() => toggleSort("recordedBy")}
                  title={sortField === "recordedBy" ? (sortDirection === "asc" ? t("sort-desc-title", "Sort Descending") : t("sort-none-title", "Clear Sort")) : t("sort-asc-title", "Sort Ascending")}
                  class="w-4 h-4 transition-colors cursor-pointer {sortField === 'recordedBy' ? 'text-slate-800' : 'text-slate-300 hover:text-slate-500'}"
                >
                  {#if sortField === "recordedBy" && sortDirection === "asc"}
                    <SortAscendingIcon />
                  {:else if sortField === "recordedBy" && sortDirection === "desc"}
                    <SortDescendingIcon />
                  {:else}
                    <ListIcon />
                  {/if}
                </button>
              </span>
            </th>
            <th data-i18n-key="taxon-col" class="p-2">
              <span class="flex justify-between items-center gap-1">
                {t("taxon-col", "Taxon Name")}
                <button
                  onclick={() => toggleSort("scientificName")}
                  title={sortField === "scientificName" ? (sortDirection === "asc" ? t("sort-desc-title", "Sort Descending") : t("sort-none-title", "Clear Sort")) : t("sort-asc-title", "Sort Ascending")}
                  class="w-4 h-4 transition-colors cursor-pointer {sortField === 'scientificName' ? 'text-slate-800' : 'text-slate-300 hover:text-slate-500'}"
                >
                  {#if sortField === "scientificName" && sortDirection === "asc"}
                    <SortAscendingIcon />
                  {:else if sortField === "scientificName" && sortDirection === "desc"}
                    <SortDescendingIcon />
                  {:else}
                    <ListIcon />
                  {/if}
                </button>
              </span>
            </th>
            <th data-i18n-key="locality-col" class="p-2">{t("locality-col", "Locality")}</th>
            <th data-i18n-key="geo-col" class="p-2">{t("geo-col", "Geo")}</th>
            <th data-i18n-key="date-col" class="p-2">
              <span class="flex justify-between items-center gap-1">
                {t("date-col", "Date")}
                <button
                  onclick={() => toggleSort("date")}
                  title={sortField === "date" ? (sortDirection === "asc" ? t("sort-desc-title", "Sort Descending") : t("sort-none-title", "Clear Sort")) : t("sort-asc-title", "Sort Ascending")}
                  class="w-4 h-4 transition-colors cursor-pointer {sortField === 'date' ? 'text-slate-800' : 'text-slate-300 hover:text-slate-500'}"
                >
                  {#if sortField === "date" && sortDirection === "asc"}
                    <SortAscendingIcon />
                  {:else if sortField === "date" && sortDirection === "desc"}
                    <SortDescendingIcon />
                  {:else}
                    <ListIcon />
                  {/if}
                </button>
              </span>
            </th>
            <th data-i18n-key="actions-col" class="p-2 text-right">{t("actions-col", "Actions")}</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-slate-100">
          {#each sortedRecords as rec}
            <tr 
              onclick={() => workspaceStore.activeRecord = { ...rec }}
              class="hover:bg-slate-50 cursor-pointer transition-colors border-b border-slate-100"
            >
              <td class="p-2 text-slate-700 font-semibold">{rec.catalogNumber || 'N/A'}</td>
              <td class="p-2 text-slate-700 font-medium">{rec.recordedBy || 'N/A'} {rec.recordNumber ? `#${rec.recordNumber}` : ''}</td>
              <td class="p-2 text-slate-900 italic font-semibold">{rec.scientificName}</td>
              <td class="p-2 text-slate-500 max-w-xs truncate" title={rec.locality}>{rec.locality || 'N/A'}</td>
              <td class="p-2 text-slate-500">
                {rec.country || ''} {rec.stateProvince || ''}
                {#if rec.islandGroup || rec.island}
                  <span class="text-[10px] text-slate-400 block">
                    {rec.islandGroup || ''} {rec.island ? `(${rec.island})` : ''}
                  </span>
                {/if}
              </td>
              <td class="p-2 text-slate-650">{rec.year ? `${rec.year}-${formatZeroPad(rec.month)}-${formatZeroPad(rec.day)}` : 'N/A'}</td>
              <td class="p-2">
                <div class="w-fit ml-auto flex gap-2">
                  <button
                    data-i18n-key="copy-btn"
                    onclick={(e) => promptDuplicateCapturedRecord(rec, e)}
                    title={t("duplicate-record", "Duplicate")}
                    class="bg-slate-50 hover:bg-slate-100 text-slate-650 border border-slate-200 px-2 py-0.5 text-[10px] uppercase font-bold tracking-wide transition-colors"
                  >
                    <div class="w-5 h-5" >
                      <CopyIcon />
                    </div>
                  </button>
                  <button
                    data-i18n-key="delete-btn"
                    onclick={(e) => promptDeleteCapturedRecord(rec, e)}
                    title={t("delete-btn", "Delete")}
                    class="bg-red-50 hover:bg-red-100 text-red-650 border border-red-200 px-2 py-0.5 text-[10px] uppercase font-bold tracking-wide transition-colors"
                  >
                    <div class="w-5 h-5" >
                      <DeleteIcon />
                    </div>
                  </button>
                </div>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {:else}
      <div data-i18n-key="no-records-session" class="py-8 text-center text-slate-400 text-xs">
        {t("no-records-session", "No specimens captured yet in this session. Start by entering data in the form above!")}
      </div>
    {/if}
  </div>
</div>

{#if showDeleteRecordModal}
  <div 
    class="fixed inset-0 z-[120] flex items-center justify-center bg-slate-900/40 backdrop-blur-xs p-4"
    role="dialog"
    aria-modal="true"
    tabindex="-1"
    onclick={(e) => { if (e.target === e.currentTarget) cancelDeleteCapturedRecord(); }}
    onkeydown={(e) => { 
      if (e.key === "Escape") {
        e.preventDefault();
        cancelDeleteCapturedRecord(); 
      } else if (e.key === "Enter") {
        e.preventDefault();
        confirmDeleteCapturedRecord();
      }
    }}
  >
    <div class="bg-white border border-slate-200 shadow-2xl max-w-sm w-full p-5 flex flex-col gap-4 rounded-none">
      <div class="flex items-start gap-3">
        <div class="p-2 bg-red-50 text-red-650 rounded-full shrink-0">
          <svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256" class="w-5 h-5">
            <path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm0,192a88,88,0,1,1,88-88A88.09,88.09,0,0,1,128,216Zm-8-80V80a8,8,0,0,1,16,0v56a8,8,0,0,1-16,0Zm20,36a12,12,0,1,1-12-12A12,12,0,0,1,140,172Z"></path>
          </svg>
        </div>
        <div class="space-y-1">
          <h3 data-i18n-key="delete-record-heading" class="font-bold text-red-700">{t("delete-record-heading", "Delete Specimen Record")}</h3>
          <p data-i18n-key="delete-record-confirm" class="text-sm text-slate-500 leading-relaxed">
            {t("delete-record-confirm", "Are you sure you want to permanently delete this captured record?")}
          </p>
          <p class="text-xs font-semibold text-slate-700 bg-slate-50 p-2 border border-slate-150 break-all">{pendingDeleteRecordDetails}</p>
        </div>
      </div>
      
      <div class="flex justify-end gap-2 mt-2">
        <button
          type="button"
          data-i18n-key="cancel-btn"
          onclick={cancelDeleteCapturedRecord}
          class="px-3 py-1.5 text-xs font-semibold text-slate-500 hover:bg-slate-50 border border-slate-200 transition-colors cursor-pointer rounded-none"
        >
          {t("cancel-btn", "Cancel")}
        </button>
        <button
          type="button"
          data-i18n-key="delete-btn"
          onclick={confirmDeleteCapturedRecord}
          class="px-3.5 py-1.5 text-xs font-semibold text-white bg-red-650 bg-red-400 hover:bg-red-700 transition-colors cursor-pointer rounded-none"
        >
          {t("delete-btn", "Delete")}
        </button>
      </div>
    </div>
  </div>
{/if}

{#if showDuplicateRecordModal}
  <div 
    class="fixed inset-0 z-[120] flex items-center justify-center bg-slate-900/40 backdrop-blur-xs p-4"
    role="dialog"
    aria-modal="true"
    tabindex="-1"
    onclick={(e) => { if (e.target === e.currentTarget) cancelDuplicateCapturedRecord(); }}
    onkeydown={(e) => { 
      if (e.key === "Escape") {
        e.preventDefault();
        cancelDuplicateCapturedRecord(); 
      } else if (e.key === "Enter") {
        e.preventDefault();
        confirmDuplicateCapturedRecord();
      }
    }}
  >
    <div class="bg-white border border-slate-200 shadow-2xl max-w-sm w-full p-5 flex flex-col gap-4 rounded-none">
      <div class="flex items-start gap-3">
        <div class="p-2 bg-slate-100 text-slate-700 rounded-full shrink-0">
          <svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256" class="w-5 h-5">
            <path d="M216,32H88a16,16,0,0,0-16,16v80a16,16,0,0,0,16,16h128a16,16,0,0,0,16-16V48A16,16,0,0,0,216,32Zm0,96H88V48H216v80ZM168,160a8,8,0,0,1-8,8H56V64a8,8,0,0,1,16,0V152H160A8,8,0,0,1,168,160Zm-32,32a8,8,0,0,1-8,8H40V96a8,8,0,0,1,16,0v88H128A8,8,0,0,1,136,192Z"></path>
          </svg>
        </div>
        <div class="space-y-1">
          <h3 data-i18n-key="duplicate-record-heading" class="font-bold text-slate-800">{t("duplicate-record-heading", "Duplicate Specimen Record")}</h3>
          <p data-i18n-key="duplicate-record-confirm" class="text-sm text-slate-500 leading-relaxed">
            {t("duplicate-record-confirm", "Are you sure you want to duplicate this record? The barcode/catalog number will be cleared and the new record will be loaded into the capture form.")}
          </p>
          <p class="text-xs font-semibold text-slate-700 bg-slate-50 p-2 border border-slate-150 break-all">{pendingDuplicateRecordDetails}</p>
        </div>
      </div>
      
      <div class="flex justify-end gap-2 mt-2">
        <button
          type="button"
          data-i18n-key="cancel-btn"
          onclick={cancelDuplicateCapturedRecord}
          class="px-3 py-1.5 text-xs font-semibold text-slate-500 hover:bg-slate-50 border border-slate-200 transition-colors cursor-pointer rounded-none"
        >
          {t("cancel-btn", "Cancel")}
        </button>
        <button
          type="button"
          data-i18n-key="duplicate-btn"
          onclick={confirmDuplicateCapturedRecord}
          class="px-3.5 py-1.5 text-xs font-semibold text-white bg-slate-800 hover:bg-slate-900 transition-colors cursor-pointer rounded-none"
        >
          {t("duplicate-btn", "Duplicate")}
        </button>
      </div>
    </div>
  </div>
{/if}
