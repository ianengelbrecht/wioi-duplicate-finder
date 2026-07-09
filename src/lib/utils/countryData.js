/**
 * @typedef {Object} Placeholders
 * @property {string} country
 * @property {string} stateProvince
 * @property {string} county
 * @property {string} locality
 * @property {string} localityNotes
 * @property {string} verbatimCoordinates
 * @property {string} searchCountry
 * @property {string} searchAdmin
 * @property {string} searchLocality
 */

/**
 * @typedef {Object} CountryConfig
 * @property {string[]} languages
 * @property {Record<string, Placeholders>} placeholders
 */

/** @type {Placeholders} */
export const DEFAULT_PLACEHOLDERS = {
  country: "eg South Africa",
  stateProvince: "eg Province/State",
  county: "eg Region/County",
  locality: "eg Locality description",
  localityNotes: "eg 'Found in humid forest'",
  verbatimCoordinates: "eg 20°34'S, 47°12'E",
  searchCountry: "Partial ex. Mad",
  searchAdmin: "Partial eg 'Itas'",
  searchLocality: "Partial search eg 'Anta'"
};

/** @type {Record<string, CountryConfig>} */
export const COUNTRY_DATA = {
  "South Africa": {
    languages: ["EN"],
    placeholders: {
      EN: {
        country: "eg South Africa",
        stateProvince: "eg KwaZulu-Natal",
        county: "eg Ugu",
        locality: "eg Port Shepstone",
        localityNotes: "eg 2km south, on coastal dunes",
        verbatimCoordinates: "eg 30°45'S, 30°27'E",
        searchCountry: "eg South Africa",
        searchAdmin: "eg KwaZulu-Natal",
        searchLocality: "eg Port Shepstone"
      }
    }
  },
  "Madagascar": {
    languages: ["MG", "FR", "EN"],
    placeholders: {
      MG: {
        country: "oh. Madagasikara",
        stateProvince: "oh. Fianarantsoa",
        county: "oh. Amoron'i Mania",
        locality: "Ambohimahasoa, eny amoron-dalana",
        localityNotes: "oh. 10km avaratr', eny amoron-dalana",
        verbatimCoordinates: "oh. 20°43'S, 47°18'E",
        searchCountry: "oh. Madagasikara",
        searchAdmin: "oh. Fianarantsoa",
        searchLocality: "oh. Ambohimahasoa"
      },
      FR: {
        country: "ex. Madagascar",
        stateProvince: "ex. Fianarantsoa",
        county: "ex. Amoron'i Mania",
        locality: "Ambositra",
        localityNotes: "ex. 10km nord, sur la route d'Antsirabe",
        verbatimCoordinates: "ex. 20°34'S, 47°12'E",
        searchCountry: "ex. Madagascar",
        searchAdmin: "ex. Fianarantsoa",
        searchLocality: "ex. Ambositra"
      },
      EN: {
        country: "eg Madagascar",
        stateProvince: "eg Fianarantsoa",
        county: "eg Amoron'i Mania",
        locality: "eg Ambositra",
        localityNotes: "eg 10km north, on road to Antsirabe",
        verbatimCoordinates: "eg 20°34'S, 47°12'E",
        searchCountry: "eg Madagascar",
        searchAdmin: "eg Fianarantsoa",
        searchLocality: "eg Ambositra"
      }
    }
  },
  "Mauritius": {
    languages: ["EN", "FR"],
    placeholders: {
      EN: {
        country: "eg Mauritius",
        stateProvince: "eg Plaines Wilhems",
        county: "eg Curepipe",
        locality: "eg Black River Gorges National Park",
        localityNotes: "eg near the viewpoint",
        verbatimCoordinates: "eg 20°25'S, 57°28'E",
        searchCountry: "eg Mauritius",
        searchAdmin: "eg Plaines Wilhems",
        searchLocality: "eg Black River"
      },
      FR: {
        country: "ex. Maurice",
        stateProvince: "ex. Plaines Wilhems",
        county: "ex. Curepipe",
        locality: "ex. Parc National des Gorges de la Rivière Noire",
        localityNotes: "ex. près du point de vue",
        verbatimCoordinates: "ex. 20°25'S, 57°28'E",
        searchCountry: "ex. Maurice",
        searchAdmin: "ex. Plaines Wilhems",
        searchLocality: "ex. Rivière Noire"
      }
    }
  },
  "Mayotte": {
    languages: ["FR", "EN"],
    placeholders: {
      EN: {
        country: "eg Mayotte",
        stateProvince: "eg Grande-Terre",
        county: "eg Mamoudzou",
        locality: "eg Mount Choungui",
        localityNotes: "eg ridge trail",
        verbatimCoordinates: "eg 12°57'S, 45°08'E",
        searchCountry: "eg Mayotte",
        searchAdmin: "eg Grande-Terre",
        searchLocality: "eg Choungui"
      },
      FR: {
        country: "ex. Mayotte",
        stateProvince: "ex. Grande-Terre",
        county: "ex. Mamoudzou",
        locality: "ex. Mont Choungui",
        localityNotes: "ex. sentier de crête",
        verbatimCoordinates: "ex. 12°57'S, 45°08'E",
        searchCountry: "ex. Mayotte",
        searchAdmin: "ex. Grande-Terre",
        searchLocality: "ex. Choungui"
      },
    }
  },
  "Reunion": {
    languages: ["FR", "EN"],
    placeholders: {
      FR: {
        country: "ex. La Réunion",
        stateProvince: "ex. Arrondissement de Saint-Denis",
        county: "ex. Saint-Denis",
        locality: "ex. Roche Écrite",
        localityNotes: "ex. sentier depuis le Brûlé",
        verbatimCoordinates: "ex. 21°00'S, 55°26'E",
        searchCountry: "ex. La Réunion",
        searchAdmin: "ex. Saint-Denis",
        searchLocality: "ex. Roche Écrite"
      },
      EN: {
        country: "eg Reunion",
        stateProvince: "eg Arrondissement de Saint-Denis",
        county: "eg Saint-Denis",
        locality: "eg Roche Ecrite",
        localityNotes: "eg trail from Le Brule",
        verbatimCoordinates: "eg 21°00'S, 55°26'E",
        searchCountry: "eg Reunion",
        searchAdmin: "eg Saint-Denis",
        searchLocality: "eg Roche Ecrite"
      }
    }
  },
  "Comoros": {
    languages: ["FR", "EN"],
    placeholders: {
      FR: {
        country: "ex. Comores",
        stateProvince: "ex. Grande Comore",
        county: "ex. Moroni",
        locality: "ex. Pente du Karthala",
        localityNotes: "ex. zone forestière",
        verbatimCoordinates: "ex. 11°45'S, 43°22'E",
        searchCountry: "ex. Comores",
        searchAdmin: "ex. Grande Comore",
        searchLocality: "ex. Karthala"
      },
      EN: {
        country: "eg Comoros",
        stateProvince: "eg Grande Comore",
        county: "eg Moroni",
        locality: "eg Mount Karthala slopes",
        localityNotes: "eg , forest zone",
        verbatimCoordinates: "eg 11°45'S, 43°22'E",
        searchCountry: "eg Comoros",
        searchAdmin: "eg Grande Comore",
        searchLocality: "eg Karthala"
      }
    }
  }
};

/**
 * Retrieves the placeholders based on country and active language.
 * Falls back to default values if not configured.
 * @param {string} country
 * @param {string} language
 * @returns {Placeholders}
 */
export function getPlaceholders(country, language) {
  if (!country || !COUNTRY_DATA[country]) {
    return DEFAULT_PLACEHOLDERS;
  }
  const config = COUNTRY_DATA[country];
  const langKey = language.toUpperCase();
  if (config.placeholders[langKey]) {
    return config.placeholders[langKey];
  }
  // Fallback to English placeholder if available, or first available language
  if (config.placeholders["EN"]) {
    return config.placeholders["EN"];
  }
  const firstLang = config.languages[0];
  return config.placeholders[firstLang];
}
