<script>
  import Autocomplete from "../Autocomplete.svelte";
  import { taxonomyService } from "../../services/taxonomyService.js";

  /**
   * @typedef {Object} TaxonomySectionProps
   * @property {any} form
   * @property {function} t
   */

  /** @type {TaxonomySectionProps} */
  let {
    form = $bindable(),
    t
  } = $props();

  /** @type {any[]} */
  let taxonSuggestions = $state([]);
  /** @type {string[]} */
  let typeStatusSuggestions = $state([]);

  const typeStatuses = [
    "Holotype", "Isotype", "Syntype", "Lectotype", "Neotype", "Paratype", "Epitype", "Isolectotype", "Isosyntype", "Isoneotype", "Original material"
  ];

  /**
   * @param {string} val
   */
  async function handleTaxonInput(val) {
    if (val.trim().length < 2) {
      taxonSuggestions = [];
      return;
    }
    try {
      taxonSuggestions = await taxonomyService.autocompleteScientificName(val);
    } catch (e) {
      console.error(e);
    }
  }

  /**
   * @param {any} sug
   */
  function handleTaxonSelect(sug) {
    form.scientificName = sug.scientificName || "";
    form.taxonID = sug.taxonID || "";
    form.family = sug.family || ""; // In case we want to capture family directly
  }

  /**
   * @param {string} val
   */
  function handleTypeStatusInput(val) {
    if (!val) {
      typeStatusSuggestions = typeStatuses;
      return;
    }
    const lowerVal = val.toLowerCase();
    typeStatusSuggestions = typeStatuses.filter(t => t.toLowerCase().includes(lowerVal));
  }

  function handleTypeStatusFocus() {
    typeStatusSuggestions = typeStatuses;
  }
</script>

<div class="space-y-3 pt-2">
  <h3 data-i18n-key="identification-heading" class="text-[10px] font-bold text-slate-400 uppercase tracking-wider border-b border-slate-100 pb-1">{t("identification-heading", "Identification")}</h3>
  
  <div class="grid grid-cols-12 gap-3">
    <!-- Qualifier -->
    <div class="col-span-3">
      <label for="capture-identificationQualifier" data-i18n-key="id-qualifier-label" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">{t("id-qualifier-label", "Qualifier")}</label>
      <select
        id="capture-identificationQualifier"
        bind:value={form.identificationQualifier}
        class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-2 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
      >
        <option value="" data-i18n-key="qualifier-none-option">{t("qualifier-none-option", "(None)")}</option>
        <option value="cf.">cf.</option>
        <option value="aff.">aff.</option>
        <option value="nr.">nr.</option>
      </select>
    </div>

    <!-- Scientific Name -->
    <div class="col-span-6">
      <Autocomplete
        id="capture-scientificName"
        label="Scientific Name"
        labelKey="scientific-name-label"
        placeholder="Partial search eg 'ab man'"
        placeholderKey="scientific-name-placeholder"
        bind:value={form.scientificName}
        suggestions={taxonSuggestions}
        oninput={handleTaxonInput}
        onselect={handleTaxonSelect}
        delay={300}
      />
    </div>

    <!-- Type Status -->
    <div class="col-span-3">
      <label for="capture-typeStatus" data-i18n-key="type-status-label" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">{t("type-status-label", "Type Status")}</label>
      <Autocomplete
        id="capture-typeStatus"
        label=""
        placeholder="eg holotype"
        placeholderKey="type-status-placeholder"
        bind:value={form.typeStatus}
        suggestions={typeStatusSuggestions}
        oninput={handleTypeStatusInput}
        onfocus={handleTypeStatusFocus}
        delay={0}
      />
    </div>
  </div>
</div>
