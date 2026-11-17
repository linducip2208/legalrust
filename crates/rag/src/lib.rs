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
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chunks_overlap() {
        let c = chunk_text("abcdefghij", 4, 2);
        assert_eq!(c, vec!["abcd", "cdef", "efgh", "ghij", "ij"]);
    }
}
