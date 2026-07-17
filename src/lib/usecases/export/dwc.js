import { parseElevation } from "$lib/utils/parseVerbatimElevation.js";
import { formatISO8601Date } from "$lib/utils/formatISO8601Date.js";

/**
 * Returns the CSV headers for Darwin Core format.
 *
 * @returns {string[]}
 */
export function getDwcHeaders() {
  return [
    "dwc:collectionCode",
    "dwc:catalogNumber",
    "duplicates",
    "dwc:recordNumber",
    "dwc:recordedBy",
    "dwc:verbatimEventDate",
    "dwc:year",
    "dwc:month",
    "dwc:day",
    "dwc:country",
    "dwc:stateProvince",
    "dwc:county",
    "dwc:municipality",
    "dwc:islandGroup",
    "dwc:island",
    "dwc:locality",
    "dwc:locationRemarks",
    "dwc:verbatimCoordinates",
    "dwc:decimalLatitude",
    "dwc:decimalLongitude",
    "dwc:verbatimElevation",
    "minElevation",
    "maxElevation",
    "elevation",
    "elevationUncertainty",
    "dwc:habitat",
    "dwc:occurrenceRemarks",
    "dwc:fieldNotes",
    "dwc:typeStatus",
    "dwc:identificationQualifier",
    "dwc:family",
    "dwc:scientificName",
    "dwc:identifiedBy",
    "dwc:dateIdentified",
    "dwc:identificationRemarks",
    "dwc:taxonID",
    "cultivated",
    "recordSource"
  ];
}

/**
 * Maps a single captured record into the Darwin Core CSV row format.
 *
 * @param {any} rec
 * @param {{familyMap?: Record<number, string>}} [options]
 * @returns {any[]}
 */
export function mapDwcRecord(rec, options = {}) {
  const { familyMap = {} } = options;
  const elevParts = parseElevation(rec.verbatimElevation);
  let dwcLocationRemarks = rec.locationNotes || "";
  if (rec.cultivated) {
    dwcLocationRemarks = dwcLocationRemarks ? `${dwcLocationRemarks}; cultivated` : "cultivated";
  }

  let locality = rec.locality || "";
  if (rec.gridReference && rec.gridReference.trim()) {
    const grid = rec.gridReference.trim();
    if (!locality.includes(grid)) {
      locality = locality ? `${locality} [${grid}]` : grid;
    }
  }

  return [
    rec.collectionCode || "",
    rec.catalogNumber || "",
    rec.duplicates !== null && rec.duplicates !== undefined ? rec.duplicates : "",
    rec.recordNumber || "",
    rec.recordedBy || "",
    rec.verbatimEventDate || "",
    rec.year !== null && rec.year !== undefined ? rec.year : "",
    rec.month !== null && rec.month !== undefined ? rec.month : "",
    rec.day !== null && rec.day !== undefined ? rec.day : "",
    rec.country || "",
    rec.stateProvince || "",
    rec.county || "",
    rec.municipality || "",
    rec.islandGroup || "",
    rec.island || "",
    locality,
    dwcLocationRemarks, // locationRemarks maps to locationNotes in UI record
    rec.verbatimCoordinates || "",
    rec.decimalLatitude !== null && rec.decimalLatitude !== undefined ? rec.decimalLatitude : "",
    rec.decimalLongitude !== null && rec.decimalLongitude !== undefined ? rec.decimalLongitude : "",
    rec.verbatimElevation || "",
    elevParts.minElevation !== null && elevParts.minElevation !== undefined ? elevParts.minElevation : "",
    elevParts.maxElevation !== null && elevParts.maxElevation !== undefined ? elevParts.maxElevation : "",
    elevParts.elevation !== null && elevParts.elevation !== undefined ? elevParts.elevation : "",
    elevParts.elevationUncertainty !== null && elevParts.elevationUncertainty !== undefined ? elevParts.elevationUncertainty : "",
    rec.habitat || "",
    rec.occurrenceRemarks || "",
    rec.fieldNotes || "",
    rec.typeStatus || "",
    rec.identificationQualifier || "",
    familyMap[rec.id] || "",
    rec.scientificName || "",
    rec.identifiedBy || "",
    formatISO8601Date(rec.yearIdentified, rec.monthIdentified, rec.dayIdentified),
    rec.identificationRemarks || "",
    rec.taxonID || "",
    rec.cultivated ? "true" : "false",
    rec.recordSource || ""
  ];
}
