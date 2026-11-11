use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

use crate::data::Catalog;

/// Rank entries for `query`. Short fields (name, display code, combo) are fuzzy
/// matched; the full text must contain every query word as a substring. Either
/// is enough to match, both rank highest.
pub fn search(cat: &Catalog, matcher: &SkimMatcherV2, query: &str) -> Vec<usize> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Vec::new();
    }
    let words: Vec<&str> = q.split_whitespace().collect();

    let mut scored: Vec<(i64, usize)> = cat
        .entries
        .iter()
        .enumerate()
        .filter_map(|(i, e)| {
            let fuzzy = [Some(&e.name), e.display.as_ref(), e.combo.as_ref(), e.short.as_ref()]
                .into_iter()
                .flatten()
                .enumerate()
                .filter_map(|(rank, field)| {
                    // Name matches beat display/combo matches.
                    let weight = if rank == 0 { 2 } else { 1 };
                    matcher.fuzzy_match(field, &q).map(|s| s * weight)
                })
                .max();
            let words_hit = words.iter().all(|w| cat.haystack[i].contains(w));
            let exact_name = e.name.to_lowercase().contains(&q);
            let score = match (fuzzy, words_hit) {
                (Some(s), true) => s + 200,
                (Some(s), false) => s,
                (None, true) => 100,
                (None, false) => return None,
            } + if exact_name { 300 } else { 0 };
            Some((score, i))
        })
        .collect();

    scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    scored.into_iter().map(|(_, i)| i).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn top(q: &str) -> String {
        let cat = Catalog::load().unwrap();
        let m = SkimMatcherV2::default();
        let r = search(&cat, &m, q);
        cat.entries[r[0]].name.clone()
    }

    #[test]
    fn finds_by_name_display_and_text() {
        assert_eq!(top("factory reset"), "Factory reset");
        assert_eq!(top("rL.Pd"), "Reload Pad");
        assert!(top("swing").to_lowercase().contains("shuffle") || top("swing").contains("Swing"));
    }
}
