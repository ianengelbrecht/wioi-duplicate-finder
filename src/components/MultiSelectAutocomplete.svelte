<script>
  import { onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";

  let {
    label = "",
    selectedValues = $bindable([]), // Array of strings representing selected items
    suggestions = [],
    placeholder = "",
    id = "",
    oninput = () => {},
    delay = 0
  } = $props();

  const currentPlaceholder = $derived(selectedValues && selectedValues.length > 0 ? undefined : placeholder);

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
  /** @type {any} */
  let timeoutId = null;

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

  onDestroy(() => {
    if (timeoutId) clearTimeout(timeoutId);
  });

  async function selectSuggestion(/** @type {string} */ sug, /** @type {boolean} */ isCustom = false) {
    if (sug && !selectedValues.includes(sug)) {
      if (isCustom) {
        try {
          const exists = await invoke("check_agent_exists", { name: sug });
          if (!exists) {
            const confirmed = await confirmNewName(sug);
            if (!confirmed) {
              return;
            }
            await invoke("add_agent", { name: sug });
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

  function focusInput() {
    if (inputRef) inputRef.focus();
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
    <label for={id} class="block text-xs font-semibold text-slate-650 uppercase tracking-wider mb-1">
      {label}
    </label>
  {/if}
  
  <div 
    class="flex flex-wrap items-center bg-white border border-slate-300 px-2 py-1 min-h-[38px] focus-within:border-slate-500 focus-within:ring-1 focus-within:ring-slate-500 transition-all cursor-text" 
    onclick={focusInput}
    role="presentation"
  >
    {#each selectedValues as val}
      <span class="bg-slate-100 text-slate-800 text-xs px-2 py-0.5 m-0.5 border border-slate-250 flex items-center gap-1 font-medium select-none">
        {val}
        <button
          type="button"
          onclick={(e) => { e.stopPropagation(); removeValue(val); }}
          class="text-slate-400 hover:text-slate-600 font-bold focus:outline-none text-[10px]"
        >
          &times;
        </button>
      </span>
    {/each}
    
    <input
      bind:this={inputRef}
      {id}
      type="text"
      placeholder={currentPlaceholder}
      value={inputValue}
      oninput={handleInput}
      onkeydown={handleKeyDown}
      onfocus={() => { if (suggestions.length > 0) showDropdown = true; }}
      class="flex-1 bg-transparent text-slate-800 text-sm py-1 outline-none min-w-[120px]"
      autocomplete="off"
    />
  </div>

  {#if showDropdown && suggestions.length > 0}
    <ul class="absolute z-50 left-0 right-0 top-full mt-[-1px] bg-white border border-slate-300 shadow-md max-h-60 overflow-y-auto rounded-none divide-y divide-slate-100">
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
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="w-5 h-5">
              <path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126zM12 15.75h.007v.008H12v-.008z" />
            </svg>
          </div>
          <div class="space-y-1">
            <h3 class="font-bold text-slate-800">New Agent Name</h3>
            <p class="text-sm text-slate-500 leading-relaxed">
              Are you sure you want to save new name?
            </p>
            <p class="text-sm font-semibold text-slate-800">{pendingName}</p>
          </div>
        </div>
        
        <div class="flex justify-end gap-2 mt-2">
          <button
            type="button"
            onclick={handleConfirmNo}
            class="px-3 py-1.5 text-xs font-semibold text-slate-500 hover:bg-slate-50 border border-slate-200 transition-colors cursor-pointer rounded-none"
          >
            Cancel
          </button>
          <button
            type="button"
            onclick={handleConfirmYes}
            class="px-3.5 py-1.5 text-xs font-semibold text-white bg-slate-800 hover:bg-slate-900 transition-colors cursor-pointer rounded-none"
          >
            Yes, Save
          </button>
        </div>
      </div>
    </div>
  {/if}
</div>
