<script>
  import { onDestroy, getContext } from "svelte";
  import { agentService } from "../services/agentService.js";

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
    promptNewAgent = false,
    extraInputClass = "",
    inputRef = $bindable(null)
  } = $props();

  const t = getContext("t");

  let showDropdown = $state(false);
  let activeIndex = $state(-1);
  let containerRef = $state(/** @type {any} */ (null));
  /** @type {any} */
  let timeoutId = null;
  /** @type {any} */
  let blurTimeoutId = null;

  // Confirm Modal States
  let showConfirmModal = $state(false);
  let pendingName = $state("");
  /** @type {((value: boolean) => void) | null} */
  let confirmResolver = null;
  let isChecking = $state(false);

  function confirmNewName(/** @type {string} */ name) {
    pendingName = name;
    showConfirmModal = true;
    return new Promise((resolve) => {
      confirmResolver = resolve;
    });
  }

  function handleConfirmYes() {
    showConfirmModal = false;
    if (confirmResolver) {
      confirmResolver(true);
      confirmResolver = null;
    }
  }

  function handleConfirmNo() {
    showConfirmModal = false;
    if (confirmResolver) {
      confirmResolver(false);
      confirmResolver = null;
    }
  }

  function clearBlurTimeout() {
    if (blurTimeoutId) {
      clearTimeout(blurTimeoutId);
      blurTimeoutId = null;
    }
  }

  async function checkAndSaveAgent(/** @type {string} */ name) {
    if (!promptNewAgent || !name) return;
    const trimmed = name.trim();
    
    isChecking = true;
    try {
      const exists = await agentService.checkAgentExists(trimmed);
      if (!exists) {
        clearBlurTimeout();
        const confirmed = await confirmNewName(trimmed);
        if (!confirmed) {
          isChecking = false;
          return;
        }
        await agentService.addAgent(trimmed);
      }
    } catch (err) {
      console.error("Error checking or adding agent:", err);
    }
    isChecking = false;
  }

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

  function handleBlur() {
    clearBlurTimeout();
    blurTimeoutId = setTimeout(async () => {
      if (isChecking) return;
      const val = value.trim();
      if (val) {
        await checkAndSaveAgent(val);
      }
    }, 200);
  }

  onDestroy(() => {
    if (timeoutId) clearTimeout(timeoutId);
    clearBlurTimeout();
  });

  async function handleKeyDown(/** @type {any} */ e) {
    if (showConfirmModal) {
      if (e.key === "Enter") {
        e.preventDefault();
        handleConfirmYes();
      } else if (e.key === "Escape") {
        e.preventDefault();
        handleConfirmNo();
      }
      return;
    }

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
        const val = value.trim();
        if (val) {
          e.preventDefault();
          clearBlurTimeout();
          await checkAndSaveAgent(val);
        }
      }
    } else if (e.key === "Escape") {
      showDropdown = false;
      activeIndex = -1;
    }
  }

  function selectSuggestion(/** @type {any} */ suggestion) {
    clearBlurTimeout();
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

    // Temporarily set isChecking = true to prevent the blur event (triggered by click) from running checkAndSaveAgent
    const prevChecking = isChecking;
    isChecking = true;

    onselect(suggestion);

    setTimeout(() => {
      isChecking = prevChecking;
    }, 250);
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

  $effect(() => {
    if (suggestions.length > 0 && inputRef && document.activeElement === inputRef) {
      showDropdown = true;
    }
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
    onblur={handleBlur}
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

  {#if showConfirmModal}
    <div 
      class="fixed inset-0 z-[100] flex items-center justify-center bg-slate-900/40 backdrop-blur-xs p-4"
      role="dialog"
      aria-modal="true"
      tabindex="-1"
      onclick={(e) => { if (e.target === e.currentTarget) handleConfirmNo(); }}
      onkeydown={(e) => { if (e.key === "Escape") handleConfirmNo(); }}
    >
      <div class="bg-white border border-slate-200 shadow-2xl max-w-sm w-full p-5 flex flex-col gap-4 rounded-none">
        <div class="flex items-start gap-3">
          <div class="p-2 bg-amber-50 text-amber-600 rounded-full shrink-0">
            <svg xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 256 256" class="w-5 h-5">
              <path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm0,192a88,88,0,1,1,88-88A88.09,88.09,0,0,1,128,216Zm-8-80V80a8,8,0,0,1,16,0v56a8,8,0,0,1-16,0Zm20,36a12,12,0,1,1-12-12A12,12,0,0,1,140,172Z"></path>
            </svg>
          </div>
          <div class="space-y-1">
            <h3 data-i18n-key="new-agent-name-dialog-heading" class="font-bold text-slate-800">{t("new-agent-name-dialog-heading", "New Agent Name")}</h3>
            <p data-i18n-key="new-agent-name-dialog-confirm" class="text-sm text-slate-500 leading-relaxed">
              {t("new-agent-name-dialog-confirm", "Are you sure you want to save new name?")}
            </p>
            <p class="text-sm font-semibold text-slate-800">{pendingName}</p>
          </div>
        </div>
        
        <div class="flex justify-end gap-2 mt-2">
          <button
            type="button"
            data-i18n-key="cancel-btn"
            onclick={handleConfirmNo}
            class="px-3 py-1.5 text-xs font-semibold text-slate-500 hover:bg-slate-50 border border-slate-200 transition-colors cursor-pointer rounded-none"
          >
            {t("cancel-btn", "Cancel")}
          </button>
          <button
            type="button"
            data-i18n-key="yes-save-btn"
            onclick={handleConfirmYes}
            class="px-3.5 py-1.5 text-xs font-semibold text-white bg-slate-800 hover:bg-slate-900 transition-colors cursor-pointer rounded-none"
          >
            {t("yes-save-btn", "Yes, Save")}
          </button>
        </div>
      </div>
    </div>
  {/if}
</div>
