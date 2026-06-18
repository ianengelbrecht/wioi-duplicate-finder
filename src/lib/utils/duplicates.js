/**
 * Filters the list of duplicates based on the current input value (comma-separated list).
 * Suggests items that match the last part of the input and are not already in the list.
 *
 * @param {string} val - Current input value (e.g. "P, K, ")
 * @param {string[]} duplicatesList - Predefined list of duplicate herbaria
 * @returns {string[]} Filtered list of suggestions
 */
export function getDuplicateSuggestions(val, duplicatesList) {
  if (!val) {
    return duplicatesList;
  }
  const parts = val.split(",").map(s => s.trim());
  const lastPart = parts[parts.length - 1].toLowerCase();
  
  return duplicatesList.filter(d => 
    d.toLowerCase().includes(lastPart) && 
    !parts.slice(0, -1).map(p => p.toLowerCase()).includes(d.toLowerCase())
  );
}
