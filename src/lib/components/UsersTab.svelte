<script>
  import { onMount, getContext } from "svelte";
  import { authStore } from "$lib/stores/authStore.svelte.js";
  import { userService } from "$lib/services/userService.js";

  const t = getContext("t");

  /** @type {import('$lib/services/userService.js').UserDto[]} */
  let users = $state([]);
  let isLoading = $state(true);
  let errorMessage = $state("");
  let successMessage = $state("");

  // Editing state
  let editingUser = $state(/** @type {import('$lib/services/userService.js').UserDto|null} */ (null));
  let editGivenName = $state("");
  let editFamilyName = $state("");
  let editInitials = $state("");
  let editIsAdmin = $state(false);
  let isInitialsManuallyEdited = $state(false);
  let isSaving = $state(false);

  // Auto-calculate initials if not manually overridden
  $effect(() => {
    if (editingUser && !isInitialsManuallyEdited) {
      const parts = `${editGivenName} ${editFamilyName}`.trim().split(/\s+/).filter(Boolean);
      editInitials = parts
        .map(part => part.charAt(0).toUpperCase() + ".")
        .join(" ");
    }
  });

  async function loadUsers() {
    isLoading = true;
    errorMessage = "";
    try {
      users = await userService.getAllUsers(authStore.currentUser.id);
    } catch (err) {
      errorMessage = (/** @type {any} */ (err)).toString();
    } finally {
      isLoading = false;
    }
  }

  /**
   * Starts editing a user.
   * @param {import('$lib/services/userService.js').UserDto} user
   */
  function startEdit(user) {
    editingUser = user;
    editGivenName = user.givenName;
    editFamilyName = user.familyName;
    editInitials = user.initials;
    editIsAdmin = user.isAdmin;
    isInitialsManuallyEdited = false;
    errorMessage = "";
    successMessage = "";
  }

  function cancelEdit() {
    editingUser = null;
  }

  async function saveUser() {
    if (!editingUser) return;
    errorMessage = "";
    successMessage = "";

    if (editGivenName.trim().length === 0 || editFamilyName.trim().length === 0 || editInitials.trim().length === 0) {
      errorMessage = "Given name, family name, and initials cannot be empty.";
      return;
    }

    isSaving = true;
    try {
      await userService.updateUserByAdmin(
        authStore.currentUser.id,
        editingUser.id,
        editGivenName,
        editFamilyName,
        editInitials,
        editIsAdmin
      );

      // If the admin edited themselves, update the current user in store
      if (editingUser.id === authStore.currentUser.id) {
        authStore.updateCurrentUserDetails(editGivenName, editFamilyName, editInitials);
      }

      successMessage = `User "${editingUser.username}" updated successfully!`;
      editingUser = null;
      await loadUsers();
    } catch (err) {
      errorMessage = (/** @type {any} */ (err)).toString();
    } finally {
      isSaving = false;
    }
  }

  onMount(() => {
    loadUsers();
  });
</script>

<div class="space-y-6 flex-1 flex flex-col min-h-0">
  <div>
    <h2 class="text-md font-bold text-slate-900 uppercase tracking-wide">{t("manage-users-title", "Manage Users")}</h2>
    <p class="text-xs text-slate-500 mt-1">{t("manage-users-desc", "View registered users, modify profile details, and manage administrator access settings.")}</p>
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
    <div class="border border-slate-200 overflow-x-auto">
      <table class="w-full text-left border-collapse text-xs">
        <thead>
          <tr class="bg-slate-100 border-b border-slate-200 text-[10px] uppercase font-bold text-slate-500 tracking-wider">
            <th class="p-3">{t("user-th-username", "Username")}</th>
            <th class="p-3">{t("user-th-name", "Full Name")}</th>
            <th class="p-3">{t("user-th-initials", "Initials")}</th>
            <th class="p-3">{t("user-th-role", "Role")}</th>
            <th class="p-3 text-right">{t("user-th-actions", "Actions")}</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-slate-200">
          {#each users as user}
            <tr class="hover:bg-slate-50 transition-colors">
              <td class="p-3 font-semibold text-slate-900 flex items-center gap-1.5">
                {user.username}
                {#if user.isAdmin}
                  <span class="bg-slate-800 text-white text-[9px] font-bold px-1.5 py-0.5 uppercase tracking-wide" title="Administrator">Admin</span>
                {/if}
              </td>
              <td class="p-3 text-slate-700">
                {#if user.givenName || user.familyName}
                  {user.givenName} {user.familyName}
                {:else}
                  <span class="text-slate-400 italic">Not set</span>
                {/if}
              </td>
              <td class="p-3 text-slate-700 font-mono font-bold">{user.initials || '-'}</td>
              <td class="p-3 text-slate-600">
                {user.isAdmin ? t("user-role-admin", "Administrator") : t("user-role-standard", "Standard User")}
              </td>
              <td class="p-3 text-right">
                <button
                  type="button"
                  onclick={() => startEdit(user)}
                  class="bg-white border border-slate-300 hover:bg-slate-50 text-slate-700 px-2.5 py-1 font-bold uppercase tracking-wide transition-colors"
                >
                  {t("edit-btn", "Edit")}
                </button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>

<!-- Edit User Modal Dialog -->
{#if editingUser}
  <div
    class="fixed inset-0 z-[120] flex items-center justify-center bg-slate-900/40 backdrop-blur-xs p-4"
    role="dialog"
    aria-modal="true"
    tabindex="-1"
    onclick={(e) => { if (e.target === e.currentTarget) cancelEdit(); }}
    onkeydown={(e) => { if (e.key === "Escape") cancelEdit(); }}
  >
    <div class="bg-white border border-slate-200 shadow-2xl max-w-sm w-full p-6 flex flex-col gap-4 rounded-none">
      <div>
        <h3 class="font-bold text-slate-900 text-sm uppercase tracking-wide">{t("edit-user-title", "Edit User Details")}</h3>
        <p class="text-[11px] text-slate-550 mt-1">{t("edit-user-desc", "Modify profile information and access privileges for this account.")}</p>
      </div>

      <div class="space-y-3 my-2">
        <div>
          <label for="editUsername" class="block text-[10px] font-semibold text-slate-500 uppercase tracking-wider mb-1">Username</label>
          <input
            id="editUsername"
            type="text"
            value={editingUser.username}
            disabled
            class="w-full bg-slate-50 border border-slate-300 text-slate-550 text-xs px-2.5 py-1.5 outline-none rounded-none cursor-not-allowed"
          />
        </div>

        <div>
          <label for="editGivenName" class="block text-[10px] font-semibold text-slate-600 uppercase tracking-wider mb-1">Given Name</label>
          <input
            id="editGivenName"
            type="text"
            bind:value={editGivenName}
            class="w-full bg-white border border-slate-300 text-slate-800 text-xs px-2.5 py-1.5 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
          />
        </div>

        <div>
          <label for="editFamilyName" class="block text-[10px] font-semibold text-slate-600 uppercase tracking-wider mb-1">Family Name</label>
          <input
            id="editFamilyName"
            type="text"
            bind:value={editFamilyName}
            class="w-full bg-white border border-slate-300 text-slate-800 text-xs px-2.5 py-1.5 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
          />
        </div>

        <div>
          <label for="editInitials" class="block text-[10px] font-semibold text-slate-600 uppercase tracking-wider mb-1">Initials</label>
          <input
            id="editInitials"
            type="text"
            bind:value={editInitials}
            oninput={() => isInitialsManuallyEdited = true}
            class="w-full bg-white border border-slate-300 text-slate-800 text-xs px-2.5 py-1.5 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
          />
        </div>

        <div class="flex items-center gap-2 pt-1.5">
          <input
            id="editIsAdmin"
            type="checkbox"
            bind:checked={editIsAdmin}
            disabled={editingUser.id === authStore.currentUser?.id}
            class="w-4 h-4 accent-slate-800 border-slate-300 focus:ring-slate-500 disabled:opacity-50 disabled:cursor-not-allowed"
          />
          <label for="editIsAdmin" class="text-xs text-slate-700 font-medium select-none cursor-pointer disabled:text-slate-400">
            {t("edit-user-is-admin", "Make Administrator")}
            {#if editingUser.id === authStore.currentUser?.id}
              <span class="text-[9px] text-slate-400 block font-normal">(Cannot revoke your own admin rights)</span>
            {/if}
          </label>
        </div>
      </div>

      <div class="flex justify-end gap-2 mt-2">
        <button
          type="button"
          onclick={cancelEdit}
          class="px-3.5 py-1.5 text-xs font-semibold text-slate-500 hover:bg-slate-50 border border-slate-200 transition-colors cursor-pointer rounded-none"
        >
          {t("cancel-btn", "Cancel")}
        </button>
        <button
          type="button"
          onclick={saveUser}
          disabled={isSaving}
          class="px-4 py-1.5 text-xs font-semibold text-white bg-slate-900 hover:bg-slate-850 transition-colors cursor-pointer rounded-none disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {isSaving ? t("saving-btn", "Saving...") : t("save-changes-btn", "Save Changes")}
        </button>
      </div>
    </div>
  </div>
{/if}
