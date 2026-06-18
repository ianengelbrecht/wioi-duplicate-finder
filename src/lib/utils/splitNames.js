/**
 * Splits a string of names separated by semicolons or vertical bars, and trims each part.
 *
 * @param {string|null|undefined} rawStr - The raw input string containing names.
 * @returns {string[]} An array of trimmed name strings.
 */
export function splitNames(rawStr) {
  if (!rawStr) return [];
  const trimmed = rawStr.trim();
  if (!trimmed) return [];
  
  let parts = [];
  if (trimmed.includes(";")) {
    parts = trimmed.split(";");
  } else if (trimmed.includes("|")) {
    parts = trimmed.split("|");
  } else {
    parts = [trimmed];
  }
  
  return parts.map(s => s.trim()).filter(Boolean);
}
