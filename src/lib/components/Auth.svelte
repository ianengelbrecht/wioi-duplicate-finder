<script>
  import { getContext } from "svelte";
  import { authStore } from "$lib/stores/authStore.svelte.js";
  import { authService } from "$lib/services/authService.js";

  const t = getContext("t");

  let { onLoginSuccess = async () => {} } = $props();

  // Auth form state (UI only)
  let isRegister = $state(false);
  let authUsername = $state("");
  let authPassword = $state("");

  async function handleAuth(/** @type {any} */ e) {
    if (e) e.preventDefault();
    authStore.authError = "";
    authStore.authSuccess = "";

    if (authUsername.trim().length === 0 || authPassword.trim().length === 0) {
      authStore.authError = "Please fill out all fields.";
      return;
    }

    try {
      if (isRegister) {
        let msg = await authService.register(authUsername, authPassword);
        authStore.authSuccess = /** @type {string} */ (msg);
        isRegister = false;
        authPassword = "";
      } else {
        let user = await authService.login(authUsername, authPassword);
        if (user) {
          authStore.setCurrentUser(user);
          authStore.setView("dashboard");
          await onLoginSuccess();
        } else {
          authStore.authError = "Invalid username or password.";
        }
      }
    } catch (err) {
      authStore.authError = (/** @type {any} */ (err)).toString();
    }
  }
</script>

<div class="flex-1 flex justify-center items-center p-6">
  <div class="w-full max-w-sm bg-white border border-slate-300 shadow-sm p-6 space-y-6">
    <div class="text-center">
      <h2 class="text-xl font-bold tracking-tight text-slate-900">
        {isRegister ? t("register-heading", "Register New Account") : t("sign-in-heading", "Sign In")}
      </h2>
      <p class="text-xs text-slate-500 mt-1 leading-relaxed">
        {isRegister 
          ? t("register-desc", "Configure login details to manage captured sessions locally.") 
          : t("sign-in-desc", "Enter credentials to unlock specimen databases.")}
      </p>
    </div>

    {#if authStore.authError}
      <div class="p-3 text-xs bg-red-50 border border-red-200 text-red-700 font-medium">
        {authStore.authError}
      </div>
    {/if}

    {#if authStore.authSuccess}
      <div class="p-3 text-xs bg-emerald-50 border border-emerald-200 text-emerald-700 font-medium">
        {authStore.authSuccess}
      </div>
    {/if}

    <form onsubmit={handleAuth} class="space-y-4">
      <div>
        <label for="username" data-i18n-key="username-label" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">{t("username-label", "Username")}</label>
        <input
          id="username"
          type="text"
          placeholder={t("username-placeholder", "Enter username")}
          bind:value={authUsername}
          class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
        />
      </div>
      <div>
        <label for="password" data-i18n-key="password-label" class="block text-xs font-semibold text-slate-600 uppercase tracking-wider mb-1">{t("password-label", "Password")}</label>
        <input
          id="password"
          type="password"
          placeholder={t("password-placeholder", "Enter password")}
          bind:value={authPassword}
          class="w-full bg-white border border-slate-300 text-slate-800 text-sm px-3 py-2 outline-none focus:border-slate-500 focus:ring-1 focus:ring-slate-500 rounded-none transition-all"
        />
      </div>

      <button
        type="submit"
        class="w-full bg-slate-800 hover:bg-slate-900 text-white py-2 text-xs font-bold uppercase tracking-wider rounded-none transition-colors"
      >
        {isRegister ? t("register-btn", "Create Account") : t("sign-in-btn", "Sign In")}
      </button>
    </form>

    <div class="text-center pt-2">
      <button
        type="button"
        onclick={() => { isRegister = !isRegister; authStore.authError = ""; authStore.authSuccess = ""; }}
        class="text-xs text-slate-500 hover:text-slate-800 underline font-medium"
      >
        {isRegister ? t("already-have-account", "Already have an account? Sign In") : t("need-account", "Need an account? Register")}
      </button>
    </div>
    <div>
      <a href="https://varuna-biodiversite.org/" target="_blank" rel="noopener noreferrer">
        <img src="funders_desat.png" alt="Funders Logos" class="w-full" />
      </a>
    </div>
  </div>
</div>
