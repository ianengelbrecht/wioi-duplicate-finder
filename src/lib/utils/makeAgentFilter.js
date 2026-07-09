export function makeAgentFilter({ initialsRequirePeriods = true } = {}) {

  const word = String.raw`[A-Za-zÀ-ÖØ-öø-ÿ'’-]{2,}`;

  const initialsPattern = initialsRequirePeriods
    // Allows: A.B. / A. B.
    // Rejects: A.B / AB
    ? String.raw`(?:[A-Z]\.\s*)+`
    // Allows: AB / A B
    // Rejects: A.B.
    : String.raw`[A-Z](?:\s*[A-Z])*`;

  const surnameCommaInitialsRegex = new RegExp(
    String.raw`^${word}(?:\s+${word})*,\s*${initialsPattern}$`
  );

  // Allows: Jones, Van der Merwe, School of Biology
  const plainNameRegex = new RegExp(
    String.raw`^${word}(?:\s+${word})*$`
  );

  // Rejects: A.B. Jones, A.B Jones
  const startsWithPeriodInitialsRegex =
    /^(?:[A-Z]\.\s*)+[A-Za-zÀ-ÖØ-öø-ÿ'’-]{2,}$/;

  // Rejects: AB Jones, A B Jones
  const startsWithNoPeriodInitialsRegex =
    /^(?:[A-Z]\s*){1,6}\s+[A-Za-zÀ-ÖØ-öø-ÿ'’-]{2,}/;

  // Rejects: Jones AB, Van der Merwe AB
  const endsWithInitialsNoCommaRegex =
    /\s+[A-Z]{1,6}$/;

  /**
  * @param {string | null} value - The name to test.
  * @returns {boolean} - True if the name matches the filter criteria.
  */
  return function nameFilter(value) {
    const text = String(value ?? "").trim().replace(/\s+/g, " ");

    if (surnameCommaInitialsRegex.test(text)) {
      return true;
    }

    if (!plainNameRegex.test(text)) {
      return false;
    }

    if (startsWithPeriodInitialsRegex.test(text)) {
      return false;
    }

    if (startsWithNoPeriodInitialsRegex.test(text)) {
      return false;
    }

    if (endsWithInitialsNoCommaRegex.test(text)) {
      return false;
    }

    return true;
  };
}