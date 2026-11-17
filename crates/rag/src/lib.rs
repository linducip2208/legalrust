#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScoredChunk {
    pub doc_id: String,
    pub version_id: i64,
    pub page: u32,
    pub text: String,
    pub score: f32,
}
pub fn chunk_text(text: &str, size: usize, overlap: usize) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() || size == 0 {
        return Vec::new();
    }
    let step = size.saturating_sub(overlap).max(1);
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        out.push(chars[i..(i + size).min(chars.len())].iter().collect());
        i += step;
    }
    out
}
/// Lowercase, trim, and collapse inner whitespace (pure, offline).
pub fn normalize(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Tokenize text into lowercase alphanumeric terms (pure, offline).
pub fn tokenize_id(text: &str) -> Vec<String> {
    normalize(text)
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .map(str::to_string)
        .collect()
}

/// Case-insensitive term-overlap score in `[0.0, 1.0]`:
/// `|query_terms ∩ chunk_terms| / |query_terms|`. Returns `0.0` for an
/// empty query.
pub fn score_keyword(query: &str, chunk: &str) -> f32 {
    let q = tokenize_id(query);
    if q.is_empty() {
        return 0.0;
    }
    let c = tokenize_id(chunk);
    if c.is_empty() {
        return 0.0;
    }
    let hits = q.iter().filter(|t| c.contains(t)).count();
    hits as f32 / q.len() as f32
}

/// Score `chunks` against `query` with [`score_keyword`], sort
/// descending by score, and return at most `limit` entries (pure).
pub fn rank(query: &str, chunks: &[ScoredChunk], limit: usize) -> Vec<ScoredChunk> {
    let mut out: Vec<ScoredChunk> = chunks
        .iter()
        .map(|c| {
            let mut s = c.clone();
            s.score = score_keyword(query, &c.text);
            s
        })
        .collect();
    out.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out.truncate(limit);
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chunks_overlap() {
        let c = chunk_text("abcdefghij", 4, 2);
        assert_eq!(c, vec!["abcd", "cdef", "efgh", "ghij", "ij"]);
    }
    #[test]
    fn normalize_collapses_ws_and_lowercases() {
        assert_eq!(
            normalize("  Akta  NOTARIS\n\tJakarta "),
            "akta notaris jakarta"
        );
    }
    #[test]
    fn tokenize_splits_terms() {
        assert_eq!(
            tokenize_id("Hak-Milik No. 123"),
            vec!["hak", "milik", "no", "123"]
        );
    }
    #[test]
    fn score_is_case_insensitive_overlap() {
        assert!((score_keyword("hak milik", "HAK MILIK tanah") - 1.0).abs() < 1e-6);
        assert!((score_keyword("hak milik", "hak guna") - 0.5).abs() < 1e-6);
        assert_eq!(score_keyword("", "anything"), 0.0);
    }
    #[test]
    fn rank_orders_and_limits() {
        let chunks = vec![
            ScoredChunk {
                doc_id: "a".into(),
                version_id: 1,
                page: 1,
                text: "unrelated".into(),
                score: 0.0,
            },
            ScoredChunk {
                doc_id: "b".into(),
                version_id: 1,
                page: 2,
                text: "hak milik tanah".into(),
                score: 0.0,
            },
        ];
        let r = rank("hak milik", &chunks, 1);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].doc_id, "b");
    }
}
