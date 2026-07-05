import { getVersion } from "@tauri-apps/api/app";
import { updaterService } from "../services/updaterService.js";

/**
 * Store managing the application updater state.
 */
export class UpdaterStore {
  /** @type {boolean} */
  isAvailable = $state(false);

  /** @type {string} */
  version = $state("");

  /** @type {string} */
  currentVersion = $state("");

  /** @type {number | null} */
  downloadProgress = $state(null);

  /** @type {'idle' | 'checking' | 'available' | 'downloading' | 'downloaded' | 'installing' | 'error'} */
  status = $state("idle");

  /** @type {string} */
  errorMessage = $state("");

  /** @type {boolean} */
  hasChecked = $state(false);

  constructor() {
    getVersion()
      .then((v) => {
        this.currentVersion = v;
      })
      .catch((err) => {
        console.error("Failed to load app version on start:", err);
      });
  }

  /**
   * Initializes the updater and checks for updates.
   * @returns {Promise<void>}
   */
  async check() {
    this.status = "checking";
    this.errorMessage = "";
    try {
      if (!this.currentVersion) {
        this.currentVersion = await getVersion();
      }
      const res = await updaterService.checkForUpdates();
      if (res && res.available) {
        this.isAvailable = true;
        this.version = res.version;
        
        const downloadedVer = localStorage.getItem("downloadedUpdateVersion");
        if (downloadedVer === this.version) {
          this.status = "downloaded";
        } else {
          this.status = "available";
        }
      } else {
        this.isAvailable = false;
        this.version = "";
        this.status = "idle";
      }
    } catch (err) {
      console.error("Failed to check for updates:", err);
      this.status = "error";
      this.errorMessage = (/** @type {any} */ (err)).toString();
    } finally {
      this.hasChecked = true;
    }
  }

  /**
   * Downloads the update package.
   * @returns {Promise<void>}
   */
  async download() {
    if (this.status !== "available") return;
    this.status = "downloading";
    this.downloadProgress = null;
    this.errorMessage = "";
    
    let totalBytes = 0;
    let downloadedBytes = 0;

    try {
      await updaterService.downloadUpdate((event) => {
        if (event.event === "Started") {
          totalBytes = event.data.contentLength || 0;
          downloadedBytes = 0;
          this.downloadProgress = totalBytes > 0 ? 0 : null;
        } else if (event.event === "Progress") {
          downloadedBytes += event.data.chunkLength;
          if (totalBytes > 0) {
            this.downloadProgress = Math.round((downloadedBytes / totalBytes) * 100);
          }
        } else if (event.event === "Finished") {
          this.downloadProgress = 100;
        }
      });
      
      localStorage.setItem("downloadedUpdateVersion", this.version);
      this.status = "downloaded";
      this.downloadProgress = null;
    } catch (err) {
      console.error("Failed to download update:", err);
      this.status = "error";
      this.errorMessage = (/** @type {any} */ (err)).toString();
    }
  }

  /**
   * Installs the downloaded update.
   * @returns {Promise<void>}
   */
  async install() {
    if (this.status !== "downloaded") return;
    this.status = "installing";
    this.errorMessage = "";
    
    let totalBytes = 0;
    let downloadedBytes = 0;

    try {
      // If we need to download first (e.g. app restarted), the service handles it.
      await updaterService.installUpdate((event) => {
        if (event.event === "Started") {
          totalBytes = event.data.contentLength || 0;
          downloadedBytes = 0;
          this.downloadProgress = totalBytes > 0 ? 0 : null;
        } else if (event.event === "Progress") {
          downloadedBytes += event.data.chunkLength;
          if (totalBytes > 0) {
            this.downloadProgress = Math.round((downloadedBytes / totalBytes) * 100);
          }
        }
      });
    } catch (err) {
      console.error("Failed to install update:", err);
      this.status = "error";
      this.errorMessage = (/** @type {any} */ (err)).toString();
      // If installation failed, reset status back to downloaded (or available if bytes invalid)
      if (updaterService.isDownloaded()) {
        this.status = "downloaded";
      } else {
        this.status = "available";
      }
    }
  }
}

export const updaterStore = new UpdaterStore();
