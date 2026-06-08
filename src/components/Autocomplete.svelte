<script>
  import { onDestroy, getContext } from "svelte";

  let {
    label = "",
    labelKey = "",
    value = $bindable(""),
    suggestions = [], // Can be Array of strings or Array of objects e.g. { scientificName, family, authors }
    placeholder = "",
    placeholderKey = "",
    id = "",
    oninput = () => {},
    onselect = () => {},
    onfocus = () => {},
    delay = 0,
    customSelect = false,
    extraInputClass = "",
    inputRef = $bindable(null)
  } = $props();

  const t = getContext("t");

  let showDropdown = $state(false);
  let activeIndex = $state(-1);
  let containerRef = $state(/** @type {any} */ (null));
  /** @type {any} */
  let timeoutId = null;

  function handleInput(/** @type {any} */ e) {
    value = e.target.value;
    showDropdown = true;
    activeIndex = -1;

    if (delay > 0) {
      if (timeoutId) clearTimeout(timeoutId);
      timeoutId = setTimeout(() => {
        oninput(value);
      }, delay);
    } else {
      oninput(value);
    }
  }

  onDestroy(() => {
    if (timeoutId) clearTimeout(timeoutId);
  });

  function handleKeyDown(/** @type {any} */ e) {
    if (!showDropdown && suggestions.length > 0 && (e.key === "ArrowDown" || e.key === "ArrowUp")) {
      showDropdown = true;
      return;
    }

    if (e.key === "ArrowDown") {
      e.preventDefault();
      if (suggestions.length > 0) {
        activeIndex = (activeIndex + 1) % suggestions.length;
      }
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      if (suggestions.length > 0) {
        activeIndex = (activeIndex - 1 + suggestions.length) % suggestions.length;
      }
    } else if (e.key === "Enter") {
      if (showDropdown && activeIndex >= 0 && activeIndex < suggestions.length) {
        e.preventDefault();
        selectSuggestion(suggestions[activeIndex]);
      } else {
        showDropdown = false;
      }
    } else if (e.key === "Escape") {
      showDropdown = false;
      activeIndex = -1;
    }
  }

  function selectSuggestion(/** @type {any} */ suggestion) {
    let selectedText = "";
    if (typeof suggestion === "string") {
      selectedText = suggestion;
    } else {
      selectedText = suggestion.scientificName || "";
    }
    
    if (!customSelect) {
      value = selectedText;
    }
    showDropdown = false;
    activeIndex = -1;
    onselect(suggestion);
  }

  function handleDocumentClick(/** @type {any} */ e) {
    if (containerRef && !containerRef.contains(e.target)) {
      showDropdown = false;
    }
  }

  // Handle window click monitoring
  $effect(() => {
    document.addEventListener("click", handleDocumentClick);
    return () => {
      document.removeEventListener("click", handleDocumentClick);
    };
  });
</script>

<div class="relative w-full" bind:this={containerRef}>
  {#if label}
    <label for={id} data-i18n-key={labelKey || null} class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">
      {labelKey && t ? t(labelKey, label) : label}
    </label>
  {/if}
  
  <input
    bind:this={inputRef}
    {id}
    data-i18n-key={placeholderKey || null}
    type="text"
    placeholder={placeholderKey && t ? t(placeholderKey, placeholder) : placeholder}
    {value}
    oninput={handleInput}
    onkeydown={handleKeyDown}
    onfocus={() => {
      onfocus();
      if (suggestions.length > 0) showDropdown = true;
    }}
    class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all {extraInputClass}"
    autocomplete="off"
  />

  {#if showDropdown && suggestions.length > 0}
    <ul class="absolute z-50 left-0 right-0 top-full mt-[-1px] bg-white border border-slate-300 shadow-md max-h-60 overflow-y-auto rounded-none divide-y divide-slate-100">
      {#each suggestions as sug, i}
        <li>
          <button
            type="button"
            onclick={() => selectSuggestion(sug)}
            class="w-full text-left px-3 py-2 text-xs transition-colors rounded-none outline-none {i === activeIndex ? 'bg-slate-100 text-slate-900 font-medium' : 'text-slate-700 hover:bg-slate-50'}"
          >
            {#if typeof sug === "string"}
              {sug}
            {:else}
              <div class="flex justify-between items-center">
                <span class="font-medium text-slate-900">{sug.scientificName}</span>
                {#if sug.family}
                  <span class="text-[10px] text-slate-400 uppercase tracking-wider font-semibold">{sug.family}</span>
                {/if}
              </div>
            {/if}
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>
