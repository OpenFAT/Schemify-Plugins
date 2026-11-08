//! The plugin over one [`Backend`] row (wasm only): events in → the pane, the Explorer's marks,
//! diff and revision tabs and annotate files out. Every program run is a `spawn` answered later
//! by `IoDone`; nothing blocks.
//!
//! - The status is read on `Load`, on `Saved` (an open only reads, so it runs nothing) and on
//!   `<name>.show`/`<name>.refresh`: [`Backend::root`], then [`Backend::status`]. One read at
//!   a time; a request meanwhile reads once more after it.
//! - `<name>.diff <k>`: row `k`'s diff in a text tab: a design file's semantic diff
//!   ([`Backend::base`] against the working file, `schemify_format::diff_text`), another
//!   file's [`Backend::diff`]. One diff at a time; a newer click replaces it.
//! - `vcs.show <rev> <path>`: the file at a revision in a text tab ([`Backend::show`]).
//! - `vcs.annotate <path>`: [`Backend::annotate`], [`Backend::log`] and the working file, then
//!   the annotate file ([`crate::annotate`]), written whole (`fs_write` is atomic). One at a time.
//!
//! The `vcs.*` names are the convention every VCS plugin declares (docs/plugins/vcs.md); the
//! last one loaded owns them.
//!
//! Contents: `pub fn on_event`, `pub fn commands`

use std::cell::RefCell;

use schemify_abi::guest::Host;
use schemify_abi::{Event, Kind, Level, Out, text};
use schemify_format as format;

use crate::{Backend, annotate, fill, pane};

/// File extensions whose diff is the semantic one (format spec §8).
const DESIGN: [&str; 3] = [".shcomp", ".shprim", ".shbench"];

/// The status as last read, and what is in flight.
#[derive(Default)]
struct State {
    /// The project root inside the working copy ([`Backend::parse_root`]).
    prefix: String,
    /// `(marker, path relative to the project root)`, in the VCS's order.
    rows: Vec<(char, String)>,
    /// Why there are no rows (not a working copy, the program missing), else empty.
    note: String,
    /// The status read's step.
    read: Read,
    /// A read was asked for while one ran.
    again: bool,
    /// The diff or revision tab in flight: its title and parts.
    tab: Option<Tab>,
    /// The annotate in flight.
    blame: Option<Blame>,
}

/// The status read's step.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum Read {
    #[default]
    Idle,
    Root(u32),
    Status(u32),
}

/// A tab in flight: `head` is the base, the diff or the revision's text; `work` the working
/// file (a design file's semantic diff only).
struct Tab {
    rel: String,
    title: String,
    /// What the body is made of.
    kind: Body,
    head: Part,
    work: Part,
}

/// A tab's body.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Body {
    /// `head` as it is (a text diff, a revision).
    Text,
    /// The semantic diff of `head` (the base) against `work`.
    Semantic,
    /// An added or untracked file: its working text (`head`) after a note.
    Added,
}

/// An annotate in flight.
struct Blame {
    rel: String,
    revs: Part,
    log: Part,
    work: Part,
}

/// One awaited I/O result.
enum Part {
    None,
    Wait(u32),
    Got(i32, Vec<u8>),
}

thread_local! {
    /// The guest is single-threaded; the state outlives events.
    static STATE: RefCell<State> = RefCell::new(State::default());
}

/// The `commands` set of backend `b`: its pane's and the `vcs.*` convention's.
///
/// # Panics
///
/// Never: the assert counts the five lines written.
#[must_use]
pub fn commands(b: &Backend) -> String {
    let (n, t) = (b.name, b.title);
    let set = format!(
        "command {n}.show \"Source control\"\ncommand {n}.refresh \"Read the {t} status again\"\n\
         command {n}.diff \"Show a changed file's diff\"\n\
         command vcs.show \"Show a file at a revision ({t})\"\n\
         command vcs.annotate \"Write who changed each line of a file ({t})\"\n"
    );
    assert_eq!(set.lines().count(), 5);
    set
}

/// Handles one event for backend `b`; `extra` is the plugin's own `command` records, added to
/// the `commands` set. Returns whether it was this driver's (the plugin handles the rest: git's
/// `git.setup`).
pub fn on_event(b: &Backend, extra: &str, ev: Event<'_>, host: &Host, out: &mut Out) -> bool {
    STATE.with_borrow_mut(|s| {
        let n = b.name;
        match ev {
            Event::Load => {
                let mut set = commands(b);
                set.push_str(extra);
                out.contribute_commands(&set);
                show(b, s, out);
                start_read(b, s, host);
            }
            Event::Saved { .. } => start_read(b, s, host),
            Event::Command { name, args } => {
                match name.strip_prefix(n).and_then(|c| c.strip_prefix('.')) {
                    Some("show") => {
                        out.command("view.pane.scm", "");
                        start_read(b, s, host);
                    }
                    Some("refresh") => start_read(b, s, host),
                    Some("diff") => start_diff(b, s, host, out, args.trim()),
                    _ => match name {
                        "vcs.show" => start_show(b, s, host, out, args),
                        "vcs.annotate" => start_blame(b, s, host, out, args),
                        _ => return false,
                    },
                }
            }
            Event::IoDone {
                handle,
                status,
                bytes,
            } => return io_done(b, s, host, out, handle, status, bytes),
            _ => return false,
        }
        true
    })
}

// ---- private ----

/// Re-declares the pane and the Explorer's marks from the rows (immediate mode).
fn show(b: &Backend, s: &State, out: &mut Out) {
    out.contribute_ui(&pane::pane(b, &s.rows, &s.note));
    out.contribute_scoped(Kind::Annotations, "explorer", &pane::marks(&s.rows));
}

/// Runs `argv` (a filled template), its handle as an awaited part.
fn run(host: &Host, argv: &[String]) -> Part {
    let argv: Vec<&str> = argv.iter().map(String::as_str).collect();
    host.spawn(&argv, b"").map_or(Part::None, Part::Wait)
}

/// Starts a status read, or asks for one more after the read in flight.
fn start_read(b: &Backend, s: &mut State, host: &Host) {
    if s.read != Read::Idle {
        s.again = true;
        return;
    }
    s.read = match run(host, &fill(b.root, "", "", "")) {
        Part::Wait(h) => Read::Root(h),
        Part::None | Part::Got(..) => Read::Idle, // no grant (a policy dropped it)
    };
}

/// Takes one I/O result; `false` when the handle is none of this driver's.
fn io_done(
    b: &Backend,
    s: &mut State,
    host: &Host,
    out: &mut Out,
    h: u32,
    status: i32,
    bytes: &[u8],
) -> bool {
    match s.read {
        Read::Root(r) if r == h => {
            s.read = Read::Idle;
            match (status, (b.parse_root)(bytes)) {
                (0, Some(prefix)) => {
                    s.prefix = prefix;
                    if let Part::Wait(st) = run(host, &fill(b.status, "", "", "")) {
                        s.read = Read::Status(st);
                    }
                }
                _ => set_rows(b, s, host, out, Vec::new(), b.none_note),
            }
            return true;
        }
        Read::Status(r) if r == h => {
            s.read = Read::Idle;
            let (rows, note) = match status {
                0 => ((b.parse_status)(bytes, &s.prefix), String::new()),
                _ => (Vec::new(), format!("{} status failed", b.program)),
            };
            set_rows(b, s, host, out, rows, &note);
            return true;
        }
        Read::Idle | Read::Root(_) | Read::Status(_) => {}
    }
    let mut taken = false;
    for part in (s.tab.iter_mut().flat_map(|t| [&mut t.head, &mut t.work])).chain(
        s.blame
            .iter_mut()
            .flat_map(|a| [&mut a.revs, &mut a.log, &mut a.work]),
    ) {
        if matches!(part, Part::Wait(w) if *w == h) {
            *part = Part::Got(status, bytes.to_vec());
            taken = true;
        }
    }
    finish_tab(b, s, host, out);
    finish_blame(b, s, host, out);
    taken
}

/// Takes a finished status read, then the next read if one was asked for.
fn set_rows(
    b: &Backend,
    s: &mut State,
    host: &Host,
    out: &mut Out,
    rows: Vec<(char, String)>,
    note: &str,
) {
    assert!(rows.len() <= pane::ROWS_MAX);
    if rows != s.rows || note != s.note {
        s.rows = rows;
        s.note = note.to_owned();
        show(b, s, out);
    }
    if std::mem::take(&mut s.again) {
        start_read(b, s, host);
    }
}

/// Starts row `arg`'s diff: a deleted file's at once; an added or untracked file's working
/// text; a design file's base and working text; another file's text diff.
fn start_diff(b: &Backend, s: &mut State, host: &Host, out: &mut Out, arg: &str) {
    let Some((mark, rel)) = arg
        .parse::<usize>()
        .ok()
        .and_then(|k| s.rows.get(k))
        .cloned()
    else {
        out.log_at(
            Level::Warn,
            &format!("{}.diff: no changed file {arg:?}", b.name),
        );
        return;
    };
    let title = format!("{rel} (diff)");
    let design = DESIGN.iter().any(|e| rel.ends_with(e));
    let read = |host: &Host| host.fs_read(&rel).map_or(Part::None, Part::Wait);
    let (kind, head, work) = match mark {
        'D' => {
            out.text_tab(&title, "deleted from the working copy\n");
            return;
        }
        'U' | 'A' => (Body::Added, read(host), Part::None),
        _ if design => (
            Body::Semantic,
            run(host, &fill(b.base, &rel, &s.prefix, "")),
            read(host),
        ),
        _ => (
            Body::Text,
            run(host, &fill(b.diff, &rel, &s.prefix, "")),
            Part::None,
        ),
    };
    s.tab = Some(Tab {
        rel,
        title,
        kind,
        head,
        work,
    });
    finish_tab(b, s, host, out);
}

/// `vcs.show <rev> <path>`: the file at revision `rev` in a text tab.
fn start_show(b: &Backend, s: &mut State, host: &Host, out: &mut Out, args: &str) {
    let mut v = text::values(args);
    let (Some(Ok(rev)), Some(Ok(rel))) = (v.next(), v.next()) else {
        out.log_at(Level::Warn, "vcs.show: needs a revision and a path");
        return;
    };
    let head = run(host, &fill(b.show, &rel, &s.prefix, &rev));
    let title = format!("{rel} @ {rev}");
    s.tab = Some(Tab {
        rel: rel.into_owned(),
        title,
        kind: Body::Text,
        head,
        work: Part::None,
    });
    finish_tab(b, s, host, out);
}

/// Opens the tab in flight once its parts landed; a design file whose sides do not both
/// parse falls back to the text diff.
fn finish_tab(b: &Backend, s: &mut State, host: &Host, out: &mut Out) {
    let Some(t) = &s.tab else { return };
    if matches!(t.head, Part::Wait(_)) || matches!(t.work, Part::Wait(_)) {
        return;
    }
    let Some(t) = s.tab.take() else { return };
    let body = match (&t.head, &t.work) {
        (Part::Got(0, text), _) if t.kind == Body::Added => {
            format!(
                "new file, not in the last revision\n\n{}",
                String::from_utf8_lossy(text)
            )
        }
        (_, _) if t.kind == Body::Added => format!("new file\n\n{}: not readable\n", t.rel),
        (Part::Got(_, head), Part::Got(_, work)) if t.kind == Body::Semantic => {
            match (format::parse(head), format::parse(work)) {
                (Ok(a), Ok(c)) => match format::diff_text(&a, &c) {
                    d if d.is_empty() => "no semantic change (formatting only)\n".to_owned(),
                    d => d,
                },
                _ => {
                    let head = run(host, &fill(b.diff, &t.rel, &s.prefix, ""));
                    s.tab = Some(Tab {
                        kind: Body::Text,
                        head,
                        work: Part::None,
                        ..t
                    });
                    return finish_tab(b, s, host, out);
                }
            }
        }
        (Part::Got(0, text), _) if text.is_empty() => "no change\n".to_owned(),
        (Part::Got(0, text), _) => String::from_utf8_lossy(text).into_owned(),
        (Part::Got(_, err), _) => {
            format!("{} failed: {}\n", b.program, String::from_utf8_lossy(err))
        }
        (Part::None | Part::Wait(_), _) => format!("{} is not available\n", b.program),
    };
    assert!(!body.is_empty(), "a tab has a body");
    out.text_tab(&t.title, &body.replace('\0', ""));
}

/// `vcs.annotate <path>`: the per-line revisions, the log and the working file.
fn start_blame(b: &Backend, s: &mut State, host: &Host, out: &mut Out, args: &str) {
    let Some(Ok(rel)) = text::values(args).next() else {
        out.log_at(Level::Warn, "vcs.annotate: needs a path");
        return;
    };
    let rel = rel.into_owned();
    if b.annotate.is_empty() {
        out.log_at(
            Level::Warn,
            &format!("vcs.annotate: {} has no per-line annotate", b.title),
        );
        return;
    }
    let revs = run(host, &fill(b.annotate, &rel, &s.prefix, ""));
    let log = run(host, &fill(b.log, &rel, &s.prefix, ""));
    let work = host.fs_read(&rel).map_or(Part::None, Part::Wait);
    s.blame = Some(Blame {
        rel,
        revs,
        log,
        work,
    }); // replaces one in flight
    finish_blame(b, s, host, out);
}

/// Writes the annotate file once every part landed; a failed run is one log line.
fn finish_blame(b: &Backend, s: &mut State, host: &Host, out: &mut Out) {
    let Some(a) = &s.blame else { return };
    if [&a.revs, &a.log, &a.work]
        .iter()
        .any(|p| matches!(p, Part::Wait(_)))
    {
        return;
    }
    let Some(a) = s.blame.take() else { return };
    let (Part::Got(0, revs), Part::Got(0, log), Part::Got(0, work)) = (&a.revs, &a.log, &a.work)
    else {
        out.log_at(
            Level::Warn,
            &format!("vcs.annotate: {} could not annotate {}", b.title, a.rel),
        );
        return;
    };
    let rows = annotate::join(&(b.parse_annotate)(revs), &(b.parse_log)(log));
    let file = annotate::text(work, b.name, &rows);
    if host
        .fs_write(&annotate::path(&a.rel), file.as_bytes())
        .is_err()
    {
        out.log_at(Level::Error, "vcs.annotate: needs the write_project grant");
    }
}
