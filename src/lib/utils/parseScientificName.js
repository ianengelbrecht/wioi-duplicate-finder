/**
 * Parses a scientific name into genus, species, author, and sub-rank details.
 *
 * @param {string|null|undefined} name
 * @returns {{
 *   genus: string,
 *   sp1: string,
 *   author1: string,
 *   rank1: string,
 *   sp2: string,
 *   author2: string
 * }}
 */
export function parseScientificName(name) {
  const result = {
    genus: "",
    sp1: "",
    author1: "",
    rank1: "",
    sp2: "",
    author2: ""
  };
  if (!name) return result;
  const tokens = name.trim().split(/\s+/);
  if (tokens.length === 0) return result;
  
  result.genus = tokens[0];
  if (tokens.length === 1) return result;
  
  const isLowercase = (/** @type {string} */ str) => {
    if (!str) return false;
    const firstChar = str.charAt(0);
    return firstChar >= 'a' && firstChar <= 'z';
  };

  const rankIndicators = [
    "subsp.", "subsp", "var.", "var", "ssp.", "ssp", 
    "forma", "form", "subg.", "subgenus", "sect.", "section",
    "f.", "f"
  ];
  
  let sp1Index = -1;
  if (isLowercase(tokens[1]) || tokens[1] === "x" || tokens[1] === "×") {
    result.sp1 = tokens[1];
    sp1Index = 1;
  }
  
  if (sp1Index === -1) {
    result.author1 = tokens.slice(1).join(" ");
    return result;
  }
  
  let rankIndex = -1;
  for (let i = 2; i < tokens.length; i++) {
    if (rankIndicators.includes(tokens[i].toLowerCase())) {
      rankIndex = i;
      break;
    }
  }
  
  if (rankIndex === -1) {
    result.author1 = tokens.slice(2).join(" ");
  } else {
    result.author1 = tokens.slice(2, rankIndex).join(" ");
    result.rank1 = tokens[rankIndex];
    if (rankIndex + 1 < tokens.length) {
      result.sp2 = tokens[rankIndex + 1];
      result.author2 = tokens.slice(rankIndex + 2).join(" ");
    }
  }
  return result;
}
