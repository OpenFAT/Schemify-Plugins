//! Jujutsu (jj-vcs.github.io/jj, CLI reference and "Templates"): `diff --summary` prints
//! `<M|A|D|R|C> <path>` per file of the working-copy commit, a rename as
//! `R <dir>/{<old> => <new>}`; `log` and `file annotate` print what a `-T` template says.
//! There are no untracked files (jj snapshots every file).
//!
//! Contents: `pub const JJ`

use crate::pane::ROWS_MAX;
use crate::{Backend, LogRow, lines};

/// Jujutsu.
pub const JJ: Backend = Backend {
    name: "jj",
    title: "Jujutsu",
    program: "jj",
    root: &["jj", "root"],
    parse_root: |_| Some(String::new()),
    none_note: "Not a Jujutsu repository: `jj git init` to track changes",
    status: &["jj", "diff", "--summary", "--color=never"],
    parse_status,
    base: &["jj", "file", "show", "-r", "@-", "{rel}"],
    diff: &["jj", "diff", "--git", "--color=never", "{rel}"],
    show: &["jj", "file", "show", "-r", "{rev}", "{rel}"],
    log: &[
        "jj",
        "log",
        "--no-graph",
        "--color=never",
        "-r",
        "::@",
        "-T",
        "commit_id ++ \"\\t\" ++ author.name() ++ \"\\t\" ++ author.timestamp().format(\"%s\") \
         ++ \"\\t\" ++ description.first_line() ++ \"\\n\"",
        "{rel}",
    ],
    parse_log,
    annotate: &[
        "jj",
        "file",
        "annotate",
        "-T",
        "commit.commit_id() ++ \"\\n\"",
        "{rel}",
    ],
    parse_annotate: |out| lines(out).map(|l| l.trim().to_owned()).collect(),
};

/// `diff --summary` lines; a rename's new path is rebuilt from `{<old> => <new>}`.
fn parse_status(out: &[u8], _prefix: &str) -> Vec<(char, String)> {
    let rows: Vec<(char, String)> = lines(out)
        .filter_map(|l| {
            let (code, path) = l.split_once(' ')?;
            let mark = match code {
                "M" => 'M',
                "A" | "C" => 'A',
                "D" => 'D',
                "R" => 'R',
                _ => return None,
            };
            // Paths are relative to the project root (the current directory): `../` is outside.
            let path = renamed_to(path);
            (!path.starts_with("../")).then_some((mark, path))
        })
        .take(ROWS_MAX)
        .collect();
    assert!(rows.len() <= ROWS_MAX);
    rows
}

/// `a/{b => c}/d` → `a/c/d`; a path without braces stays.
fn renamed_to(path: &str) -> String {
    let (Some(open), Some(close)) = (path.find('{'), path.rfind('}')) else {
        return path.to_owned();
    };
    let inner = &path[open + 1..close];
    let new = inner.split_once(" => ").map_or(inner, |(_, n)| n);
    let joined = format!("{}{new}{}", &path[..open], &path[close + 1..]);
    joined.replace("//", "/")
}

/// The log template's lines: commit id, author, unix time, first line.
fn parse_log(out: &[u8]) -> Vec<LogRow> {
    lines(out)
        .filter_map(|l| {
            let mut f = l.splitn(4, '\t');
            let (rev, author) = (f.next()?.to_owned(), f.next()?.to_owned());
            let time = f.next()?.trim().parse().ok()?;
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
    fn summary_lines_and_renames() {
        let out = b"M ota.shcomp\nA new.shcomp\nD old.lua\nR cells/{a.shcomp => b.shcomp}\nR {x => y}/z.lua\n";
        let rows = parse_status(out, "");
        let want = [
            ('M', "ota.shcomp"),
            ('A', "new.shcomp"),
            ('D', "old.lua"),
            ('R', "cells/b.shcomp"),
            ('R', "y/z.lua"),
        ];
        assert_eq!(rows, want.map(|(m, p)| (m, p.to_owned())));
        assert_eq!(
            renamed_to("a/{b => }/c"),
            "a/c",
            "a folder collapsed into its parent"
        );
    }

    #[test]
    fn log_and_annotate_templates_parse() {
        let log = parse_log(b"e1f2\tAnn Lee\t1700000000\tsize the tail\n");
        assert_eq!((log[0].rev.as_str(), log[0].time), ("e1f2", 1_700_000_000));
        assert_eq!((JJ.parse_annotate)(b"e1f2\ne1f2\n"), ["e1f2", "e1f2"]);
    }
}
