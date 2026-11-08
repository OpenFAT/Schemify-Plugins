//! Subversion (svnbook.red-bean.com, "svn Reference"): `status` prints seven status columns
//! and the path from column 9 (`A` added, `D` deleted, `M` modified, `R` replaced, `C`
//! conflicted, `?` unversioned, `!` missing, `~` obstructed; column 2 `M` a property change);
//! `blame` prints `<rev> <author> <line>` right-aligned; `log` prints records
//! `r<rev> | <author> | <YYYY-MM-DD hh:mm:ss ±hhmm> (<day>) | <n> line(s)`, a blank line,
//! the message, a dashed separator.
//!
//! Contents: `pub const SVN`

use crate::pane::ROWS_MAX;
use crate::{Backend, LogRow, lines};

/// Subversion.
pub const SVN: Backend = Backend {
    name: "svn",
    title: "Subversion",
    program: "svn",
    root: &["svn", "info", "--non-interactive", "."],
    parse_root: |_| Some(String::new()),
    none_note: "Not a Subversion working copy: `svn checkout` one to track changes",
    status: &["svn", "status", "--non-interactive"],
    parse_status,
    base: &["svn", "cat", "--non-interactive", "-r", "BASE", "{rel}"],
    diff: &["svn", "diff", "--non-interactive", "{rel}"],
    show: &["svn", "cat", "--non-interactive", "-r", "{rev}", "{rel}"],
    log: &["svn", "log", "--non-interactive", "{rel}"],
    parse_log,
    annotate: &["svn", "blame", "--non-interactive", "{rel}"],
    parse_annotate: |out| {
        lines(out)
            .map(|l| l.split_whitespace().next().unwrap_or("").to_owned())
            .collect()
    },
};

/// `status` lines: column 1's code, else a property change in column 2, then the path.
fn parse_status(out: &[u8], _prefix: &str) -> Vec<(char, String)> {
    let rows: Vec<(char, String)> = lines(out)
        .filter_map(|l| {
            let b = l.as_bytes();
            if b.len() < 9 || !l.is_char_boundary(8) {
                return None;
            }
            let mark = match (b[0], b[1]) {
                (b'A', _) => 'A',
                (b'D' | b'!', _) => 'D',
                (b'M' | b'R' | b'C' | b'~', _) | (b' ', b'M' | b'C') => 'M',
                (b'?', _) => 'U',
                _ => return None,
            };
            Some((mark, l[8..].to_owned()))
        })
        .take(ROWS_MAX)
        .collect();
    assert!(rows.len() <= ROWS_MAX);
    rows
}

/// `log` records: the header line's revision (without `r`), author and date; the first
/// message line after the blank line.
fn parse_log(out: &[u8]) -> Vec<LogRow> {
    let mut rows: Vec<LogRow> = Vec::new();
    let mut want_summary = false;
    for l in lines(out) {
        if l.starts_with("-----") {
            want_summary = false;
            continue;
        }
        let head: Vec<&str> = l.split(" | ").collect();
        if head.len() == 4 && head[0].starts_with('r') && head[0][1..].parse::<u64>().is_ok() {
            rows.push(LogRow {
                rev: head[0][1..].to_owned(),
                author: head[1].to_owned(),
                time: unix_time(head[2]).unwrap_or(0),
                summary: String::new(),
            });
            want_summary = true;
        } else if want_summary && let Some(r) = rows.last_mut() {
            r.summary = l; // `lines` skipped the blank line before the message
            want_summary = false;
        }
    }
    rows
}

/// `YYYY-MM-DD hh:mm:ss ±hhmm (…)` → seconds since 1970 (UTC).
fn unix_time(date: &str) -> Option<i64> {
    let mut w = date.split(' ');
    let (day, clock, zone) = (w.next()?, w.next()?, w.next()?);
    let d: Vec<i64> = day
        .split('-')
        .map(str::parse)
        .collect::<Result<_, _>>()
        .ok()?;
    let c: Vec<i64> = clock
        .split(':')
        .map(str::parse)
        .collect::<Result<_, _>>()
        .ok()?;
    if d.len() != 3 || c.len() != 3 || zone.len() != 5 {
        return None;
    }
    let sign = if zone.starts_with('-') { -1 } else { 1 };
    let (zh, zm): (i64, i64) = (zone[1..3].parse().ok()?, zone[3..5].parse().ok()?);
    let local = days_from_civil(d[0], d[1], d[2]) * 86_400 + c[0] * 3600 + c[1] * 60 + c[2];
    Some(local - sign * (zh * 3600 + zm * 60))
}

/// Days from 1970-01-01 to the proleptic Gregorian date `y-m-d` (Howard Hinnant's
/// `days_from_civil`).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_columns_map_to_markers() {
        let out = b"M       ota.shcomp\nA  +    new.shcomp\nD       gone.lua\n?       notes/a b.txt\n M      props.lua\n!       lost.lua\nI       ignored\n";
        let rows = parse_status(out, "");
        let want = [
            ('M', "ota.shcomp"),
            ('A', "new.shcomp"),
            ('D', "gone.lua"),
            ('U', "notes/a b.txt"),
            ('M', "props.lua"),
            ('D', "lost.lua"),
        ];
        assert_eq!(rows, want.map(|(m, p)| (m, p.to_owned())));
    }

    /// The svnbook's `svn blame` and `svn log` samples.
    #[test]
    fn blame_and_log_records() {
        let blame = b"     3      sally This is a README file.\n     5      harry Don't bother.\n";
        assert_eq!((SVN.parse_annotate)(blame), ["3", "5"]);
        let log = b"------------------------------------------------------------------------\n\
r5 | harry | 2008-05-19 09:28:57 -0500 (Mon, 19 May 2008) | 1 line\n\
\n\
Added the README.\n\
------------------------------------------------------------------------\n\
r3 | sally | 1970-01-01 00:00:10 +0000 (Thu, 01 Jan 1970) | 2 lines\n\
\n\
First.\nSecond.\n\
------------------------------------------------------------------------\n";
        let rows = parse_log(log);
        assert_eq!(rows.len(), 2);
        assert_eq!(
            (rows[0].rev.as_str(), rows[0].author.as_str()),
            ("5", "harry")
        );
        assert_eq!(rows[0].summary, "Added the README.");
        assert_eq!(rows[0].time, 1_211_207_337, "2008-05-19 14:28:57 UTC");
        assert_eq!((rows[1].time, rows[1].summary.as_str()), (10, "First."));
    }
}
