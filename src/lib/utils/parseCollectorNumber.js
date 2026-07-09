/**
 * Parses a collector number into prefix, number, and suffix parts.
 *
 * @param {string|null|undefined} recordNumber
 * @returns {{prefix: string, number: string, suffix: string}}
 */
export function parseCollectorNumber(recordNumber) {
  const result = {
    prefix: "",
    number: "",
    suffix: ""
  };
  if (!recordNumber) return result;
  const str = recordNumber.trim().replace(/\s+/g, "");

  const yearSlashRegex = /^(.*?\b(?:19|20)?\d{2}\s*\/\s*)(\d+)(.*)$/;
  const yearSlashMatch = str.match(yearSlashRegex);
  if (yearSlashMatch) {
    result.prefix = yearSlashMatch[1];
    result.number = yearSlashMatch[2];
    result.suffix = yearSlashMatch[3];
    return result;
  }

  const digitRegex = /^(.*?)(\d+)(.*)$/;
  const digitMatch = str.match(digitRegex);
  if (digitMatch) {
    result.prefix = digitMatch[1];
    result.number = digitMatch[2];
    result.suffix = digitMatch[3];
    return result;
  }

  if (str.toLowerCase() == "s.n." || str.toLowerCase() == "s.n") {
    result.number = str;
    return result;
  }

  return result; // the value is not a valid collector number, return empty parts

}
