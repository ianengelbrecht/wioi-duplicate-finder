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
/// It also strips common honorifics/titles (like Dr, Prof, Mr, Mrs, Ms, Sir, Lady) so that
/// names with titles (e.g. "I.B. Walters (Dr)" or "Dr. I.B. Walters") match their plain versions.
pub fn normalize_search_recorded_by(s: &str) -> String {
    let title_prefixes = ["dr", "prof", "mr", "mrs", "ms", "sir", "lady"];
    let words: Vec<&str> = s.split_whitespace().collect();
    let cleaned_words: Vec<&str> = words
        .into_iter()
        .filter(|&word| {
            let clean =
                word.trim_matches(|c: char| c.is_ascii_punctuation() || c == '(' || c == ')');
            !title_prefixes.contains(&clean.to_lowercase().as_str())
        })
        .collect();
    let s_cleaned = cleaned_words.join(" ");

    let mapped: String = s_cleaned
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
    tokens.iter().all(|t| {
        let char_count = t.chars().count();
        if char_count == 1 {
            true
        } else if char_count <= 3 {
            t.chars().all(|c| c.is_ascii_uppercase())
        } else {
            false
        }
    })
}

fn split_by_other_delimiters(s: &str) -> Vec<String> {
    let s = s.trim();
    if s.is_empty() {
        return Vec::new();
    }

    if s.contains('|') {
        let mut result = Vec::new();
        for part in s.split('|') {
            result.extend(split_by_other_delimiters(part));
        }
        result
    } else if s.contains(';') {
        let mut result = Vec::new();
        for part in s.split(';') {
            result.extend(split_by_other_delimiters(part));
        }
        result
    } else if s.contains(',') {
        let comma_count = s.matches(',').count();
        if comma_count == 1 {
            let temp_parts: Vec<&str> = s.split(',').collect();
            let part_after = temp_parts[1].trim();
            if is_initials(part_after) {
                vec![s.to_string()]
            } else {
                let mut result = Vec::new();
                for part in temp_parts {
                    result.extend(split_by_other_delimiters(part));
                }
                result
            }
        } else {
            let mut result = Vec::new();
            for part in s.split(',') {
                result.extend(split_by_other_delimiters(part));
            }
            result
        }
    } else {
        vec![s.to_string()]
    }
}

/// Splits a raw collector string containing multiple names separated by |, ;, &, and (word boundaries), or ,
pub fn split_names(raw_str: &str) -> Vec<String> {
    let raw_str = raw_str.trim();
    if raw_str.is_empty() {
        return Vec::new();
    }

    // Find case-insensitive "and" with word boundaries
    let mut and_positions = Vec::new();
    let bytes = raw_str.as_bytes();
    let len = bytes.len();
    if len >= 3 {
        for i in 0..=(len - 3) {
            let matches_and = (bytes[i] == b'a' || bytes[i] == b'A')
                && (bytes[i + 1] == b'n' || bytes[i + 1] == b'N')
                && (bytes[i + 2] == b'd' || bytes[i + 2] == b'D');
            if matches_and {
                let boundary_before = if i == 0 {
                    true
                } else {
                    let prev_char = raw_str[..i].chars().next_back().unwrap();
                    !prev_char.is_alphanumeric()
                };
                let boundary_after = if i + 3 == len {
                    true
                } else {
                    let next_char = raw_str[i + 3..].chars().next().unwrap();
                    !next_char.is_alphanumeric()
                };
                if boundary_before && boundary_after {
                    and_positions.push(i);
                }
            }
        }
    }

    // Split on "and" first
    let mut and_split_parts = Vec::new();
    if !and_positions.is_empty() {
        let mut last_idx = 0;
        for pos in and_positions {
            and_split_parts.push(raw_str[last_idx..pos].to_string());
            last_idx = pos + 3;
        }
        and_split_parts.push(raw_str[last_idx..].to_string());
    } else {
        and_split_parts.push(raw_str.to_string());
    }

    // Split on "&" for all and_split_parts
    let mut amp_split_parts = Vec::new();
    for part in and_split_parts {
        if part.contains('&') {
            for sub in part.split('&') {
                amp_split_parts.push(sub.to_string());
            }
        } else {
            amp_split_parts.push(part);
        }
    }

    // Now split each part on the other delimiters recursively
    let mut final_parts = Vec::new();
    for part in amp_split_parts {
        let trimmed = part.trim();
        if !trimmed.is_empty() {
            let sub_parts = split_by_other_delimiters(trimmed);
            final_parts.extend(sub_parts);
        }
    }

    final_parts
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
        assert_eq!(
            normalize_search_recorded_by("I.B. Walters (Dr)"),
            "IB WALTERS"
        );
        assert_eq!(
            normalize_search_recorded_by("Dr. I.B. Walters"),
            "IB WALTERS"
        );
        assert_eq!(
            normalize_search_recorded_by("Prof. O.M. Hilliard"),
            "OM HILLIARD"
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

    #[test]
    fn test_is_initials() {
        assert!(is_initials("A."));
        assert!(is_initials("A"));
        assert!(is_initials("AJ"));
        assert!(is_initials("AJ."));
        assert!(is_initials("A.J."));
        assert!(is_initials("A. J."));
        assert!(!is_initials("Jones"));
        assert!(!is_initials("SMITH"));
        assert!(!is_initials("A. Jones"));
    }

    #[test]
    fn test_split_names() {
        assert_eq!(split_names("Verbist, AJ"), vec!["Verbist, AJ".to_string()]);
        assert_eq!(split_names("Verbist, A."), vec!["Verbist, A.".to_string()]);
        assert_eq!(
            split_names("Verbist A., Jones B."),
            vec!["Verbist A.".to_string(), "Jones B.".to_string()]
        );
        assert_eq!(
            split_names("O. M. Hilliard & B. L. Burtt"),
            vec!["O. M. Hilliard".to_string(), "B. L. Burtt".to_string()]
        );
        assert_eq!(
            split_names("O. M. Hilliard and B. L. Burtt"),
            vec!["O. M. Hilliard".to_string(), "B. L. Burtt".to_string()]
        );
        assert_eq!(
            split_names("O. M. Hilliard AND B. L. Burtt"),
            vec!["O. M. Hilliard".to_string(), "B. L. Burtt".to_string()]
        );
        assert_eq!(
            split_names("Anderson & Brand"),
            vec!["Anderson".to_string(), "Brand".to_string()]
        );
        assert_eq!(
            split_names("Anderson and Brand"),
            vec!["Anderson".to_string(), "Brand".to_string()]
        );
        assert_eq!(
            split_names("M. A. García, G. López, L. Mucina & D. Nickrent"),
            vec![
                "M. A. García".to_string(),
                "G. López".to_string(),
                "L. Mucina".to_string(),
                "D. Nickrent".to_string()
            ]
        );
    }
}
