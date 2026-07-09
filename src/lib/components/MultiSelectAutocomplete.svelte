<script>
  import { onDestroy, getContext } from "svelte";
  import { agentService } from "../services/agentService.js";

  let {
    label = "",
    labelKey = "",
    selectedValues = $bindable([]), // Array of strings representing selected items
    suggestions = [],
    placeholder = "",
    placeholderKey = "",
    id = "",
    oninput = () => {},
    delay = 0
  } = $props();

  const t = getContext("t");

  const currentPlaceholder = $derived(selectedValues && selectedValues.length > 0 ? undefined : (placeholderKey && t ? t(placeholderKey, placeholder) : placeholder));

  let showConfirmModal = $state(false);
  let pendingName = $state("");
  /** @type {((value: boolean) => void) | null} */
  let confirmResolver = null;

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

  let inputValue = $state("");
  let showDropdown = $state(false);
  let activeIndex = $state(-1);
  let containerRef = $state(/** @type {any} */ (null));
  let inputRef = $state(/** @type {any} */ (null));
  let dropdownListRef = $state(/** @type {HTMLUListElement | null} */ (null));
  let editDropdownListRef = $state(/** @type {HTMLUListElement | null} */ (null));
  /** @type {any} */
  let timeoutId = null;
  /** @type {any} */
  let blurTimeoutId = null;
  let isChecking = $state(false);

  function handleInput(/** @type {any} */ e) {
    inputValue = e.target.value;
    showDropdown = true;
    activeIndex = -1;

    if (delay > 0) {
      if (timeoutId) clearTimeout(timeoutId);
      timeoutId = setTimeout(() => {
        oninput(inputValue);
      }, delay);
    } else {
      oninput(inputValue);
    }
  }

  function clearBlurTimeout() {
    if (blurTimeoutId) {
      clearTimeout(blurTimeoutId);
      blurTimeoutId = null;
    }
  }

  function handleBlur() {
    clearBlurTimeout();
    blurTimeoutId = setTimeout(async () => {
      showDropdown = false;
      if (isChecking) return;
      const val = inputValue.trim();
      if (val) {
        await selectSuggestion(val, true);
      }
    }, 200);
  }

  /** @type {any} */
  let editBlurTimeoutId = null;

  function clearEditBlurTimeout() {
    if (editBlurTimeoutId) {
      clearTimeout(editBlurTimeoutId);
      editBlurTimeoutId = null;
    }
  }

  function handleEditBlur() {
    clearEditBlurTimeout();
    editBlurTimeoutId = setTimeout(() => {
      showEditDropdown = false;
    }, 200);
  }

  onDestroy(() => {
    if (timeoutId) clearTimeout(timeoutId);
    clearBlurTimeout();
    clearEditBlurTimeout();
  });

  async function selectSuggestion(/** @type {string} */ sug, /** @type {boolean} */ isCustom = false) {
    clearBlurTimeout();
    const prevChecking = isChecking;
    isChecking = true;

    if (sug && !selectedValues.includes(sug)) {
      if (isCustom) {
        try {
          const exists = await agentService.checkAgentExists(sug);
          if (!exists) {
            const confirmed = await confirmNewName(sug);
            if (!confirmed) {
              // Do not return; still add it to selectedValues.
            } else {
              await agentService.addAgent(sug);
            }
          }
        } catch (err) {
          console.error("Error checking or adding agent:", err);
        }
      }
      selectedValues = [...selectedValues, sug];
    }
    inputValue = "";
    showDropdown = false;
    activeIndex = -1;
    oninput(""); // Clear suggestions in parent!

    setTimeout(() => {
      isChecking = prevChecking;
    }, 250);

    if (inputRef) inputRef.focus();
  }

  function removeValue(/** @type {string} */ val) {
    selectedValues = selectedValues.filter(v => v !== val);
    oninput(""); // Clear suggestions in parent!
    if (inputRef) inputRef.focus();
  }

  function handleKeyDown(/** @type {any} */ e) {
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
      e.preventDefault();
      if (showDropdown && activeIndex >= 0 && activeIndex < suggestions.length) {
        selectSuggestion(suggestions[activeIndex], false);
      } else {
        const val = inputValue.trim();
        if (val) {
          selectSuggestion(val, true);
        }
      }
    } else if (e.key === "Escape") {
      showDropdown = false;
      activeIndex = -1;
    } else if (e.key === "Backspace" && inputValue === "") {
      if (selectedValues.length > 0) {
        removeValue(selectedValues[selectedValues.length - 1]);
      }
    }
  }

  // Edit chip in-place state
  let editingIndex = $state(-1);
  let editInputValue = $state("");
  let editSuggestions = $state(/** @type {string[]} */ ([]));
  let showEditDropdown = $state(false);
  let editActiveIndex = $state(-1);
  let editInputRef = $state(/** @type {HTMLInputElement|null} */ (null));

  function startEdit(/** @type {number} */ index) {
    editingIndex = index;
    editInputValue = selectedValues[index];
    editSuggestions = [];
    showEditDropdown = false;
    editActiveIndex = -1;
    setTimeout(() => {
      if (editInputRef) {
        editInputRef.focus();
        editInputRef.select();
      }
    }, 50);
  }

  async function handleEditInput(/** @type {any} */ e) {
    editInputValue = e.target.value;
    showEditDropdown = true;
    editActiveIndex = -1;

    if (editInputValue.trim().length < 2) {
      editSuggestions = [];
      return;
    }

    try {
      const res = /** @type {string[]} */ (await agentService.autocompleteAgent(editInputValue));
      // Exclude already selected values unless it is the one currently being edited
      editSuggestions = res.filter(name => 
        name === selectedValues[editingIndex] || !selectedValues.includes(name)
      );
    } catch (err) {
      console.error(err);
    }
  }

  async function handleEditFocus() {
    showEditDropdown = true;
    if (editInputValue.trim().length >= 2) {
      try {
        const res = /** @type {string[]} */ (await agentService.autocompleteAgent(editInputValue));
        editSuggestions = res.filter(name => 
          name === selectedValues[editingIndex] || !selectedValues.includes(name)
        );
      } catch (err) {
        console.error(err);
      }
    }
  }

  function selectEditSuggestion(/** @type {string} */ sug) {
    clearEditBlurTimeout();
    editInputValue = sug;
    showEditDropdown = false;
    editActiveIndex = -1;
    saveEdit(editingIndex, sug);
  }

  async function saveEdit(/** @type {number} */ index, /** @type {string} */ newValue) {
    const trimmed = newValue.trim();
    if (!trimmed) {
      // If empty, remove the item
      selectedValues = selectedValues.filter((_, i) => i !== index);
      editingIndex = -1;
      return;
    }

    if (trimmed === selectedValues[index]) {
      editingIndex = -1;
      return;
    }

    if (selectedValues.includes(trimmed)) {
      alert(t("name-already-in-list-error", "This name is already in the list."));
      return;
    }

    // Check if new agent to database
    try {
      const exists = await agentService.checkAgentExists(trimmed);
      if (!exists) {
        const confirmed = await confirmNewName(trimmed);
        if (confirmed) {
          await agentService.addAgent(trimmed);
        }
      }
    } catch (err) {
      console.error("Error checking or adding agent:", err);
    }

    selectedValues[index] = trimmed;
    selectedValues = [...selectedValues];
    editingIndex = -1;
  }

  function handleEditKeyDown(/** @type {any} */ e) {
    if (showConfirmModal) return;

    if (e.key === "ArrowDown") {
      e.preventDefault();
      if (editSuggestions.length > 0) {
        editActiveIndex = (editActiveIndex + 1) % editSuggestions.length;
      }
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      if (editSuggestions.length > 0) {
        editActiveIndex = (editActiveIndex - 1 + editSuggestions.length) % editSuggestions.length;
      }
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (showEditDropdown && editActiveIndex >= 0 && editActiveIndex < editSuggestions.length) {
        selectEditSuggestion(editSuggestions[editActiveIndex]);
      } else {
        saveEdit(editingIndex, editInputValue);
      }
    } else if (e.key === "Escape") {
      e.preventDefault();
      if (showEditDropdown) {
        showEditDropdown = false;
      } else {
        editingIndex = -1;
      }
    }
  }

  function focusInput() {
    if (inputRef) inputRef.focus();
  }

  function handleDocumentClick(/** @type {any} */ e) {
    if (containerRef && !containerRef.contains(e.target)) {
      showDropdown = false;
      showEditDropdown = false;
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
    if (showDropdown && activeIndex >= 0 && dropdownListRef) {
      const activeEl = dropdownListRef.children[activeIndex];
      if (activeEl) {
        const containerRect = dropdownListRef.getBoundingClientRect();
        const elemRect = activeEl.getBoundingClientRect();
        
        if (elemRect.top < containerRect.top) {
          dropdownListRef.scrollTop -= (containerRect.top - elemRect.top);
        } else if (elemRect.bottom > containerRect.bottom) {
          dropdownListRef.scrollTop += (elemRect.bottom - containerRect.bottom);
        }
      }
    }
  });

  $effect(() => {
    if (showEditDropdown && editActiveIndex >= 0 && editDropdownListRef) {
      const activeEl = editDropdownListRef.children[editActiveIndex];
      if (activeEl) {
        const containerRect = editDropdownListRef.getBoundingClientRect();
        const elemRect = activeEl.getBoundingClientRect();
        
        if (elemRect.top < containerRect.top) {
          editDropdownListRef.scrollTop -= (containerRect.top - elemRect.top);
        } else if (elemRect.bottom > containerRect.bottom) {
          editDropdownListRef.scrollTop += (elemRect.bottom - containerRect.bottom);
        }
      }
    }
  });
</script>

<div class="relative w-full" bind:this={containerRef}>
  {#if label}
    <label for={id} data-i18n-key={labelKey || null} class="block text-xs font-semibold text-slate-650 uppercase tracking-wider mb-1">
      {labelKey && t ? t(labelKey, label) : label}
    </label>
  {/if}
  
  <div 
    class="flex flex-wrap items-center bg-white border border-slate-300 px-2 py-1 min-h-[38px] focus-within:border-slate-500 focus-within:ring-1 focus-within:ring-slate-500 transition-all cursor-text" 
    onclick={focusInput}
    role="presentation"
  >
    {#each selectedValues as val, idx}
      <span 
        onclick={(e) => { e.stopPropagation(); startEdit(idx); }}
        class="bg-slate-100 text-slate-800 text-xs px-2 py-0.5 m-0.5 border border-slate-250 flex items-center gap-1 font-medium select-none cursor-pointer hover:bg-slate-200 transition-all animate-fade-in"
        role="button"
        tabindex="0"
        onkeydown={(e) => { if (e.key === "Enter" || e.key === " ") { e.stopPropagation(); startEdit(idx); } }}
      >
        {val}
        <button
          type="button"
          onclick={(e) => { e.stopPropagation(); removeValue(val); }}
          class="text-slate-400 hover:text-slate-600 font-bold focus:outline-none text-[10px] cursor-pointer"
        >
          &times;
        </button>
      </span>
    {/each}
    
    <input
      bind:this={inputRef}
      {id}
      data-i18n-key={placeholderKey || null}
      type="text"
      placeholder={currentPlaceholder}
      value={inputValue}
      oninput={handleInput}
      onkeydown={handleKeyDown}
      onblur={handleBlur}
      onfocus={() => { if (suggestions.length > 0) showDropdown = true; }}
      class="flex-1 bg-transparent text-slate-800 text-sm py-1 outline-none min-w-[120px]"
      autocomplete="off"
    />
  </div>

  {#if showDropdown && suggestions.length > 0}
    <ul bind:this={dropdownListRef} class="absolute z-50 left-0 right-0 top-full mt-[-1px] bg-white border border-slate-300 shadow-md max-h-60 overflow-y-auto rounded-none divide-y divide-slate-100">
      {#each suggestions as sug, i}
        <li>
          <button
            type="button"
            onclick={() => selectSuggestion(sug, false)}
            class="w-full text-left px-3 py-2 text-xs transition-colors rounded-none outline-none {i === activeIndex ? 'bg-slate-100 text-slate-900 font-medium' : 'text-slate-700 hover:bg-slate-50'}"
          >
            {sug}
          </button>
        </li>
      {/each}
    </ul>
  {/if}
  {#if editingIndex >= 0}
    <div 
      class="fixed inset-0 z-[80] flex items-center justify-center bg-slate-900/40 backdrop-blur-xs p-4"
      role="dialog"
      aria-modal="true"
      tabindex="-1"
      onclick={(e) => { if (e.target === e.currentTarget) editingIndex = -1; }}
      onkeydown={(e) => { if (e.key === "Escape") editingIndex = -1; }}
    >
      <div class="bg-white border border-slate-200 shadow-2xl max-w-md w-full p-5 flex flex-col gap-4 rounded-none z-[80] relative">
        <div class="space-y-1">
          <h3 data-i18n-key="edit-name-dialog-heading" class="font-bold text-slate-800">{t("edit-name-dialog-heading", "Edit Name")}</h3>
          <p data-i18n-key="edit-name-dialog-desc" class="text-xs text-slate-500">
            {t("edit-name-dialog-desc", "Modify the selected name. Emptying the field removes it from the list.")}
          </p>
        </div>

        <div class="relative w-full">
          <input
            bind:this={editInputRef}
            data-i18n-key="edit-name-dialog-placeholder"
            type="text"
            value={editInputValue}
            oninput={handleEditInput}
            onkeydown={handleEditKeyDown}
            onfocus={handleEditFocus}
            onblur={handleEditBlur}
            class="w-full bg-white border border-slate-300 px-3 py-2 text-sm focus:border-slate-500 focus:ring-1 focus:ring-slate-500 outline-none transition-all rounded-none text-slate-850"
            autocomplete="off"
            placeholder={t("edit-name-dialog-placeholder", "Enter name...")}
          />

          {#if showEditDropdown && editSuggestions.length > 0}
            <ul bind:this={editDropdownListRef} class="absolute z-[90] left-0 right-0 top-full mt-[-1px] bg-white border border-slate-300 shadow-md max-h-60 overflow-y-auto rounded-none divide-y divide-slate-100">
              {#each editSuggestions as sug, i}
                <li>
                  <button
                    type="button"
                    onclick={() => selectEditSuggestion(sug)}
                    class="w-full text-left px-3 py-2 text-xs transition-colors rounded-none outline-none {i === editActiveIndex ? 'bg-slate-100 text-slate-900 font-medium' : 'text-slate-700 hover:bg-slate-50'}"
                  >
                    {sug}
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
        </div>

        <div class="flex justify-end gap-2 mt-2">
          <button
            type="button"
            data-i18n-key="cancel-btn"
            onclick={() => editingIndex = -1}
            class="px-3 py-1.5 text-xs font-semibold text-slate-500 hover:bg-slate-50 border border-slate-200 transition-colors cursor-pointer rounded-none"
          >
            {t("cancel-btn", "Cancel")}
          </button>
          <button
            type="button"
            data-i18n-key="save-btn"
            onclick={() => saveEdit(editingIndex, editInputValue)}
            class="px-3.5 py-1.5 text-xs font-semibold text-white bg-slate-800 hover:bg-slate-900 transition-colors cursor-pointer rounded-none"
          >
            {t("save-btn", "Save")}
          </button>
        </div>
      </div>
    </div>
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
