//! Unicode-aware text normalisation.

use unicode_normalization::UnicodeNormalization;

/// Normalise text into the canonical form used for matching:
///
/// 1. Unicode NFC normalisation (composed form),
/// 2. remove soft hyphens (`U+00AD`) and zero-width spaces (`U+200B`),
/// 3. collapse runs of whitespace to a single space and trim the ends.
///
/// Takes a borrow of the input and returns a freshly allocated `String`.
pub fn normalize_text(text: &str) -> String {
    let composed: String = text.nfc().collect();
    composed
        .replace(['\u{00ad}', '\u{200b}'], "")
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapses_and_trims_whitespace() {
        assert_eq!(normalize_text("  Hello\n\tWorld  "), "Hello World");
    }

    #[test]
    fn removes_soft_hyphen_and_zero_width() {
        assert_eq!(normalize_text("co\u{00ad}operate"), "cooperate");
        assert_eq!(normalize_text("a\u{200b}b"), "ab");
    }

    #[test]
    fn applies_nfc_composition() {
        // "e" + combining acute accent (U+0301) becomes the single "é" (U+00E9).
        assert_eq!(normalize_text("e\u{0301}"), "\u{00e9}");
    }

    #[test]
    fn empty_and_whitespace_only_become_empty() {
        assert_eq!(normalize_text(""), "");
        assert_eq!(normalize_text("   \n\t "), "");
    }
}
