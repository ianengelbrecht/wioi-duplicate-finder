/**
 * Parse a verbatim elevation string into standardized elevation values.
 *
 * @param {string} verbatimElevation
 * @returns {{
 *   minElevation: number | null,
 *   maxElevation: number | null,
 *   elevation: number | null,
 *   elevationUncertainty: number | null,
 *   elevationUnit: string | null
 * }}
 */
export function parseElevation(verbatimElevation) {
  if (!verbatimElevation || typeof verbatimElevation !== 'string') {
    return {
      minElevation: null,
      maxElevation: null,
      elevation: null,
      elevationUncertainty: null,
      elevationUnit: null
    };
  }

  const text = verbatimElevation.trim().toLowerCase();

  // Determine units
  const isFeet = /(feet|foot|ft|f)\b/i.test(text);
  const conversionFactor = isFeet ? 0.305 : 1;

  // Look for explicit ranges first
  const rangeMatch = text.match(
    /(\d+(?:\.\d+)?)\s*(?:-|–|—|\bto\b)\s*(\d+(?:\.\d+)?)/i
  );

  if (rangeMatch) {
    let v1 = parseFloat(rangeMatch[1]) * conversionFactor;
    let v2 = parseFloat(rangeMatch[2]) * conversionFactor;

    const minElevation = Math.round(Math.min(v1, v2));
    const maxElevation = Math.round(Math.max(v1, v2));

    return {
      minElevation,
      maxElevation,
      elevation: Math.round((minElevation + maxElevation) / 2),
      elevationUncertainty: Math.round(
        (maxElevation - minElevation) / 2
      ),
      elevationUnit: isFeet ? 'ft' : 'm'
    };
  }

  // Fall back to extracting all numbers
  const numbers = [...text.matchAll(/\d+(?:\.\d+)?/g)]
    .map((m) => parseFloat(m[0]));

  if (numbers.length === 0) {
    return {
      minElevation: null,
      maxElevation: null,
      elevation: null,
      elevationUncertainty: null,
      elevationUnit: null

    };
  }

  // If exactly two numbers are present, assume they represent a range
  if (numbers.length === 2) {
    let v1 = numbers[0] * conversionFactor;
    let v2 = numbers[1] * conversionFactor;

    const minElevation = Math.round(Math.min(v1, v2));
    const maxElevation = Math.round(Math.max(v1, v2));

    return {
      minElevation,
      maxElevation,
      elevation: Math.round((minElevation + maxElevation) / 2),
      elevationUncertainty: Math.round(
        (maxElevation - minElevation) / 2
      ),
      elevationUnit: isFeet ? 'ft' : 'm'
    };
  }

  // Otherwise use the first number found
  const elevation = Math.round(numbers[0] * conversionFactor);

  return {
    minElevation: elevation,
    maxElevation: elevation,
    elevation,
    elevationUncertainty: 0,
    elevationUnit: isFeet ? 'ft' : 'm'
  };
}