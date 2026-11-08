//! Mercurial (`hg help status`, `hg help templates`, `hg help annotate`): `status` prints
//! `<code> <path>` per file, paths relative to the current directory when a pattern is
//! given; `log` and `annotate` print what a `-T` template says (`{date|hgdate}` is
//! `<unix time> <offset>`; an annotate template iterates `{lines}`).
//!
//! Contents: `pub const HG`

use crate::pane::ROWS_MAX;
use crate::{Backend, LogRow, lines};

/// Mercurial.
pub const HG: Backend = Backend {
    name: "hg",
    title: "Mercurial",
    program: "hg",
    root: &["hg", "root"],
    parse_root: |_| Some(String::new()),
    none_note: "Not a Mercurial repository: `hg init` to track changes",
    status: &["hg", "status", "--color=never", "."],
    parse_status,
    base: &["hg", "cat", "-r", ".", "{rel}"],
    diff: &["hg", "diff", "--color=never", "{rel}"],
    show: &["hg", "cat", "-r", "{rev}", "{rel}"],
    log: &[
        "hg",
        "log",
        "-T",
        "{node}\\t{author|person}\\t{date|hgdate}\\t{desc|firstline}\\n",
        "{rel}",
    ],
    parse_log,
    annotate: &["hg", "annotate", "-T", "{lines % \"{node}\\n\"}", "{rel}"],
    parse_annotate: |out| lines(out).map(|l| l.trim().to_owned()).collect(),
};

/// `status` lines `<code> <path>`: `M` modified, `A` added, `R` removed and `!` missing as
/// `D`, `?` unknown as `U`; clean and ignored files are not listed by default.
fn parse_status(out: &[u8], _prefix: &str) -> Vec<(char, String)> {
    let rows: Vec<(char, String)> = lines(out)
        .filter_map(|l| {
            let (code, path) = l.split_once(' ')?;
            let mark = match code {
                "M" => 'M',
                "A" => 'A',
                "R" | "!" => 'D',
                "?" => 'U',
                _ => return None,
            };
            (!path.is_empty()).then(|| (mark, path.to_owned()))
        })
        .take(ROWS_MAX)
        .collect();
    assert!(rows.len() <= ROWS_MAX);
    rows
}

/// The log template's lines: node, person, `hgdate` (time and offset), first line.
fn parse_log(out: &[u8]) -> Vec<LogRow> {
    lines(out)
        .filter_map(|l| {
            let mut f = l.splitn(4, '\t');
            let (rev, author) = (f.next()?.to_owned(), f.next()?.to_owned());
            let time = f.next()?.split(' ').next()?.parse().ok()?;
            Some(LogRow {
                rev,
                author,
                time,
                summary: f.next().unwrap_or("").to_owned(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_codes_map_to_markers() {
        let out =
            b"M ota.shcomp\nA new.shcomp\nR gone.shcomp\n! lost.lua\n? notes/a b.txt\nC clean\n";
        let rows = parse_status(out, "");
        let want = [
            ('M', "ota.shcomp"),
            ('A', "new.shcomp"),
            ('D', "gone.shcomp"),
        ];
        assert_eq!(rows[..3], want.map(|(m, p)| (m, p.to_owned())));
        assert_eq!(
            rows[3..],
            [
                ('D', "lost.lua".to_owned()),
                ('U', "notes/a b.txt".to_owned())
            ]
        );
    }

    #[test]
    fn log_and_annotate_templates_parse() {
        let log = parse_log(b"9f3c\tAnn Lee\t1157407993 25200\tfirst cut\n");
        assert_eq!(log[0].time, 1_157_407_993, "hgdate's first word");
        assert_eq!(
            (log[0].rev.as_str(), log[0].summary.as_str()),
            ("9f3c", "first cut")
        );
        assert_eq!(
            (HG.parse_annotate)(b"9f3c\n9f3c\nab12\n"),
            ["9f3c", "9f3c", "ab12"]
        );
    }
}
