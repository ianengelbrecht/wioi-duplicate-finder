/**
 * Compares verbatim coordinates with decimal latitude and longitude to determine if they differ.
 *
 * @param {string|null|undefined} verbatim
 * @param {number|string|null|undefined} lat
 * @param {number|string|null|undefined} lon
 * @returns {boolean}
 */
export function coordinatesDiffer(verbatim, lat, lon) {
  if (!verbatim) return false;
  if (lat === null || lat === undefined || lon === null || lon === undefined || lat === "" || lon === "") return true;
  
  const cleanVerbatim = verbatim.replace(/\s+/g, "");
  const cleanDec = `${lat},${lon}`;
  const cleanDecAbs = `${Math.abs(Number(lat))},${Math.abs(Number(lon))}`;
  
  if (cleanVerbatim === cleanDec || cleanVerbatim === cleanDecAbs) {
    return false;
  }
  
  const parts = verbatim.split(/[\s,]+/);
  if (parts.length === 2) {
    const vLat = parseFloat(parts[0]);
    const vLon = parseFloat(parts[1]);
    if (!isNaN(vLat) && !isNaN(vLon)) {
      if (Math.abs(vLat - Number(lat)) < 0.00001 && Math.abs(vLon - Number(lon)) < 0.00001) {
        return false;
      }
    }
  }
  
  return true;
}
