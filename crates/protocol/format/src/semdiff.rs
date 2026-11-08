//! The semantic diff as text: two parsed files → one line per changed record (spec §8
//! "Diff"). `schemify diff` prints it; a plugin (a version-control plugin's Source control
//! pane) links it to show a design file's change against a revision.
//!
//! Contents: `pub fn diff_text`

use crate::diff::{ChangeKind, Parts, diff};
use crate::document::{Document, NONE};

/// The semantic diff of `a` (old) and `b` (new): per change `+`, `-` or `~`, the record's
/// word and key, and for a changed record the parts that differ (`[fields]`, `[text]`,
/// `[comments]`, `[attrs: k1, k2]`); empty when they are equal. Cost: one [`diff`].
///
/// # Examples
///
/// ```
/// use schemify_format as format;
///
/// let cell = "schemify 1 shcomp\n\nimpl sch schematic\n\ninst m1 dev:nfet\n  at 0 0 R0\n  name M1\n";
/// let (old, new) = (format!("{cell}  W 4u\n"), format!("{cell}  W 6u\n"));
/// let (a, b) = (format::parse(old.as_bytes())?, format::parse(new.as_bytes())?);
/// assert_eq!(format::diff_text(&a, &a), "");
/// assert!(format::diff_text(&a, &b).starts_with("~ inst m1"));
/// assert!(format::diff_text(&a, &b).ends_with("[attrs: W]\n"));
/// # Ok::<(), format::ParseError>(())
/// ```
///
/// # Panics
///
/// If [`diff`] breaks its contract (a change without its record, a changed record
/// with no differing part).
#[must_use]
pub fn diff_text(a: &Document<'_>, b: &Document<'_>) -> String {
    let d = diff(a, b);
    let mut out = String::new();
    for c in &d.changes {
        let (sign, doc, r) = match c.kind {
            ChangeKind::Added => ('+', b, c.b),
            ChangeKind::Removed => ('-', a, c.a),
            ChangeKind::Changed => ('~', b, c.b),
        };
        assert_ne!(r, NONE, "{sign} change without its record");
        if c.kind == ChangeKind::Changed {
            assert_ne!(
                c.parts,
                Parts::default(),
                "a changed record differs somewhere"
            );
        } else {
            assert_eq!(c.parts, Parts::default(), "only a changed record has parts");
            assert!(c.keys.is_empty(), "only a changed record has keys");
        }
        let (k1, k2) = doc.record_key(r);
        out.push(sign);
        out.push(' ');
        out.push_str(doc.record_word(r));
        for k in [k1, k2].into_iter().filter(|k| !k.is_empty()) {
            out.push(' ');
            out.push_str(k);
        }
        if c.kind == ChangeKind::Changed {
            assert!(c.keys.start <= c.keys.end);
            assert!(
                c.keys.end as usize <= d.keys.len(),
                "a change's keys lie in the diff's key table"
            );
            push_changed_parts(
                &mut out,
                c.parts,
                &d.keys[c.keys.start as usize..c.keys.end as usize],
            );
        }
        out.push('\n');
    }
    assert_eq!(out.is_empty(), d.changes.is_empty(), "one line per change");
    out
}

// ---- private ----

/// Appends ` [fields] [text] [comments] [attrs: k1, k2]` for the parts a changed record
/// differs in.
fn push_changed_parts(out: &mut String, parts: Parts, keys: &[&str]) {
    if !keys.is_empty() {
        assert!(
            parts.contains(Parts::ATTRS),
            "attribute keys differ only with ATTRS"
        );
    }
    let names = [
        (Parts::FIELDS, "fields"),
        (Parts::VERBATIM, "text"),
        (Parts::COMMENTS, "comments"),
    ];
    let differing = names.into_iter().filter(|(p, _)| parts.contains(*p));
    for (_, name) in differing {
        out.push_str(" [");
        out.push_str(name);
        out.push(']');
    }
    if !keys.is_empty() {
        out.push_str(" [attrs: ");
        out.push_str(&keys.join(", "));
        out.push(']');
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn equal_files_have_no_lines_and_a_changed_attr_has_one() {
        let a = b"schemify 1 shcomp\n\nimpl sch schematic\n\ninst m1 dev:nfet\n  at 0 0 R0\n  name M1\n  W 4u\n";
        let b = b"schemify 1 shcomp\n\nimpl sch schematic\n\ninst m1 dev:nfet\n  at 0 0 R0\n  name M1\n  W 6u\n";
        let (da, db) = (
            crate::parse(a).expect("parses"),
            crate::parse(b).expect("parses"),
        );
        assert_eq!(super::diff_text(&da, &da), "");
        let t = super::diff_text(&da, &db);
        assert_eq!(t.lines().count(), 1, "{t}");
        assert!(t.starts_with("~ inst m1"), "{t}");
        assert!(t.contains("[attrs: W]"), "{t}");
    }
}
