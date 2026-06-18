/**
 * Determine if the provided year, month, and day represent a valid date, allowing for partial dates.
 * Reject dates that are in the future.
 * @param {string|number|null} year 
 * @param {string|number|null} month 
 * @param {string|number|null} day 
 * @returns {boolean}
 */
export function isValidPartialDate(year, month, day) {
  // Allow completely empty dates
  if (
    (year == null || year === "") &&
    (month == null || month === "") &&
    (day == null || day === "")
  ) {
    return true;
  }

  // Year is required if month or day is supplied
  if (year == null || year === "") {
    return false;
  }

  // Month is required if day is supplied
  if (day != null && day !== "" && (month == null || month === "")) {
    return false;
  }

  const y = Number(year);
  const m = month == null || month === "" ? 1 : Number(month);
  const d = day == null || day === "" ? 1 : Number(day);

  // Basic validation
  if (!Number.isInteger(y)) return false;
  if (!Number.isInteger(m) || m < 1 || m > 12) return false;
  if (!Number.isInteger(d) || d < 1 || d > 31) return false;

  // Construct date
  const date = new Date(y, m - 1, d);

  // Reject rolled dates (e.g. Feb 30 becomes March 2)
  if (
    date.getFullYear() !== y ||
    date.getMonth() + 1 !== m ||
    date.getDate() !== d
  ) {
    return false;
  }

  // Reject future dates
  const today = new Date();
  const currentYear = today.getFullYear();
  if (y > currentYear) return false;

  if (y === currentYear && month != null && month !== "") {
    const currentMonth = today.getMonth() + 1;
    const mNum = Number(month);
    if (mNum > currentMonth) return false;

    if (mNum === currentMonth && day != null && day !== "") {
      const currentDay = today.getDate();
      const dNum = Number(day);
      if (dNum > currentDay) return false;
    }
  }

  return true;
}

/**
 * Compare two partial dates.
 * Returns:
 *   - positive number if date1 > date2
 *   - negative number if date1 < date2
 *   - 0 if they are equal or cannot be compared definitively (e.g., missing year in either, or missing month when years are same)
 * 
 * @param {string|number|null} y1 Year of date 1
 * @param {string|number|null} m1 Month of date 1
 * @param {string|number|null} d1 Day of date 1
 * @param {string|number|null} y2 Year of date 2
 * @param {string|number|null} m2 Month of date 2
 * @param {string|number|null} d2 Day of date 2
 * @returns {number}
 */
export function comparePartialDates(y1, m1, d1, y2, m2, d2) {
  if (y1 == null || y1 === "" || y2 == null || y2 === "") {
    return 0;
  }

  const year1 = Number(y1);
  const year2 = Number(y2);
  if (year1 !== year2) {
    return year1 - year2;
  }

  // Same year. If either month is missing, we cannot compare further.
  if (m1 == null || m1 === "" || m2 == null || m2 === "") {
    return 0;
  }

  const month1 = Number(m1);
  const month2 = Number(m2);
  if (month1 !== month2) {
    return month1 - month2;
  }

  // Same year and month. If either day is missing, we cannot compare further.
  if (d1 == null || d1 === "" || d2 == null || d2 === "") {
    return 0;
  }

  const day1 = Number(d1);
  const day2 = Number(d2);
  return day1 - day2;
}