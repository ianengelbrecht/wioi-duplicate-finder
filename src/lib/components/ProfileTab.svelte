<script>
  import { onMount, getContext } from "svelte";
  import { authStore } from "$lib/stores/authStore.svelte.js";
  import { userService } from "$lib/services/userService.js";

  const t = getContext("t");

  // Local state for the form
  let givenName = $state("");
  let familyName = $state("");
  let initials = $state("");
  let initialsManuallyEdited = $state(false);

  let successMessage = $state("");
  let errorMessage = $state("");
  let isEdited = $state(false);
  let isSaving = $state(false);
  let isLoading = $state(true);

  /**
   * Helper to calculate initials from given and family names.
   * @param {string} gName
   * @param {string} fName
   * @returns {string}
   */
  function calculateInitials(gName, fName) {
    const parts = `${gName} ${fName}`.trim().split(/\s+/).filter(Boolean);
    return parts
      .map(part => part.charAt(0).toUpperCase() + ".")
      .join(" ");
  }

  // Auto-calculate initials if not manually overridden
  $effect(() => {
    if (!initialsManuallyEdited) {
      initials = calculateInitials(givenName, familyName);
    }
  });

  $effect(() => {
    isEdited = (
      givenName !== (authStore.currentUser?.givenName || "") ||
      familyName !== (authStore.currentUser?.familyName || "") ||
      initials !== (authStore.currentUser?.initials || "")
    );
  });

  /**
   * Handles saving the updated profile details.
   * @param {SubmitEvent} e
   */
  async function handleSave(e) {
    if (e) e.preventDefault();
    successMessage = "";
    errorMessage = "";

    if (givenName.trim().length === 0 || familyName.trim().length === 0 || initials.trim().length === 0) {
      errorMessage = "Given name, family name, and initials cannot be empty.";
      return;
    }

    isSaving = true;
    try {
      await userService.updateUserProfile(
        authStore.currentUser.id,
        givenName,
        familyName,
        initials
      );
      
      // Update in-memory auth store
      authStore.updateCurrentUserDetails(givenName, familyName, initials);
      successMessage = "Profile updated successfully!";
      initialsManuallyEdited = false; // Reset to allow subsequent calculations
    } catch (err) {
      errorMessage = (/** @type {any} */ (err)).toString();
    } finally {
      isSaving = false;
    }
  }

  async function loadProfile() {
    if (!authStore.currentUser) return;
    isLoading = true;
    errorMessage = "";
    try {
      const user = await userService.getUserById(authStore.currentUser.id);
      if (user) {
        // Set initialsManuallyEdited first so the $effect doesn't overwrite it
        const expected = calculateInitials(user.givenName || "", user.familyName || "");
        initialsManuallyEdited = !!(user.initials && user.initials !== expected);

        givenName = user.givenName || "";
        familyName = user.familyName || "";
        initials = user.initials || "";

        // Update in-memory auth store so it remains synchronized
        authStore.updateCurrentUserDetails(user.givenName, user.familyName, user.initials);
      }
    } catch (err) {
      errorMessage = (/** @type {any} */ (err)).toString();
    } finally {
      isLoading = false;
    }
  }

  onMount(() => {
    loadProfile();
  });
</script>

<div class="space-y-6 flex-1 flex flex-col min-h-0">
  <div>
    <h2 class="text-md font-bold text-slate-900 uppercase tracking-wide">{t("profile-title", "My Profile Details")}</h2>
    <p class="text-xs text-slate-500 mt-1">{t("profile-desc", "Update your personal details and initials used for identifying captured specimens.")}</p>
  </div>

  {#if successMessage}
    <div class="p-3 text-xs bg-emerald-50 border border-emerald-200 text-emerald-800 font-semibold rounded-none flex items-center gap-2">
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" class="w-4 h-4 text-emerald-600">
        <path fill-rule="evenodd" d="M2.25 12c0-5.385 4.365-9.75 9.75-9.75s9.75 4.365 9.75 9.75-4.365 9.75-9.75 9.75S2.25 17.385 2.25 12zm13.36-1.814a.75.75 0 10-1.22-.872l-3.236 4.53L9.53 12.22a.75.75 0 00-1.06 1.06l2.25 2.25a.75.75 0 001.14-.094l3.75-5.25z" clip-rule="evenodd" />
      </svg>
      {successMessage}
    </div>
  {/if}

  {#if errorMessage}
    <div class="p-3 text-xs bg-red-50 border border-red-200 text-red-800 font-semibold rounded-none flex items-center gap-2">
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" class="w-4 h-4 text-red-600">
        <path fill-rule="evenodd" d="M12 2.25c-5.385 0-9.75 4.365-9.75 9.75s4.365 9.75 9.75 9.75 9.75-4.365 9.75-9.75S17.385 2.25 12 2.25zm-1.72 6.97a.75.75 0 10-1.06 1.06L10.94 12l-1.72 1.72a.75.75 0 101.06 1.06L12 13.06l1.72 1.72a.75.75 0 101.06-1.06L13.06 12l1.72-1.72a.75.75 0 10-1.06-1.06L12 10.94l-1.72-1.72z" clip-rule="evenodd" />
      </svg>
      {errorMessage}
    </div>
  {/if}

  {#if isLoading}
    <div class="py-12 flex justify-center items-center">
      <div class="w-6 h-6 border-2 border-slate-200 border-t-slate-800 rounded-full animate-spin"></div>
    </div>
  {:else}
    <form onsubmit={handleSave} class="space-y-4 max-w-md">
      <div>
        <label for="username" class="block text-xs font-semibold text-slate-500 uppercase tracking-wider mb-1">{t("username-label", "Username")}</label>
        <input
          id="username"
          type="text"
          value={authStore.currentUser?.username}
          disabled
          class="w-full bg-slate-50 border border-slate-300 text-slate-500 text-sm px-3 py-2 outline-none rounded-none cursor-not-allowed"
        />
      </div>

      <div>
        <label for="givenName" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">{t("given-name-label", "Given Name")}</label>
        <input
          id="givenName"
          type="text"
          placeholder={t("given-name-placeholder", "Enter given name")}
          bind:value={givenName}
          class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
        />
      </div>

      <div>
        <label for="familyName" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">{t("family-name-label", "Family Name")}</label>
        <input
          id="familyName"
          type="text"
          placeholder={t("family-name-placeholder", "Enter family name")}
          bind:value={familyName}
          class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
        />
      </div>

      <div>
        <label for="initials" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">{t("initials-label", "Initials")}</label>
        <input
          id="initials"
          type="text"
          placeholder={t("initials-placeholder", "Enter initials")}
          bind:value={initials}
          oninput={() => initialsManuallyEdited = true}
          class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
        />
      </div>

      <div class="flex w-full justify-end">
        <button
          type="submit"
          disabled={isSaving || !isEdited}
          class="{ isEdited? 'bg-slate-900 hover:bg-slate-800 text-white' : 'bg-slate-200 text-slate-500 cursor-default!'} px-6 py-2 text-xs font-bold uppercase tracking-wider rounded-none transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {isSaving ? t("saving-btn", "Saving...") : t("save-changes-btn", "Save Changes")}
        </button>
      </div>
    </form>
  {/if}
</div>
