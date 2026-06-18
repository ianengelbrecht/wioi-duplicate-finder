/**
 * Formats a year, month, and day into an ISO 8601 YYYY-MM-DD, YYYY-MM, or YYYY string.
 *
 * @param {number|string|null} year
 * @param {number|string|null} month
 * @param {number|string|null} day
 * @returns {string}
 */
export function formatISO8601Date(year, month, day) {
  if (!year) return "";
  let dateStr = String(year);
  if (month) {
    let m = String(month).padStart(2, "0");
    dateStr += `-${m}`;
    if (day) {
      let d = String(day).padStart(2, "0");
      dateStr += `-${d}`;
    }
  }
  return dateStr;
}
