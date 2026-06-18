<script>
  import parser from "any-date-parser";
  import Autocomplete from "../Autocomplete.svelte";
  import MultiSelectAutocomplete from "../MultiSelectAutocomplete.svelte";
  import { agentService } from "../../services/agentService.js";
  import { isValidPartialDate, comparePartialDates } from "../../utils/isValidPartialDate.js";

  let {
    form = $bindable(),
    titleCasedStates = $bindable(),
    statusMessageKey = $bindable(""),
    statusMessageDefault = $bindable(""),
    statusType = $bindable(""),
    undoTitleCaseField,
    titleCaseField,
    t
  } = $props();

  let eventDateLanguage = $state("EN");
  let collectorSuggestions = $state([]);
  let additionalCollectorsSuggestions = $state([]);

  function parseVerbatimDate() {
    let dateStr = form.verbatimEventDate.trim();
    if (!dateStr) return;
    let { day, month, year } = parser.attempt(dateStr, eventDateLanguage === "FR" ? "fr-FR" : "en-US");
    form.day = day ? String(day) : "";
    form.month = month ? String(month) : "";
    form.year = year ? String(year) : "";
  }

  $effect(() => {
    if (eventDateLanguage) {
      parseVerbatimDate();
    }
  });

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

  async function handleCollectorInput(val) {
    if (val.trim().length < 2) {
      collectorSuggestions = [];
      return;
    }
    try {
      const res = await agentService.autocompleteAgent(val);
      collectorSuggestions = res.filter(name => !form.additionalCollectors.includes(name));
    } catch (e) {
      console.error(e);
    }
  }

  async function handleAdditionalCollectorsInput(val) {
    if (val.trim().length < 2) {
      additionalCollectorsSuggestions = [];
      return;
    }
    try {
      const res = await agentService.autocompleteAgent(val);
      additionalCollectorsSuggestions = res.filter(name => 
        name !== form.recordedBy && 
        !form.additionalCollectors.includes(name)
      );
    } catch (e) {
      console.error(e);
    }
  }
</script>

<div class="grid grid-cols-12 gap-3">
  <!-- Primary Collector -->
  <div class="col-span-3">
    <Autocomplete
      id="capture-recordedBy"
      label="Primary Collector"
      labelKey="recorded-by-label"
      placeholder="Partial eg 'Raza'"
      placeholderKey="recorded-by-placeholder"
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
    <label for="capture-verbatimEventDate" data-i18n-key="verbatim-event-date-label" class="block text-xs font-semibold text-slate-650 uppercase tracking-wider mb-1">
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
          <button
            type="button"
            onclick={() => eventDateLanguage = "FR"}
            class="px-1 py-0.5 text-[10px] font-bold tracking-wider hover:bg-slate-50 transition-colors cursor-pointer {eventDateLanguage === 'FR' ? 'bg-slate-800 text-white hover:bg-slate-800' : 'bg-white text-slate-400'}"
          >
            FR
          </button>
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
    <div class="flex-1">
      <label for="capture-year" data-i18n-key="year-label" class="block text-xs font-semibold text-slate-650 uppercase tracking-wider mb-1">{t("year-label", "Year")}</label>
      <input
        id="capture-year"
        type="number"
        bind:value={form.year}
        onblur={handleCollectionDateBlur}
        class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-2 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
      />
    </div>
    <div class="flex-1">
      <label for="capture-month" data-i18n-key="month-label" class="block text-xs font-semibold text-slate-650 uppercase tracking-wider mb-1">{t("month-label", "Month")}</label>
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
    <div class="flex-1">
      <label for="capture-day" data-i18n-key="day-label" class="block text-xs font-semibold text-slate-650 uppercase tracking-wider mb-1">{t("day-label", "Day")}</label>
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
      placeholderKey="add-collectors-placeholder"
      bind:selectedValues={form.additionalCollectors}
      suggestions={additionalCollectorsSuggestions}
      oninput={handleAdditionalCollectorsInput}
      delay={300}
    />
  </div>
</div>

<!-- Field Notes & General Notes (Occurrence Remarks) -->
<div class="grid grid-cols-2 gap-3 pt-1">
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
      {#if form.fieldNotes === titleCasedStates.fieldNotes.titleCased && titleCasedStates.fieldNotes.titleCased !== ""}
        <button
          type="button"
          onclick={() => undoTitleCaseField("fieldNotes")}
          data-i18n-key="undo-title-case"
          title={t("undo-title-case", "Undo Casing")}
          class="absolute right-2 bottom-3.5 text-slate-400 hover:text-slate-600 z-10 cursor-pointer flex items-center justify-center"
          tabindex="-1"
        >
          <svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256" class="w-3.5 h-3.5">
            <path d="M232,144a64.07,64.07,0,0,1-64,64H80a8,8,0,0,1,0-16h88a48,48,0,0,0,0-96H51.31l34.35,34.34a8,8,0,0,1-11.32,11.32l-48-48a8,8,0,0,1,0-11.32l48-48A8,8,0,0,1,85.66,45.66L51.31,80H168A64.07,64.07,0,0,1,232,144Z"></path>
          </svg>
        </button>
      {:else}
        <button
          type="button"
          onclick={() => titleCaseField("fieldNotes")}
          data-i18n-key="title-case-field-notes"
          title={t("title-case-field-notes", "Title case Field Notes")}
          class="absolute right-2 bottom-3.5 text-slate-400 hover:text-slate-600 z-10 cursor-pointer flex items-center justify-center"
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
    <label for="capture-occurrenceRemarks" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">{t("occurrence-remarks-label", "General Notes")}</label>
    <div class="relative flex items-center">
      <textarea
        id="capture-occurrenceRemarks"
        placeholder={t("occurrence-remarks-placeholder", "eg 'Common in tapia forest'")}
        bind:value={form.occurrenceRemarks}
        rows="2"
        class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all pr-12"
      ></textarea>
      {#if form.occurrenceRemarks === titleCasedStates.occurrenceRemarks.titleCased && titleCasedStates.occurrenceRemarks.titleCased !== ""}
        <button
          type="button"
          onclick={() => undoTitleCaseField("occurrenceRemarks")}
          data-i18n-key="undo-title-case"
          title={t("undo-title-case", "Undo Casing")}
          class="absolute right-2 bottom-3.5 text-slate-400 hover:text-slate-600 z-10 cursor-pointer flex items-center justify-center"
          tabindex="-1"
        >
          <svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256" class="w-3.5 h-3.5">
            <path d="M232,144a64.07,64.07,0,0,1-64,64H80a8,8,0,0,1,0-16h88a48,48,0,0,0,0-96H51.31l34.35,34.34a8,8,0,0,1-11.32,11.32l-48-48a8,8,0,0,1,0-11.32l48-48A8,8,0,0,1,85.66,45.66L51.31,80H168A64.07,64.07,0,0,1,232,144Z"></path>
          </svg>
        </button>
      {:else}
        <button
          type="button"
          onclick={() => titleCaseField("occurrenceRemarks")}
          data-i18n-key="title-case-occurrence-remarks"
          title={t("title-case-occurrence-remarks", "Title case General Notes")}
          class="absolute right-2 bottom-3.5 text-slate-400 hover:text-slate-600 z-10 cursor-pointer flex items-center justify-center"
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
