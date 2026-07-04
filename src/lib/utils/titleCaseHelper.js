import { totitleCase } from "./totitleCase.js";

/**
 * Proper-cases (Title cases) a field on a form object, and updates the tracking state.
 *
 * @param {any} form - The Svelte form object.
 * @param {any} trackingState - The titleCasedStates tracking object.
 * @param {string} field - The field name to title case.
 * @returns {void}
 */
export function titleCaseField(form, trackingState, field) {
  const val = form[field];
  if (typeof val === "string") {
    trackingState[field] = {
      original: val,
      titleCased: totitleCase(val)
    };
    form[field] = trackingState[field].titleCased;
  }
}

/**
 * Restores a field on a form object to its original pre-title-cased value.
 *
 * @param {any} form - The Svelte form object.
 * @param {any} trackingState - The titleCasedStates tracking object.
 * @param {string} field - The field name to restore.
 * @returns {void}
 */
export function undoTitleCaseField(form, trackingState, field) {
  const stateObj = trackingState[field];
  if (stateObj && stateObj.original !== "") {
    form[field] = stateObj.original;
    trackingState[field] = { original: "", titleCased: "" };
  }
}

/**
 * Returns a fresh, empty tracking state object.
 *
 * @returns {any}
 */
export function getInitialTrackingState() {
  return {
    country: { original: "", titleCased: "" },
    stateProvince: { original: "", titleCased: "" },
    county: { original: "", titleCased: "" },
    municipality: { original: "", titleCased: "" },
    islandGroup: { original: "", titleCased: "" },
    island: { original: "", titleCased: "" },
    locality: { original: "", titleCased: "" },
    locationNotes: { original: "", titleCased: "" },
    habitat: { original: "", titleCased: "" },
    fieldNotes: { original: "", titleCased: "" },
    occurrenceRemarks: { original: "", titleCased: "" }
  };
}
