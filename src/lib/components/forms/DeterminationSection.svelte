<script>
  import MultiSelectAutocomplete from "../MultiSelectAutocomplete.svelte";
  import { agentService } from "../../services/agentService.js";
  import { isValidPartialDate, comparePartialDates } from "../../utils/isValidPartialDate.js";

  /**
   * @typedef {Object} DeterminationSectionProps
   * @property {any} form
   * @property {string} [statusMessageKey=""]
   * @property {string} [statusMessageDefault=""]
   * @property {string} [statusType=""]
   * @property {function} t
   */

  /** @type {DeterminationSectionProps} */
  let {
    form = $bindable(),
    statusMessageKey = $bindable(""),
    statusMessageDefault = $bindable(""),
    statusType = $bindable(""),
    t
  } = $props();

  /** @type {string[]} */
  let identifiedBySuggestions = $state([]);

  /**
   * @param {string} val
   */
  async function handleIdentifiedByInput(val) {
    if (val.trim().length < 2) {
      identifiedBySuggestions = [];
      return;
    }
    try {
      const res = await agentService.autocompleteAgent(val);
      identifiedBySuggestions = res.filter(name => !form.identifiedBy.includes(name));
    } catch (e) {
      console.error(e);
    }
  }

  function handleIdentificationDateBlur() {
    const { dayIdentified, monthIdentified, yearIdentified } = form;
    if (!isValidPartialDate(yearIdentified, monthIdentified, dayIdentified)) {
      statusMessageKey = "invalid-id-date-error";
      statusMessageDefault = "Error: Invalid identification date.";
      statusType = "error";
    } else if (
      yearIdentified && form.year &&
      comparePartialDates(yearIdentified, monthIdentified, dayIdentified, form.year, form.month, form.day) < 0
    ) {
      statusMessageKey = "id-date-before-collection-error";
      statusMessageDefault = "Error: Identification date cannot be before collection date.";
      statusType = "error";
    } else {
      if (
        statusMessageKey === "invalid-id-date-error" ||
        statusMessageKey === "id-date-before-collection-error"
      ) {
        statusMessageKey = "";
        statusMessageDefault = "";
        statusType = "";
      }
    }
  }
</script>

<div class="grid grid-cols-12 gap-3 pt-2">
  <!-- Det By -->
  <div class="col-span-6">
    <MultiSelectAutocomplete
      id="capture-identifiedBy"
      label="Det By"
      labelKey="det-by-label"
      placeholder="Type name and press Enter..."
      placeholderKey="det-by-placeholder"
      bind:selectedValues={form.identifiedBy}
      suggestions={identifiedBySuggestions}
      oninput={handleIdentifiedByInput}
      delay={300}
    />
  </div>

  <!-- Identification Date Fields -->
  <div class="col-span-2">
    <label for="capture-yearIdentified" data-i18n-key="det-year-label" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">{t("det-year-label", "Year Ident.")}</label>
    <input
      id="capture-yearIdentified"
      type="number"
      bind:value={form.yearIdentified}
      onblur={handleIdentificationDateBlur}
      class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-2 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
    />
  </div>
  <div class="col-span-2">
    <label for="capture-monthIdentified" data-i18n-key="det-month-label" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">{t("det-month-label", "Month Ident.")}</label>
    <input
      id="capture-monthIdentified"
      type="number"
      min="1"
      max="12"
      bind:value={form.monthIdentified}
      onblur={handleIdentificationDateBlur}
      class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-2 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
    />
  </div>
  <div class="col-span-2">
    <label for="capture-dayIdentified" data-i18n-key="det-day-label" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">{t("det-day-label", "Day Ident.")}</label>
    <input
      id="capture-dayIdentified"
      type="number"
      min="1"
      max="31"
      bind:value={form.dayIdentified}
      onblur={handleIdentificationDateBlur}
      class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-2 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
    />
  </div>
</div>

<div class="grid grid-cols-12 gap-3 pt-1">
  <!-- Identification Notes -->
  <div class="col-span-8">
    <label for="capture-identificationRemarks" data-i18n-key="det-remarks-label" class="block text-xs font-semibold text-slate-655 uppercase tracking-wider mb-1">{t("det-remarks-label", "Identification Notes")}</label>
    <input
      id="capture-identificationRemarks"
      data-i18n-key="det-remarks-placeholder"
      type="text"
      placeholder={t("det-remarks-placeholder", "eg 'cf. species A'")}
      bind:value={form.identificationRemarks}
      class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
    />
  </div>

  <!-- Cultivated Specimen Flag -->
  <div class="col-span-4 flex items-center pt-5">
    <label class="flex items-center gap-2 cursor-pointer select-none">
      <input
        type="checkbox"
        bind:checked={form.cultivated}
        class="w-4 h-4 border border-slate-300 rounded-none checked:bg-slate-800 outline-none cursor-pointer"
      />
      <span data-i18n-key="cultivated-label" class="text-xs font-semibold text-slate-600 uppercase tracking-wider">{t("cultivated-label", "Cultivated Specimen")}</span>
    </label>
  </div>
</div>
