import Papa from "papaparse";
import { getDwcHeaders, mapDwcRecord } from "./export/dwc.js";
import { getBrahmsHeaders, mapBrahmsRecord } from "./export/brahms.js";

/**
 * Generates CSV string content based on chosen format and records list.
 *
 * @param {any[]} records
 * @param {string} format "DwC" or "BRAHMS"
 * @param {Record<number, string>} [familyMap] Resolved taxon families mapping
 * @param {boolean} [includeQDS] Include QDS references
 * @returns {string}
 */
export function generateCSVContent(records, format, familyMap = {}, includeQDS = false) {
  if (format === "DwC") {
    const headers = getDwcHeaders();
    const data = records.map(rec => mapDwcRecord(rec, { familyMap }));
    return Papa.unparse({ fields: headers, data });
  } else if (format === "BRAHMS") {
    const headers = getBrahmsHeaders({ includeQDS });
    const data = records.map(rec => mapBrahmsRecord(rec, { familyMap, includeQDS }));
    return Papa.unparse({ fields: headers, data });
  }
  throw new Error(`Unsupported export format: ${format}`);
}
