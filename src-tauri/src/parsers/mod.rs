/// Normalizes a scientific name by keeping only the lowercase epithets in order.
/// It keeps the Genus (the first word, lowercased), and skips:
/// - Rank indicators (subsp., var., f., ssp., etc.)
/// - Capitalized author names (including parenthesized authors)
/// - Common connectors (ex, in, and, &)
pub fn normalize_taxon_name(name: &str) -> String {
    let words: Vec<&str> = name.split_whitespace().collect();
    if words.is_empty() {
        return String::new();
    }

    let mut normalized_words = Vec::new();

    // 1. Keep the first word (genus)
    normalized_words.push(words[0].to_lowercase());

    // Known rank indicators to skip
    let rank_indicators = [
        "subsp.", "subsp", "var.", "var", "f.", "f", "ssp.", "ssp", "forma", "form", "subg.",
        "subgenus", "sect.", "section",
    ];

    // Known author connectors to skip
    let connectors = ["ex", "in", "and", "&"];

    for &word in words.iter().skip(1) {
        // Strip punctuation like parentheses, brackets, periods for checking, e.g. "(L.)" -> "L"
        let clean_word = word.trim_matches(|c: char| c.is_ascii_punctuation());

        if clean_word.is_empty() {
            continue;
        }

        let lower_word = clean_word.to_lowercase();

        // Skip rank indicators
        if rank_indicators.contains(&lower_word.as_str()) {
            continue;
        }

        // Skip connectors
        if connectors.contains(&lower_word.as_str()) {
            continue;
        }

        // Skip author names (if clean_word starts with an uppercase letter)
        if let Some(first_char) = clean_word.chars().next() {
            if first_char.is_uppercase() {
                continue;
            }
        }

        // If it reaches here, it is a lowercase epithet! Keep it.
        normalized_words.push(lower_word);
    }

    normalized_words.join(" ")
}

/// Normalizes a locality string by:
/// 1. Replacing accented characters (French & English) with base ASCII characters.
/// 2. Converting to lowercase.
/// 3. Replacing non-alphanumeric characters with spaces.
/// 4. Filtering out grammatical and prepositional stop words in English and French.
/// 5. Removing single-letter connector words (like 'd' or 'l') unless they are numbers.
pub fn normalize_locality(s: &str) -> String {
    let mut mapped = String::new();
    for c in s.chars() {
        let mc = match c {
            'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' => {
                'a'
            }
            'è' | 'é' | 'ê' | 'ë' | 'È' | 'É' | 'Ê' | 'Ë' => 'e',
            'ì' | 'í' | 'î' | 'ï' | 'Ì' | 'Í' | 'Î' | 'Ï' => 'i',
            'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' | 'Ø' => {
                'o'
            }
            'ù' | 'ú' | 'û' | 'ü' | 'Ù' | 'Ú' | 'Û' | 'Ü' => 'u',
            'ñ' | 'Ñ' => 'n',
            'ç' | 'Ç' => 'c',
            'ý' | 'ÿ' | 'Ý' | 'Ÿ' => 'y',
            _ => c.to_ascii_lowercase(),
        };
        if mc.is_ascii_alphanumeric() {
            mapped.push(mc);
        } else {
            mapped.push(' ');
        }
    }

    let stopwords = [
        // English
        "the", "a", "an", "and", "or", "near", "about", "along", "to", "of", "in", "on", "at", "by",
        "from", "with", "under", "over", "between", "around", // French
        "le", "la", "les", "un", "une", "des", "du", "de", "et", "ou", "pres", "vers", "dans",
        "sur", "sous", "par", "pour", "avec", "chez", "au", "aux",
    ];

    let mut result_words = Vec::new();
    for word in mapped.split_whitespace() {
        if stopwords.contains(&word) {
            continue;
        }
        if word.len() == 1 && !word.chars().next().unwrap().is_ascii_digit() {
            continue;
        }
        result_words.push(word);
    }

    result_words.join(" ")
}

/// Normalizes a collector name for searchRecordedBy (uppercase, alphanumeric + spaces only).
pub fn normalize_search_recorded_by(s: &str) -> String {
    let mapped: String = s
        .chars()
        .map(|c| match c {
            'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' => {
                'A'
            }
            'è' | 'é' | 'ê' | 'ë' | 'È' | 'É' | 'Ê' | 'Ë' => 'E',
            'ì' | 'í' | 'î' | 'ï' | 'Ì' | 'Í' | 'Î' | 'Ï' => 'I',
            'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' | 'Ø' => {
                'O'
            }
            'ù' | 'ú' | 'û' | 'ü' | 'Ù' | 'Ú' | 'Û' | 'Ü' => 'U',
            'ñ' | 'Ñ' => 'N',
            'ç' | 'Ç' => 'C',
            'ý' | 'ÿ' | 'Ý' | 'Ÿ' => 'Y',
            _ => c.to_ascii_uppercase(),
        })
        .filter(|&c| c.is_ascii_alphanumeric() || c == ' ')
        .collect();

    // Remove double/multiple spaces
    let mut result = String::new();
    let mut last_was_space = false;
    for c in mapped.trim().chars() {
        if c == ' ' {
            if !last_was_space {
                result.push(c);
                last_was_space = true;
            }
        } else {
            result.push(c);
            last_was_space = false;
        }
    }
    result
}

/// Helper function to check if a string consists entirely of initials
pub fn is_initials(s: &str) -> bool {
    let tokens: Vec<&str> = s.split([' ', '.']).filter(|t| !t.is_empty()).collect();
    if tokens.is_empty() {
        return false;
    }
    tokens.iter().all(|t| t.chars().count() == 1)
}

/// Splits a raw collector string containing multiple names separated by |, ;, or ,
pub fn split_names(raw_str: &str) -> Vec<String> {
    let raw_str = raw_str.trim();
    if raw_str.is_empty() {
        return Vec::new();
    }

    let parts = if raw_str.contains('|') {
        raw_str.split('|').map(|s| s.to_string()).collect()
    } else if raw_str.contains(';') {
        raw_str.split(';').map(|s| s.to_string()).collect()
    } else if raw_str.contains(',') {
        let comma_count = raw_str.matches(',').count();
        if comma_count == 1 {
            let temp_parts: Vec<&str> = raw_str.split(',').collect();
            let part_after = temp_parts[1].trim();
            if is_initials(part_after) {
                vec![raw_str.to_string()]
            } else {
                temp_parts.iter().map(|s| s.to_string()).collect()
            }
        } else {
            raw_str.split(',').map(|s| s.to_string()).collect()
        }
    } else {
        vec![raw_str.to_string()]
    };

    parts
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Extracts only the digit sequences from a record number, separating them by spaces
pub fn extract_digits(s: &str) -> String {
    let mut result = String::new();
    let mut in_digit = false;
    for c in s.chars() {
        if c.is_ascii_digit() {
            if !in_digit && !result.is_empty() {
                result.push(' ');
            }
            result.push(c);
            in_digit = true;
        } else {
            in_digit = false;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_taxon_name() {
        assert_eq!(
            normalize_taxon_name("Abelmoschus manihot (L.) Medik."),
            "abelmoschus manihot"
        );
        assert_eq!(
            normalize_taxon_name("Aus bus subsp. cus Author"),
            "aus bus cus"
        );
        assert_eq!(
            normalize_taxon_name("Acanthosicyos horridus Welw. ex Benth. & Hook.f."),
            "acanthosicyos horridus"
        );
        assert_eq!(
            normalize_taxon_name("Senebiera heleniana (L.) DC. var. heleniana"),
            "senebiera heleniana heleniana"
        );
        assert_eq!(normalize_taxon_name(""), "");
    }

    #[test]
    fn test_normalize_search_recorded_by() {
        assert_eq!(
            normalize_search_recorded_by("Müller-Landry"),
            "MULLERLANDRY"
        );
        assert_eq!(normalize_search_recorded_by("Smith, J."), "SMITH J");
        assert_eq!(
            normalize_search_recorded_by("J. René Smith"),
            "J RENE SMITH"
        );
    }

    #[test]
    fn test_normalize_locality() {
        assert_eq!(
            normalize_locality("near the forest of Antananarivo"),
            "forest antananarivo"
        );
        assert_eq!(
            normalize_locality("près de la forêt d'Antananarivo"),
            "foret antananarivo"
        );
        assert_eq!(
            normalize_locality("Along Route 3, 5 km west of town"),
            "route 3 5 km west town"
        );
    }
}
