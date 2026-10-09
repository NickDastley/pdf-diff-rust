//! Stable, deterministic identifiers and hashing utilities.

use sha1::{Digest, Sha1};

/// Lowercase hexadecimal SHA-1 of the UTF-8 bytes of `data`.
pub fn sha1_hex(data: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(data.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Round half to even ("banker's rounding"): `0.5 -> 0`, `1.5 -> 2`,
/// `2.5 -> 2`, `-0.5 -> 0`.
pub fn round_half_to_even(value: f64) -> i64 {
    value.round_ties_even() as i64
}

/// Build the deterministic element id from its identifying attributes:
///
/// ```text
/// sha1("{type}|{page}|{round(x0)}|{round(y0)}|{round(width)}|{round(height)}|{tok}|{hash[:10]}")
/// ```
///
/// `tok` is `-1` when no token count is supplied, and `hash[:10]` is the first
/// ten characters of the normalised-text hash (clamped for short inputs).
pub fn build_element_id(
    element_type: &str,
    page: i32,
    bbox: [f64; 4],
    normalized_text_hash: &str,
    token_count: Option<i32>,
) -> String {
    let [x0, y0, x1, y1] = bbox;
    let width = x1 - x0;
    let height = y1 - y0;
    let token_count = token_count.unwrap_or(-1);
    let hash_prefix = &normalized_text_hash[..normalized_text_hash.len().min(10)];

    let base = format!(
        "{element_type}|{page}|{}|{}|{}|{}|{token_count}|{hash_prefix}",
        round_half_to_even(x0),
        round_half_to_even(y0),
        round_half_to_even(width),
        round_half_to_even(height),
    );

    sha1_hex(&base)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha1_matches_known_vector() {
        assert_eq!(sha1_hex("abc"), "a9993e364706816aba3e25717850c26c9cd0d89d");
    }

    #[test]
    fn round_half_to_even_is_correct() {
        assert_eq!(round_half_to_even(2.4), 2);
        assert_eq!(round_half_to_even(2.6), 3);
        assert_eq!(round_half_to_even(0.5), 0);
        assert_eq!(round_half_to_even(1.5), 2);
        assert_eq!(round_half_to_even(2.5), 2);
        assert_eq!(round_half_to_even(-0.5), 0);
        assert_eq!(round_half_to_even(-1.5), -2);
        assert_eq!(round_half_to_even(-0.0), 0);
    }

    #[test]
    fn element_id_is_stable_for_known_input() {
        let hash = sha1_hex("Hello");
        assert_eq!(
            build_element_id("text_block", 0, [10.2, 10.5, 100.5, 40.0], &hash, None),
            "dc7efc45ce08ba79e1aa931cddc9c73c89d787bb",
        );
    }

    #[test]
    fn element_id_is_deterministic() {
        let hash = sha1_hex("Hello");
        let a = build_element_id("text_block", 1, [0.0, 0.0, 1.0, 1.0], &hash, Some(2));
        let b = build_element_id("text_block", 1, [0.0, 0.0, 1.0, 1.0], &hash, Some(2));
        assert_eq!(a, b);
    }
}
