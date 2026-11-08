//! git (git-scm.com/docs): `status --porcelain=v1 -z` (git-status "Porcelain Format Version
//! 1"), `log --format` placeholders (git-log "PRETTY FORMATS"), `blame --porcelain`
//! (git-blame "THE PORCELAIN FORMAT"). The real-git app tests run this row end to end.
//!
//! Contents: `pub const GIT`

use crate::pane::ROWS_MAX;
use crate::{Backend, LogRow, lines};

/// git.
pub const GIT: Backend = Backend {
    name: "git",
    title: "Git",
    program: "git",
    root: &["git", "rev-parse", "--show-prefix"],
    parse_root,
    none_note: "Not a git repository: `git init` to track changes",
    status: &[
        "git",
        "status",
        "--porcelain=v1",
        "-z",
        "--untracked-files=all",
        ".",
    ],
    parse_status,
    base: &["git", "show", "HEAD:{repo}"],
    diff: &["git", "diff", "HEAD", "--", "{rel}"],
    show: &["git", "show", "{rev}:{repo}"],
    log: &[
        "git",
        "log",
        "--format=%H%x09%an%x09%at%x09%s",
        "--",
        "{rel}",
    ],
    parse_log,
    annotate: &["git", "blame", "--porcelain", "--", "{rel}"],
    parse_annotate,
};

/// `rev-parse --show-prefix`: the project root inside the repository, `/`-terminated.
fn parse_root(out: &[u8]) -> Option<String> {
    Some(String::from_utf8_lossy(out).trim().to_owned())
}

/// The rows of `status --porcelain=v1 -z` under `prefix`, the prefix stripped: `M` modified,
/// `A` added, `D` deleted, `R` renamed, `U` untracked; paths outside the project left out.
fn parse_status(out: &[u8], prefix: &str) -> Vec<(char, String)> {
    let mut rows = Vec::new();
    let mut fields = out.split(|&b| b == 0).filter(|f| f.len() > 3);
    while let Some(f) = fields.next() {
        let (xy, path) = (&f[..2], String::from_utf8_lossy(&f[3..]));
        let mark = match xy {
            b"??" => 'U',
            [b'R', _] | [_, b'R'] => {
                let _ = fields.next(); // the rename's source path
                'R'
            }
            [b'A', _] => 'A',
            [b'D', _] | [_, b'D'] => 'D',
            _ => 'M',
        };
        let Some(rel) = path.strip_prefix(prefix) else {
            continue;
        };
        if rows.len() == ROWS_MAX {
            break;
        }
        rows.push((mark, rel.to_owned()));
    }
    assert!(rows.len() <= ROWS_MAX);
    rows
}

/// `log --format=%H%x09%an%x09%at%x09%s`: hash, author, unix time, subject per line.
fn parse_log(out: &[u8]) -> Vec<LogRow> {
    lines(out)
        .filter_map(|l| {
            let mut f = l.splitn(4, '\t');
            let (rev, author) = (f.next()?.to_owned(), f.next()?.to_owned());
            let time = f.next()?.parse().ok()?;
            Some(LogRow {
                rev,
                author,
                time,
                summary: f.next().unwrap_or("").to_owned(),
            })
        })
        .collect()
}

/// `blame --porcelain`: each line of the file is a header `<40-hex> <orig> <final> [<n>]`,
/// optional `key value` lines, then the line itself after a TAB; the revision per line is
/// the header's hash (all zeros for an uncommitted line, which the log never holds).
fn parse_annotate(out: &[u8]) -> Vec<String> {
    let mut revs = Vec::new();
    let mut current = String::new();
    for l in out.split(|&b| b == b'\n') {
        if l.starts_with(b"\t") {
            revs.push(current.clone());
        } else if l.len() > 40 && l[..40].iter().all(u8::is_ascii_hexdigit) && l[40] == b' ' {
            current = String::from_utf8_lossy(&l[..40]).into_owned();
        }
    }
    revs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn porcelain_rows_map_to_markers_under_the_prefix() {
        let out = b" M p/ota.shcomp\0?? p/lua/a.lua\0A  p/new.shcomp\0 D p/old.shcomp\0R  p/b.lua\0p/a.lua\0M  other/x\0";
        let rows = parse_status(out, "p/");
        let want = [('M', "ota.shcomp"), ('U', "lua/a.lua"), ('A', "new.shcomp")];
        assert_eq!(rows[..3], want.map(|(m, p)| (m, p.to_owned())));
        assert_eq!(
            rows[3..],
            [('D', "old.shcomp".to_owned()), ('R', "b.lua".to_owned())]
        );
        assert!(parse_status(b"", "").is_empty());
    }

    /// git-blame's documented porcelain sample: a group header carries the commit's keys
    /// once; later lines of the same commit repeat only the header.
    #[test]
    fn blame_porcelain_gives_one_revision_per_line() {
        let a = "a".repeat(40);
        let z = "0".repeat(40);
        let out = format!(
            "{a} 1 1 2\nauthor Ann\nauthor-time 10\nsummary first\nfilename x\n\tline one\n\
             {a} 2 2\n\tline two\n{z} 3 3 1\nauthor Not Committed Yet\nfilename x\n\tnew\n"
        );
        assert_eq!(parse_annotate(out.as_bytes()), [a.clone(), a, z]);
        let log = parse_log(b"abc\tAnn Lee\t1700000000\tfix: the bias\nbad line\n");
        assert_eq!(
            log,
            [LogRow {
                rev: "abc".into(),
                author: "Ann Lee".into(),
                time: 1_700_000_000,
                summary: "fix: the bias".into(),
            }]
        );
    }
}
