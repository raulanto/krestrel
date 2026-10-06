//! Pure fuzzy matching and accent folding utilities for sidebar search.

/// Folds accented characters to their unaccented lower-case equivalents.
pub fn fold_accents(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'á' | 'à' | 'ä' | 'â' | 'ã' | 'Á' | 'À' | 'Ä' | 'Â' | 'Ã' => 'a',
            'é' | 'è' | 'ë' | 'ê' | 'É' | 'È' | 'Ë' | 'Ê' => 'e',
            'í' | 'ì' | 'ï' | 'î' | 'Í' | 'Ì' | 'Ï' | 'Î' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' | 'õ' | 'Ó' | 'Ò' | 'Ö' | 'Ô' | 'Õ' => 'o',
            'ú' | 'ù' | 'ü' | 'û' | 'Ú' | 'Ù' | 'Ü' | 'Û' => 'u',
            'ñ' | 'Ñ' => 'n',
            'ç' | 'Ç' => 'c',
            c => c.to_ascii_lowercase(),
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchResult {
    pub score: usize,
    pub matched_indices: Vec<usize>,
}

/// Performs a fuzzy subsequence match of `query` inside `text`.
/// Returns `None` if the query does not match as a subsequence.
pub fn fuzzy_match(text: &str, query: &str) -> Option<MatchResult> {
    let clean_query = fold_accents(query.trim());
    if clean_query.is_empty() {
        return Some(MatchResult {
            score: 0,
            matched_indices: Vec::new(),
        });
    }

    let folded_text = fold_accents(text);
    let text_chars: Vec<(usize, char)> = text.char_indices().collect();
    let query_chars: Vec<char> = clean_query.chars().collect();
    let folded_chars: Vec<char> = folded_text.chars().collect();

    let mut query_idx = 0;
    let mut matched_indices = Vec::new();
    let mut consecutive_bonus = 0;
    let mut score = 0;

    let mut i = 0;
    while i < folded_chars.len() && query_idx < query_chars.len() {
        if folded_chars[i] == query_chars[query_idx] {
            let orig_byte_offset = text_chars[i].0;
            matched_indices.push(orig_byte_offset);
            score += 10 + consecutive_bonus;
            consecutive_bonus += 5;
            query_idx += 1;
        } else {
            consecutive_bonus = 0;
        }
        i += 1;
    }

    if query_idx == query_chars.len() {
        Some(MatchResult {
            score,
            matched_indices,
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzzy_match_exact_and_subsequence() {
        let m = fuzzy_match("Users", "usr").expect("Should match 'usr' in 'Users'");
        assert!(!m.matched_indices.is_empty());

        let m2 = fuzzy_match("Get User By ID", "usr");
        assert!(m2.is_some());

        let m3 = fuzzy_match("Users", "xyz");
        assert!(m3.is_none());
    }

    #[test]
    fn test_fuzzy_match_case_and_accents() {
        let m1 = fuzzy_match("Canción", "cancion").expect("Accent folding match");
        assert_eq!(m1.matched_indices.len(), 7);

        let m2 = fuzzy_match("Úsérs", "usr").expect("Accent folding in text");
        assert!(!m2.matched_indices.is_empty());
    }

    #[test]
    fn test_empty_query_matches_all() {
        let m = fuzzy_match("Any text", "  ").expect("Empty query matches with score 0");
        assert_eq!(m.score, 0);
        assert!(m.matched_indices.is_empty());
    }
}
