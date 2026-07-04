/**
 * Checks if a string consists entirely of initials.
 * Replicates the Rust and Python helper logic.
 * 
 * @param {string} s 
 * @returns {boolean}
 */
export function isInitials(s) {
  const trimmed = s.trim();
  if (!trimmed) return false;
  const tokens = trimmed.split(/[ .]+/).filter(Boolean);
  if (tokens.length === 0) return false;
  return tokens.every(t => {
    if (t.length === 1) {
      return true;
    } else if (t.length <= 3) {
      return /^[A-Z]+$/.test(t);
    } else {
      return false;
    }
  });
}

/**
 * Splits a string of names separated by semicolons, vertical bars, or commas.
 *
 * @param {string|null|undefined} rawStr - The raw input string containing names.
 * @returns {string[]} An array of trimmed name strings.
 */
export function splitNames(rawStr) {
  if (!rawStr) return [];
  const trimmed = rawStr.trim();
  if (!trimmed) return [];
  
  // First split by & and case-insensitive word and
  const parts = trimmed.split(/&|\band\b/i);

  const finalCollectors = [];
  for (const part of parts) {
    const trimmedPart = part.trim();
    if (!trimmedPart) continue;
    
    const subParts = splitByOtherDelimiters(trimmedPart);
    finalCollectors.push(...subParts);
  }
  
  return finalCollectors.map(s => s.trim()).filter(Boolean);
}

/**
 * Splits a string by other delimiters recursively.
 * 
 * @param {string} s 
 * @returns {string[]}
 */
function splitByOtherDelimiters(s) {
  const trimmed = s.trim();
  if (!trimmed) return [];

  if (trimmed.includes("|")) {
    return trimmed.split("|").flatMap(splitByOtherDelimiters);
  } else if (trimmed.includes(";")) {
    return trimmed.split(";").flatMap(splitByOtherDelimiters);
  } else if (trimmed.includes(",")) {
    const commaCount = (trimmed.match(/,/g) || []).length;
    if (commaCount === 1) {
      const tempParts = trimmed.split(",");
      const partAfter = tempParts[1].trim();
      if (isInitials(partAfter)) {
        return [trimmed];
      } else {
        return tempParts.flatMap(splitByOtherDelimiters);
      }
    } else {
      return trimmed.split(",").flatMap(splitByOtherDelimiters);
    }
  } else {
    return [trimmed];
  }
}

