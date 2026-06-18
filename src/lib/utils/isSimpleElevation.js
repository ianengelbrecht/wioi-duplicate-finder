/**
 * Checks if a verbatim elevation is a simple numeric representation.
 *
 * @param {string|null|undefined} verbatim
 * @returns {boolean}
 */
export function isSimpleElevation(verbatim) {
  if (!verbatim) return true;
  return /^\d+(?:\.\d+)?\s*m?$/i.test(verbatim.trim());
}
