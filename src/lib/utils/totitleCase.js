import { titleCase, SMALL_WORDS } from "title-case";

/**
 * Converts a string to title case with support for specific geographic small words in EN/FR.
 *
 * @param {string|null|undefined} str - The string to convert.
 * @returns {string} The title cased string.
 */
export function totitleCase(str) {
  if (!str) return "";
  const smallWords = new Set([
    ...SMALL_WORDS,
    "along", "from", "towards", "above", "below", "road", "km", "mi", "m", "ft", "side", "slope", "bank", "valley", "ridge", "peak", "mountain", "hill",
    "island", "peninsula", "cape", "bay", "gulf", "strait", "channel", "canyon", "cliff", "plateau", "desert", "forest", "wood", "swamp", "marsh",
    "river", "creek", "stream", "lake", "pond", "spring", "waterfall", "glacier", "volcano", "harbor", "fjord", "delta", "ocean", "sea", "beach", 
    "coast", "shore", "isle", "avenue", "street", "boulevard", "drive", "lane", "court", "square", "parkway", "trail", "terrace", "place",
    "le long de", "de", "vers", "au-dessus de", "au-dessous de",
    "route", "rd", "rd.", "hwy", "hwy.", "st", "st.", "ave", "ave.", "blvd", "blvd.", "dr", "dr.", "ln", "ln.", "ct", "ct.", "sq", "sq.", "pkwy", "pkwy.",
    "trl", "trl.", "ter", "ter.", "pl", "pl.", "mt", "mt.", "mtn", "mtn.", "mts", "mts.", "pk", "pk.", "pt", "pt.", "isl", "isl.", "is", "is.", "pen", "pen.",
    "riv", "riv.", "cr", "cr.", "ck", "ck.", "str", "str.", "lk", "lk.", "pd", "pd.", "spr", "spr.", "falls", "val", "val.", "vly", "vly.", "rdg", "rdg.",
    "cl", "cl.", "plt", "plt.", "for", "for.", "wd", "wd.", "sw", "sw.", "mar", "mar.", "har", "har.", "fj", "fj.", "del", "del.", "oc", "oc.", "sea",
    "bch", "bch.", "cst", "cst.", "shr", "shr.", "n", "s", "e", "w", "ne", "nw", "se", "sw", "nne", "ene", "ese", "sse", "ssw", "wsw", "wnw", "nnw",
    "km", "mi", "m", "ft",   
    "côté", "pente", "rive", "vallée", "crête", "sommet", "montagne", "colline",  
    "île", "péninsule", "cap", "baie", "golfe", "détroit", "chenal", "canyon",
    "falaise", "plateau", "désert", "forêt", "bois", "marécage", "marais",
    "rivière", "ruisseau", "cours d'eau", "lac", "étang", "source", "cascade",
    "glacier", "volcan", "port", "fjord", "delta", "océan", "mer", "plage",
    "côte", "rivage", "îlot",
    "avenue", "rue", "boulevard", "allée", "voie", "cour", "place",
    "promenade", "sentier", "terrasse", "lieu",
    "riv.", "mt", "ste", "st", "rte", "av."
  ]);
  const directions = [
    'N', 'S', 'E', 'W',
    'NE', 'NW', 'SE', 'SW',
    'NNE', 'ENE', 'ESE', 'SSE',
    'SSW', 'WSW', 'WNW', 'NNW'
  ];
  const regex = new RegExp(`\\b(${directions.join('|')})\\b`, 'gi');

  return titleCase(str.toLowerCase(), { smallWords }).replace(regex, match => match.toUpperCase());
}
