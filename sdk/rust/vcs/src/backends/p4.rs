//! Perforce Helix Core (P4 Command Reference): `-ztag` prints each record as `... <field>
//! <value>` lines with a blank line between records (`info`'s `clientName`, `clientCwd`;
//! `fstat`'s `action`, `clientFile` in local syntax; `changes`' `change`, `time`, `user`,
//! `desc`); `annotate -c -q` prints `<change>: <line>` per line. Revisions are changelist
//! numbers; the working files are the opened ones (`fstat -Ro`), so a file not opened for
//! edit never shows.
//!
//! Contents: `pub const P4`

use crate::pane::ROWS_MAX;
use crate::{Backend, LogRow, lines};

/// Perforce.
pub const P4: Backend = Backend {
    name: "p4",
    title: "Perforce",
    program: "p4",
    root: &["p4", "-ztag", "info"],
    parse_root,
    none_note: "Not in a Perforce client workspace (set P4CONFIG or P4CLIENT)",
    status: &[
        "p4",
        "-ztag",
        "fstat",
        "-Ro",
        "-T",
        "action,clientFile",
        "./...",
    ],
    parse_status,
    base: &["p4", "print", "-q", "{rel}#have"],
    diff: &["p4", "diff", "{rel}"],
    show: &["p4", "print", "-q", "{rel}@{rev}"],
    log: &["p4", "-ztag", "changes", "-l", "-t", "{rel}"],
    parse_log,
    annotate: &["p4", "annotate", "-c", "-q", "{rel}"],
    parse_annotate: |out| {
        lines(out)
            .filter_map(|l| l.split_once(':').map(|(c, _)| c.trim().to_owned()))
            .collect()
    },
};

/// The `-ztag` records of `out`: per record its `(field, value)` pairs in order; a line
/// without the `... ` prefix continues the previous value (a multi-line `desc`).
fn records(out: &[u8]) -> Vec<Vec<(String, String)>> {
    let mut all: Vec<Vec<(String, String)>> = vec![Vec::new()];
    for l in String::from_utf8_lossy(out).lines() {
        let l = l.strip_suffix('\r').unwrap_or(l);
        if l.is_empty() {
            all.push(Vec::new());
        } else if let Some(tagged) = l.strip_prefix("... ") {
            let (k, v) = tagged.split_once(' ').unwrap_or((tagged, ""));
            all.last_mut()
                .expect("one record open")
                .push((k.to_owned(), v.to_owned()));
        }
    }
    all.retain(|r| !r.is_empty());
    all
}

/// The field `key` of `record`.
fn get<'r>(record: &'r [(String, String)], key: &str) -> Option<&'r str> {
    record
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
}

/// `info`: in a client workspace (`clientName` other than `*unknown*`), the current
/// directory with a trailing separator: the prefix of `fstat`'s local paths.
fn parse_root(out: &[u8]) -> Option<String> {
    let r = records(out).into_iter().next()?;
    let client = get(&r, "clientName")?;
    let cwd = get(&r, "clientCwd")?;
    (client != "*unknown*").then(|| format!("{}/", cwd.trim_end_matches(['/', '\\'])))
}

/// `fstat -Ro` records: `add`/`branch`/`move/add` added, `delete`/`move/delete` deleted,
/// any other open action modified; the local path with `prefix` stripped.
fn parse_status(out: &[u8], prefix: &str) -> Vec<(char, String)> {
    let rows: Vec<(char, String)> = records(out)
        .iter()
        .filter_map(|r| {
            let mark = match get(r, "action")? {
                "add" | "branch" | "move/add" | "import" => 'A',
                "delete" | "move/delete" | "purge" | "archive" => 'D',
                _ => 'M',
            };
            let rel = get(r, "clientFile")?
                .strip_prefix(prefix)?
                .replace('\\', "/");
            Some((mark, rel))
        })
        .take(ROWS_MAX)
        .collect();
    assert!(rows.len() <= ROWS_MAX);
    rows
}

/// `changes -l -t` records: change number, user, `time` (unix), the description's first line.
fn parse_log(out: &[u8]) -> Vec<LogRow> {
    records(out)
        .iter()
        .filter_map(|r| {
            Some(LogRow {
                rev: get(r, "change")?.to_owned(),
                author: get(r, "user").unwrap_or("").to_owned(),
                time: get(r, "time")?.parse().ok()?,
                summary: get(r, "desc").unwrap_or("").trim().to_owned(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn info_fstat_and_changes_records() {
        let info =
            b"... userName ann\n... clientName ann-ws\n... clientRoot /w\n... clientCwd /w/proj\n";
        assert_eq!(parse_root(info).as_deref(), Some("/w/proj/"));
        assert_eq!(
            parse_root(b"... clientName *unknown*\n... clientCwd /tmp\n"),
            None
        );
        let fstat = b"... action edit\n... clientFile /w/proj/ota.shcomp\n\n\
... action add\n... clientFile /w/proj/cells/new.shcomp\n\n\
... action delete\n... clientFile /w/proj/old.lua\n\n\
... action edit\n... clientFile /w/other/x.lua\n";
        let rows = parse_status(fstat, "/w/proj/");
        let want = [
            ('M', "ota.shcomp"),
            ('A', "cells/new.shcomp"),
            ('D', "old.lua"),
        ];
        assert_eq!(rows, want.map(|(m, p)| (m, p.to_owned())));
        let changes = b"... change 1234\n... time 1700000000\n... user ann\n... client ann-ws\n\
... desc Size the tail\nand bias it\n\n... change 1200\n... time 1600000000\n... user bo\n... desc Start\n";
        let log = parse_log(changes);
        assert_eq!(log.len(), 2);
        assert_eq!(
            (log[0].rev.as_str(), log[0].author.as_str()),
            ("1234", "ann")
        );
        assert_eq!(
            (log[0].time, log[0].summary.as_str()),
            (1_700_000_000, "Size the tail")
        );
    }

    /// The command reference's `annotate` sample, with `-c` numbers.
    #[test]
    fn annotate_lines_carry_their_change() {
        let out = b"320: sr->w.digest.Clear();\n172:   sr->w.size.Unknown();\n";
        assert_eq!((P4.parse_annotate)(out), ["320", "172"]);
    }
}
