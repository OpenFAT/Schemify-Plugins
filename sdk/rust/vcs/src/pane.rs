//! The Source control pane and the Explorer's marks: a backend's status rows in → the `ui`
//! set and the `explorer` annotation set out (plugin-abi §7). Every VCS plugin draws the same
//! pane, so a person switching VCS sees no new UI.
//!
//! Contents: `pub const ROWS_MAX`, `pub fn pane`, `pub fn marks`

use std::fmt::Write as _;

use schemify_abi::text::push_value;

use crate::Backend;

/// Changed paths listed at most; the rest is left out.
pub const ROWS_MAX: usize = 1024;

/// The pane's place in the default UI's icon bar (`activity.*` orders: Explorer 10, Search 20,
/// Packages 40, …): Source control's slot, so any VCS plugin's icon sits there.
const ICON_ORDER: i32 = 30;

/// The icon bar button's 21 px glyph (the mockup's branch icon).
const ICON: &str = concat!(
    "M5.25 1 C6.22 1 7 1.78 7 2.75 C7 3.72 6.22 4.5 5.25 4.5 C4.28 4.5 3.5 3.72 3.5 2.75 ",
    "C3.5 1.78 4.28 1 5.25 1 M5.25 11.5 C6.22 11.5 7 12.28 7 13.25 C7 14.22 6.22 15 5.25 15 ",
    "C4.28 15 3.5 14.22 3.5 13.25 C3.5 12.28 4.28 11.5 5.25 11.5 M15.75 3.62 C16.72 3.62 17.5 ",
    "4.41 17.5 5.38 C17.5 6.34 16.72 7.12 15.75 7.12 C14.78 7.12 14 6.34 14 5.38 C14 4.41 ",
    "14.78 3.62 15.75 3.62 M5.25 4.5 L5.25 11.5 M15.75 7.12 C15.75 10.62 10.5 9.75 6.12 11.5",
);

/// The `ui` set: the pane `scm` (left dock; git's titled `Source control`, another VCS's
/// `Source control: <title>`), its icon-bar button running `<name>.show`, and the content node
/// `scm.pane`: a head (`CHANGES  <n>`, Refresh running `<name>.refresh`), the note when set,
/// then one row per change (`scm.row.<k>` running `<name>.diff.<k>`, its marker in
/// `scm.mark.<k>`).
///
/// # Panics
///
/// If `rows` holds more than [`ROWS_MAX`] (the parsers cap it) or the row's name is not one
/// word.
#[must_use]
pub fn pane(b: &Backend, rows: &[(char, String)], note: &str) -> String {
    assert!(rows.len() <= ROWS_MAX);
    assert!(!b.name.contains(['.', ' ']), "a backend name is one word");
    let (name, title) = (b.name, b.title);
    let mut t = String::with_capacity(1024 + rows.len() * 256);
    t.push_str("pane scm ");
    match name {
        "git" => push_value(&mut t, "Source control"), // as before the plugins
        _ => push_value(&mut t, &format!("Source control: {title}")),
    }
    t.push_str(" dock left size 240\n");
    let _ = writeln!(
        t,
        "node activity.scm button\n  parent activity\n  order {ICON_ORDER}"
    );
    let _ = write!(
        t,
        "  label \"Source control\"\n  command {name}.show\n  tooltip "
    );
    push_value(
        &mut t,
        &format!("Source control: the project's changed files ({title})"),
    );
    let _ = writeln!(t, "\n  icon \"{ICON}\"\n  style activity-button");
    t.push_str("node scm.pane panel\n  style dock-pane\n");
    t.push_str("node scm.head panel\n  parent scm.pane\n  style tree-line\n");
    t.push_str("node scm.count label\n  parent scm.head\n  label ");
    push_value(&mut t, &format!("CHANGES  {}", rows.len()));
    t.push_str("\n  style tree-section\n");
    t.push_str("node scm.head.spacer panel\n  parent scm.head\n  style spacer\n");
    t.push_str("node scm.refresh button\n  parent scm.head\n  label Refresh\n");
    let _ = write!(t, "  command {name}.refresh\n  tooltip ");
    push_value(&mut t, &format!("Read the {title} status again"));
    t.push_str("\n  style dock-button\n");
    t.push_str("node scm.list scroll\n  parent scm.pane\n  style dialog-list-scroll\n");
    if !note.is_empty() {
        t.push_str("node scm.note label\n  parent scm.list\n  label ");
        push_value(&mut t, note);
        t.push_str("\n  style dock-note\n");
    }
    for (k, (mark, rel)) in rows.iter().enumerate() {
        let _ = writeln!(
            t,
            "node scm.line.{k} panel\n  parent scm.list\n  style tree-line"
        );
        let _ = write!(
            t,
            "node scm.row.{k} button\n  parent scm.line.{k}\n  label "
        );
        push_value(&mut t, &format!("  {rel}"));
        let _ = write!(t, "\n  command {name}.diff.{k}\n  tooltip ");
        push_value(
            &mut t,
            &format!("Show the diff of {rel} against its last revision"),
        );
        t.push_str("\n  style tree-fill\n");
        let _ = writeln!(
            t,
            "node scm.mark.{k} label\n  parent scm.line.{k}\n  label {mark}"
        );
        t.push_str("  style scm-mark\n");
    }
    assert!(t.ends_with('\n'));
    t
}

/// The `explorer` annotation set: one `mark <path> info <marker>` per row (plugin-abi §7), so
/// the Explorer shows the marker after the file's row.
///
/// # Panics
///
/// Never: a quoted path holds no line break.
#[must_use]
pub fn marks(rows: &[(char, String)]) -> String {
    let mut t = String::with_capacity(rows.len() * 64);
    for (mark, rel) in rows {
        t.push_str("mark ");
        push_value(&mut t, rel);
        let _ = writeln!(t, " info {mark}");
    }
    assert_eq!(t.lines().count(), rows.len(), "one mark per row");
    t
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::{GIT, HG};

    /// Oracle: the records parse as a `ui` set would (record word first, attributes
    /// indented), name the backend's commands, and quote a path with spaces.
    #[test]
    fn the_pane_names_the_backend_and_quotes_paths() {
        let rows = [('M', "a b.lua".to_owned()), ('U', "c".to_owned())];
        assert_eq!(marks(&rows), "mark \"a b.lua\" info M\nmark c info U\n");
        let p = pane(&HG, &rows, "");
        assert!(
            p.starts_with("pane scm \"Source control: Mercurial\" dock left"),
            "{p}"
        );
        assert!(
            p.contains("command hg.diff.1\n") && p.contains("command hg.show\n"),
            "{p}"
        );
        assert!(p.contains("label \"  a b.lua\"\n"), "{p}");
        assert!(p.contains("label \"CHANGES  2\""), "{p}");
        assert!(pane(&GIT, &[], "x").starts_with("pane scm \"Source control\" dock"));
        for line in p.lines() {
            let word = line.split(' ').next().unwrap_or("");
            assert!(
                ["pane", "node", ""].contains(&word),
                "a record or an attribute: {line:?}"
            );
        }
    }
}
