//! Differential tests of `abi::text` against `schemify-format`, the reader every Schemify
//! file goes through (format spec §2). `abi` keeps its own copy of the codec so guests link
//! no other crate (the ponytail note in `src/text.rs`); these properties keep the copy
//! equal to the original: the same word rule, and every value `push_value` writes reads
//! back through the format reader as the value written. The decoding direction (abi
//! `values` against the format reader) is tested in the host.
//!
//! Contents: tests only.

use proptest::prelude::*;
use schemify_abi::text;

/// Header of a forward-compatible lockfile: every record is an unknown record whose fields
/// the format reader unescapes (what the plugin host lexes contribution sets as).
const LEX_HEADER: &str = "schemify 65535 lock\n";

/// Strings mixing words, specials, controls and non-ASCII.
fn text_strategy() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9_.:\\- \t\"\\\\#|{}\u{0}-\u{1f}\u{7f}éß\u{1F600}]{0,12}"
}

proptest! {
    /// Oracle: `schemify_format::is_word`, the reader's own rule.
    #[test]
    fn is_word_agrees_with_format(s in text_strategy()) {
        prop_assert_eq!(text::is_word(&s), schemify_format::is_word(&s));
    }

    /// Oracle: the format reader decodes what `abi::text::push_value` encodes (round trip
    /// through the independent implementation); a bare value is written as itself.
    #[test]
    fn push_value_reads_back_through_format(value in text_strategy()) {
        let mut line = String::from(LEX_HEADER);
        line.push_str("argv ");
        text::push_value(&mut line, &value);
        line.push('\n');
        let doc = schemify_format::parse(line.as_bytes());
        prop_assert!(doc.is_ok(), "{line:?}");
        let doc = doc.expect("checked above");
        prop_assert_eq!(doc.record_count(), 2);
        prop_assert_eq!(doc.field(1, 0), Some(value.as_str()));
        prop_assert_eq!(doc.field(1, 1), None, "one value");
        let written = &line[LEX_HEADER.len() + "argv ".len()..line.len() - 1];
        prop_assert_eq!(text::is_bare(&value), written == value);
    }
}
