<script>
  import { invoke } from "@tauri-apps/api/core";
  import Autocomplete from "./Autocomplete.svelte";

  let {
    sessionId = null,
    activeRecord = $bindable(null), // The selected record to edit (or empty for new)
    onSaveSuccess = () => {}
  } = $props();

  let form = $state({
    id: null,
    recordedBy: "",
    recordNumber: "",
    locality: "",
    locationNotes: "",
    verbatimLocality: "",
    scientificName: "",
    family: "",
    genus: "",
    specificEpithet: "",
    infraSpecificEpithet: "",
    country: "",
    stateProvince: "",
    year: "",
    month: "",
    day: ""
  });

  let saving = $state(false);
  let statusMessage = $state("");
  let statusType = $state(""); // "success" or "error"

  // Dropdown suggestions lists
  let taxonSuggestions = $state(/** @type {any[]} */ ([]));
  let localitySuggestions = $state(/** @type {any[]} */ ([]));
  let collectorSuggestions = $state(/** @type {any[]} */ ([]));

  // Watch activeRecord changes (when a search result is clicked, copy it to the form!)
  $effect(() => {
    if (activeRecord) {
      form.id = activeRecord.id && activeRecord.sessionId ? activeRecord.id : null; // Only reuse id if it is a previously captured record, not a reference database record
      form.recordedBy = activeRecord.recordedBy || "";
      form.recordNumber = activeRecord.recordNumber || "";
      form.locality = activeRecord.locality || "";
      form.locationNotes = activeRecord.locationNotes || "";
      form.verbatimLocality = activeRecord.verbatimLocality || "";
      form.scientificName = activeRecord.scientificName || "";
      form.family = activeRecord.family || "";
      form.genus = activeRecord.genus || "";
      form.specificEpithet = activeRecord.specificEpithet || "";
      form.infraSpecificEpithet = activeRecord.infraSpecificEpithet || "";
      form.country = activeRecord.country || "";
      form.stateProvince = activeRecord.stateProvince || "";
      form.year = activeRecord.year !== null && activeRecord.year !== undefined ? activeRecord.year.toString() : "";
      form.month = activeRecord.month !== null && activeRecord.month !== undefined ? activeRecord.month.toString() : "";
      form.day = activeRecord.day !== null && activeRecord.day !== undefined ? activeRecord.day.toString() : "";
      
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
    // Populate matching taxonomic fields automatically
    form.scientificName = sug.scientificName || "";
    if (sug.family) form.family = sug.family;
    if (sug.genus) form.genus = sug.genus;
    if (sug.specificEpithet) form.specificEpithet = sug.specificEpithet;
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
      year: form.year !== "" ? parseInt(form.year) : null,
      month: form.month !== "" ? parseInt(form.month) : null,
      day: form.day !== "" ? parseInt(form.day) : null
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
      recordedBy: "",
      recordNumber: "",
      locality: "",
      locationNotes: "",
      verbatimLocality: "",
      scientificName: "",
      family: "",
      genus: "",
      specificEpithet: "",
      infraSpecificEpithet: "",
      country: "",
      stateProvince: "",
      year: "",
      month: "",
      day: ""
    };
    activeRecord = null;
    statusMessage = "";
    taxonSuggestions = [];
    localitySuggestions = [];
    collectorSuggestions = [];
  }

  // Keyboard shortcut listener (Ctrl+S to save)
  function handleGlobalKeyDown(/** @type {any} */ e) {
    if (e.key === "s" && e.ctrlKey) {
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
  <div class="px-4 py-3 bg-slate-100 border-b border-slate-300 flex justify-between items-center">
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

    <!-- 1. Taxonomic Details Section -->
    <div class="space-y-3">
      <h3 class="text-[10px] font-bold text-slate-400 uppercase tracking-wider border-b border-slate-100 pb-1">Taxonomic Identification</h3>
      
      <!-- Scientific Name Autocomplete -->
      <div>
        <Autocomplete
          id="capture-scientificName"
          label="Scientific Name *"
          placeholder="Start typing scientific name..."
          bind:value={form.scientificName}
          suggestions={taxonSuggestions}
          oninput={handleTaxonInput}
          onselect={handleTaxonSelect}
        />
      </div>

      <div class="grid grid-cols-2 gap-3">
        <!-- Family -->
        <div>
          <label for="capture-family" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">Family</label>
          <input
            id="capture-family"
            type="text"
            placeholder="e.g. Malvaceae"
            bind:value={form.family}
            class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
          />
        </div>
        <!-- Genus -->
        <div>
          <label for="capture-genus" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">Genus</label>
          <input
            id="capture-genus"
            type="text"
            placeholder="e.g. Abelmoschus"
            bind:value={form.genus}
            class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
          />
        </div>
      </div>

      <div class="grid grid-cols-2 gap-3">
        <!-- Species Epithet -->
        <div>
          <label for="capture-specificEpithet" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">Specific Epithet</label>
          <input
            id="capture-specificEpithet"
            type="text"
            placeholder="e.g. manihot"
            bind:value={form.specificEpithet}
            class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
          />
        </div>
        <!-- Infraspecific Epithet -->
        <div>
          <label for="capture-infraSpecificEpithet" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">Infraspecific Epithet</label>
          <input
            id="capture-infraSpecificEpithet"
            type="text"
            placeholder="e.g. heleniana"
            bind:value={form.infraSpecificEpithet}
            class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
          />
        </div>
      </div>
    </div>

    <!-- 2. Collector Details Section -->
    <div class="space-y-3 pt-2">
      <h3 class="text-[10px] font-bold text-slate-400 uppercase tracking-wider border-b border-slate-100 pb-1">Collector Details</h3>
      
      <div class="grid grid-cols-3 gap-3">
        <!-- Recorded By Autocomplete -->
        <div class="col-span-2">
          <Autocomplete
            id="capture-recordedBy"
            label="Collector Name"
            placeholder="Search or enter collector name..."
            bind:value={form.recordedBy}
            suggestions={collectorSuggestions}
            oninput={handleCollectorInput}
          />
        </div>
        <!-- Record Number -->
        <div>
          <label for="capture-recordNumber" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">Collector No.</label>
          <input
            id="capture-recordNumber"
            type="text"
            placeholder="e.g. 1042"
            bind:value={form.recordNumber}
            class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
          />
        </div>
      </div>
    </div>

    <!-- 3. Locality Section -->
    <div class="space-y-3 pt-2">
      <h3 class="text-[10px] font-bold text-slate-400 uppercase tracking-wider border-b border-slate-100 pb-1">Geographic Locality</h3>
      
      <!-- Locality Autocomplete -->
      <div>
        <Autocomplete
          id="capture-locality"
          label="Locality"
          placeholder="Search or enter exact locality description..."
          bind:value={form.locality}
          suggestions={localitySuggestions}
          oninput={handleLocalityInput}
        />
      </div>

      <!-- Verbatim Locality -->
      <div>
        <label for="capture-verbatimLocality" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">Verbatim Locality</label>
        <input
          id="capture-verbatimLocality"
          type="text"
          placeholder="e.g. Kestell district, near riverbank"
          bind:value={form.verbatimLocality}
          class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
        />
      </div>

      <!-- Location Notes -->
      <div>
        <label for="capture-locationNotes" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">Location / Habitat Notes</label>
        <textarea
          id="capture-locationNotes"
          rows="2"
          placeholder="e.g. Moist loamy soil under shade, growing with ferns"
          bind:value={form.locationNotes}
          class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all resize-none"
        ></textarea>
      </div>

      <div class="grid grid-cols-2 gap-3">
        <!-- Country -->
        <div>
          <label for="capture-country" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">Country</label>
          <input
            id="capture-country"
            type="text"
            placeholder="e.g. South Africa"
            bind:value={form.country}
            class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
          />
        </div>
        <!-- State / Province -->
        <div>
          <label for="capture-stateProvince" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">State / Province</label>
          <input
            id="capture-stateProvince"
            type="text"
            placeholder="e.g. Free State"
            bind:value={form.stateProvince}
            class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
          />
        </div>
      </div>
    </div>

    <!-- 4. Date Captured Section -->
    <div class="space-y-3 pt-2 pb-6">
      <h3 class="text-[10px] font-bold text-slate-400 uppercase tracking-wider border-b border-slate-100 pb-1">Date Collected</h3>
      
      <div class="grid grid-cols-3 gap-3">
        <!-- Year -->
        <div>
          <label for="capture-year" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">Year</label>
          <input
            id="capture-year"
            type="number"
            placeholder="YYYY"
            bind:value={form.year}
            class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
          />
        </div>
        <!-- Month -->
        <div>
          <label for="capture-month" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">Month</label>
          <input
            id="capture-month"
            type="number"
            placeholder="MM"
            min="1"
            max="12"
            bind:value={form.month}
            class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
          />
        </div>
        <!-- Day -->
        <div>
          <label for="capture-day" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">Day</label>
          <input
            id="capture-day"
            type="number"
            placeholder="DD"
            min="1"
            max="31"
            bind:value={form.day}
            class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
          />
        </div>
      </div>
    </div>
  </form>

  <!-- Save Action Footer -->
  <div class="p-4 border-t border-slate-300 bg-slate-50 flex justify-between gap-2">
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
        <span>Save Specimen</span>
      {/if}
    </button>
  </div>
</div>
