//! The annotate file, the contract between a VCS plugin and any plugin that shows who changed
//! what (the blame plugin; docs/plugins/vcs.md "The annotate file"): the per-line revisions
//! and the file's log in → `.schemify/vcs/annotate/<path>.tsv` out.
//!
//! Format, version 1: a header line `schemify-annotate 1 <hash> <vcs>`, `<hash>` [`hash`] of
//! the working file's bytes when the plugin ran (a reader whose file hashes otherwise ignores it as stale), then one line per line of the
//! file as the VCS annotates it: `<rev>\t<author>\t<unix time>\t<summary>`, the fields free
//! of tabs and line breaks. A line with no revision in the log (an uncommitted line) is
//! `\t\t0\tnot committed`.
//!
//! Contents: `pub const VERSION`, `pub const DIR`, `pub fn path`, `pub fn hash`, `pub fn join`,
//! `pub fn text`, `pub fn parse`

use std::fmt::Write as _;

use crate::{LogRow, field};

/// The format version in the header.
pub const VERSION: u32 = 1;

/// Where the files live, project-relative (inside `.schemify/`, which projects keep out of
/// version control).
pub const DIR: &str = ".schemify/vcs/annotate";

/// The annotate file of project file `rel`.
///
/// # Panics
///
/// If `rel` is empty or absolute (a project path never is).
#[must_use]
pub fn path(rel: &str) -> String {
    assert!(
        !rel.is_empty() && !rel.starts_with('/'),
        "a project-relative path"
    );
    format!("{DIR}/{rel}.tsv")
}

/// A file's content hash as the header writes it: `schemify_format::fnv1a64` from
/// `FNV_OFFSET`, 16 lowercase hex digits.
///
/// # Panics
///
/// Never: a `u64` in hex is 16 digits.
#[must_use]
pub fn hash(bytes: &[u8]) -> String {
    let h = format!(
        "{:016x}",
        schemify_format::fnv1a64(schemify_format::FNV_OFFSET, bytes)
    );
    assert_eq!(h.len(), 16);
    h
}

/// One row per line: each line's revision looked up in `log`. A revision is found by equality
/// or as a prefix of the log's (a short hash against a full one); one missing is the
/// uncommitted row. Cost: O(L log R) for L lines and R log rows.
///
/// # Panics
///
/// Never: the assert restates one row per revision.
#[must_use]
pub fn join(revs: &[String], log: &[LogRow]) -> Vec<LogRow> {
    let mut by_rev: Vec<&LogRow> = log.iter().collect();
    by_rev.sort_by(|a, b| a.rev.cmp(&b.rev));
    let find = |rev: &str| -> Option<&LogRow> {
        if rev.is_empty() {
            return None;
        }
        let at = by_rev.partition_point(|r| r.rev.as_str() < rev);
        by_rev.get(at).copied().filter(|r| r.rev.starts_with(rev))
    };
    let rows: Vec<LogRow> = (revs.iter())
        .map(|rev| {
            find(rev).cloned().unwrap_or_else(|| LogRow {
                summary: "not committed".to_owned(),
                ..LogRow::default()
            })
        })
        .collect();
    assert_eq!(rows.len(), revs.len(), "one row per line");
    rows
}

/// The file's text: the header for working-file bytes `work` and VCS `vcs`, then `rows`.
///
/// # Panics
///
/// Never: fields lose their tabs and line breaks, so each row is one line.
#[must_use]
pub fn text(work: &[u8], vcs: &str, rows: &[LogRow]) -> String {
    let hash = hash(work);
    let mut t = String::with_capacity(64 + rows.len() * 80);
    let _ = writeln!(t, "schemify-annotate {VERSION} {hash} {vcs}");
    for r in rows {
        let (rev, author, summary) = (field(&r.rev), field(&r.author), field(&r.summary));
        let _ = writeln!(t, "{rev}\t{author}\t{}\t{summary}", r.time);
    }
    assert_eq!(
        t.lines().count(),
        rows.len() + 1,
        "a header and one line per row"
    );
    t
}

/// Reads a file [`text`] wrote, when it is version [`VERSION`] and was made from bytes that
/// hash as `work` does: the rows, else `None` (stale, another version, malformed).
#[must_use]
pub fn parse(file: &str, work: &[u8]) -> Option<Vec<LogRow>> {
    let mut lines = file.lines();
    let mut head = lines.next()?.split(' ');
    let fresh = head.next() == Some("schemify-annotate")
        && head.next() == Some(&VERSION.to_string())
        && head.next() == Some(&hash(work));
    if !fresh {
        return None;
    }
    lines
        .map(|l| {
            let mut f = l.splitn(4, '\t');
            let (rev, author) = (f.next()?.to_owned(), f.next()?.to_owned());
            let time = f.next()?.parse().ok()?;
            Some(LogRow {
                rev,
                author,
                time,
                summary: f.next()?.to_owned(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(rev: &str, author: &str, time: i64, summary: &str) -> LogRow {
        LogRow {
            rev: rev.into(),
            author: author.into(),
            time,
            summary: summary.into(),
        }
    }

    /// Round trip (the oracle): what `text` writes `parse` reads back for the same bytes, and
    /// a changed working file makes it stale; short revisions find their log row.
    #[test]
    fn join_write_and_read_back() {
        let log = [
            row("abc123", "Ann", 10, "first"),
            row("def456", "Bo\tb", 20, "two\nlines"),
        ];
        let revs = ["def4", "abc123", "", "zzz"].map(String::from);
        let rows = join(&revs, &log);
        assert_eq!(rows[0].author, "Bo\tb");
        assert_eq!(rows[1].summary, "first");
        assert_eq!(rows[2].summary, "not committed");
        assert_eq!(rows[3].rev, "", "a revision the log lacks is uncommitted");
        let t = text(b"cell", "git", &rows);
        let back = parse(&t, b"cell").expect("fresh");
        assert_eq!(
            back[0],
            row("def456", "Bo b", 20, "two lines"),
            "fields lose tabs"
        );
        assert_eq!(back.len(), 4);
        assert_eq!(parse(&t, b"cell changed"), None, "stale");
        assert_eq!(path("a/b.shcomp"), ".schemify/vcs/annotate/a/b.shcomp.tsv");
    }
}
