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
        "subsp.", "subsp", "var.", "var", "f.", "f", "ssp.", "ssp", 
        "forma", "form", "subg.", "subgenus", "sect.", "section"
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
        assert_eq!(
            normalize_taxon_name(""),
            ""
        );
    }
}
