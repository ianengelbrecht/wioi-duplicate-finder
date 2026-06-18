import { parseElevation } from "$lib/utils/parseVerbatimElevation.js";
import { coordsToQDS } from "$lib/utils/coordsToQDS.js";
import { parseCollectorNumber } from "$lib/utils/parseCollectorNumber.js";
import { parseScientificName } from "$lib/utils/parseScientificName.js";
import { coordinatesDiffer } from "$lib/utils/coordinatesDiffer.js";
import { isSimpleElevation } from "$lib/utils/isSimpleElevation.js";
import { splitNames } from "$lib/utils/splitNames.js";

/**
 * Returns the CSV headers for BRAHMS format.
 *
 * @param {{includeQDS?: boolean}} [options]
 * @returns {string[]}
 */
export function getBrahmsHeaders(options = {}) {
  const headers = [
    "tag", "del", "barcode", "dups",
    "collector", "addcol", "prefix", "number", "suffix",
    "dd", "mm", "yy",
    "family",
    "type category",
    "genus", "sp1", "author1", "rank1", "sp2", "author2",
    "detby", "detdd", "detmm", "detyy", "detstatus",
    "country", "majorarea", "minorarea", "gazetteer",
    "lat", "long", "ns", "ew", "llunit"
  ];
  if (options.includeQDS) {
    headers.push("qds");
  }
  headers.push(
    "alt", "altunit",
    "locality notes",
    "habitat/site description",
    "plant description", "cultivated",
    "general notes"
  );
  return headers;
}

/**
 * Maps a single captured record into the BRAHMS CSV row format.
 *
 * @param {any} rec
 * @param {{familyMap?: Record<number, string>, includeQDS?: boolean}} [options]
 * @returns {any[]}
 */
export function mapBrahmsRecord(rec, options = {}) {
  const { familyMap = {}, includeQDS = false } = options;

  const duplicates = rec.duplicates ? rec.collectionCode + "," + rec.duplicates.replace(/\s*/g, "") : rec.collectionCode;

  const collectors = splitNames(rec.recordedBy);
  const collector = collectors[0] || "";
  const addcol = collectors.slice(1).join("; ");

  const colNumParts = parseCollectorNumber(rec.recordNumber);

  const family = familyMap[rec.id] || "";

  const nameParts = parseScientificName(rec.scientificName);

  let gazetteer = rec.locality || "";
  if (rec.municipality && rec.municipality.trim()) {
    const muni = rec.municipality.trim();
    const locLower = (rec.locality || "").toLowerCase();
    const remarksLower = (rec.locationNotes || "").toLowerCase();
    if (!locLower.includes(muni.toLowerCase()) && !remarksLower.includes(muni.toLowerCase())) {
      gazetteer = rec.locality ? `${muni}, ${rec.locality}` : muni;
    }
  }

  const hasLat = rec.decimalLatitude !== null && rec.decimalLatitude !== undefined && rec.decimalLatitude !== "";
  const hasLon = rec.decimalLongitude !== null && rec.decimalLongitude !== undefined && rec.decimalLongitude !== "";
  const lat = hasLat ? Math.abs(Number(rec.decimalLatitude)) : "";
  const long = hasLon ? Math.abs(Number(rec.decimalLongitude)) : "";
  const ns = hasLat ? (Number(rec.decimalLatitude) >= 0 ? "N" : "S") : "";
  const ew = hasLon ? (Number(rec.decimalLongitude) >= 0 ? "E" : "W") : "";

  let qdsVal = "";
  if (includeQDS) {
    try {
      qdsVal = coordsToQDS(rec.decimalLatitude, rec.decimalLongitude) || "";
    } catch (e) {
      qdsVal = "";
    }
  }

  const elevParts = parseElevation(rec.verbatimElevation);
  const alt = elevParts.elevation !== null && elevParts.elevation !== undefined ? elevParts.elevation : "";

  let localityNotes = rec.locationNotes || "";
  if (rec.cultivated) {
    localityNotes = localityNotes ? `${localityNotes}; cultivated` : "cultivated";
  }
  if (coordinatesDiffer(rec.verbatimCoordinates, rec.decimalLatitude, rec.decimalLongitude)) {
    const coordNote = `verbatim coordinates: ${rec.verbatimCoordinates}`;
    localityNotes = localityNotes ? `${localityNotes}. ${coordNote}` : coordNote;
  }
  if (!isSimpleElevation(rec.verbatimElevation)) {
    const elevNote = `verbatim elevation: ${rec.verbatimElevation}`;
    localityNotes = localityNotes ? `${localityNotes}. ${elevNote}` : elevNote;
  }

  let generalNotes = rec.occurrenceRemarks || "";
  if (rec.identificationRemarks && rec.identificationRemarks.trim()) {
    const detNotes = `detnotes: ${rec.identificationRemarks.trim()}`;
    generalNotes = generalNotes ? `${generalNotes}. ${detNotes}` : detNotes;
  }

  const row = [
    "", // tag
    "", // del
    rec.catalogNumber || "", // barcode
    duplicates, // dups
    collector,
    addcol,
    colNumParts.prefix,
    colNumParts.number,
    colNumParts.suffix,
    rec.day !== null && rec.day !== undefined ? rec.day : "",
    rec.month !== null && rec.month !== undefined ? rec.month : "",
    rec.year !== null && rec.year !== undefined ? rec.year : "",
    family,
    rec.typeStatus || "", // type category
    nameParts.genus,
    nameParts.sp1,
    nameParts.author1,
    nameParts.rank1,
    nameParts.sp2,
    nameParts.author2,
    rec.dayIdentified !== null && rec.dayIdentified !== undefined ? rec.dayIdentified : "",
    rec.monthIdentified !== null && rec.monthIdentified !== undefined ? rec.monthIdentified : "",
    rec.yearIdentified !== null && rec.yearIdentified !== undefined ? rec.yearIdentified : "",
    splitNames(rec.identifiedBy)[0] || "", // detby
    rec.identificationQualifier || "", // detstatus
    rec.country || "",
    rec.stateProvince || "", // majorarea
    rec.county || "", // minorarea
    gazetteer,
    lat,
    long,
    ns,
    ew,
    "DD" // llunit
  ];

  if (includeQDS) {
    row.push(qdsVal);
  }

  row.push(
    alt,
    "", // altunit
    localityNotes,
    rec.habitat || "", // habitat/site description
    rec.fieldNotes || "", // plant description
    rec.cultivated ? "true" : "false", // cultivated
    generalNotes
  );

  return row;
}
