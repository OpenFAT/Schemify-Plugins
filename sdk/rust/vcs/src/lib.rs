//! What every version-control plugin shares (docs/plugins/vcs.md). A VCS is one [`Backend`]
//! row: its program, the argv of each operation and the parsers of their output. The rows
//! live in [`backends`] (git, Mercurial, Jujutsu, Subversion, Perforce); each plugin crate
//! (github.com/Scouren/Schemify-Git) is the `driver` over its row (wasm only). The pane, the Explorer's marks and
//! the annotate file ([`annotate`]) are the same for every row.
//!
//! Operations, each an argv template run with `spawn` in the project root (`{rel}`: the path
//! relative to the project root; `{repo}`: the same path inside the working copy; `{rev}`: a
//! revision):
//! - detect: [`Backend::root`] succeeds in a working copy, and its output gives the project's
//!   place inside it ([`Backend::parse_root`]);
//! - status: [`Backend::status`] → rows `(marker, rel)`, markers `M A D R U`;
//! - diff and show: [`Backend::base`] (the file at the base revision: the semantic diff of a
//!   design file), [`Backend::diff`] (another file's text diff), [`Backend::show`] (any
//!   revision);
//! - log: [`Backend::log`] → [`LogRow`]s; annotate: [`Backend::annotate`] → one revision per
//!   line, joined with the log ([`annotate::join`]).
//!
//! Commits and checkouts are not here: they change the repository, so they are the person's
//! own, in their terminal or the VCS's tools (docs/plugins/vcs.md, Rules).
//!
//! Contents: `pub struct Backend`, `pub type Rows`, `pub struct LogRow`, `pub fn fill`;
//! `pub mod backends,
//! pane, annotate, driver` (driver: wasm only).

// Natively only the tests build (the driver is wasm only), so some parts look unused.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

pub mod annotate;
pub mod backends;
#[cfg(target_arch = "wasm32")]
pub mod driver;
pub mod pane;

/// One version-control system: how to run each operation and read its output. A `const`
/// per VCS in [`backends`]; argv templates hold `{rel}`, `{repo}` and `{rev}` ([`fill`]).
#[derive(Debug)]
pub struct Backend {
    /// The plugin's name: its built-in name and its commands' prefix (`hg.refresh`).
    pub name: &'static str,
    /// The name the person reads (`Mercurial`).
    pub title: &'static str,
    /// The program every operation runs (also the `spawn:` grant).
    pub program: &'static str,
    /// Detection: exits 0 only in a working copy.
    pub root: &'static [&'static str],
    /// The project root's place inside the working copy from [`Backend::root`]'s output:
    /// the prefix [`Backend::parse_status`] strips (`""` when status prints project paths).
    pub parse_root: fn(&[u8]) -> Option<String>,
    /// The pane's note outside a working copy.
    pub none_note: &'static str,
    /// The working copy's changes under the project.
    pub status: &'static [&'static str],
    /// `status` output and the prefix → rows `(marker, path relative to the project root)`.
    pub parse_status: fn(&[u8], &str) -> Rows,
    /// Prints the file at the base revision (what the working file is compared with).
    pub base: &'static [&'static str],
    /// Prints the file's text diff against the base revision.
    pub diff: &'static [&'static str],
    /// Prints the file at revision `{rev}`.
    pub show: &'static [&'static str],
    /// Prints the file's history ([`Backend::parse_log`]).
    pub log: &'static [&'static str],
    /// `log` output → rows, newest first.
    pub parse_log: fn(&[u8]) -> Vec<LogRow>,
    /// Prints one revision per line of the file ([`Backend::parse_annotate`]); empty for a
    /// VCS without a per-line annotate (`vcs.annotate` then answers with a note).
    pub annotate: &'static [&'static str],
    /// `annotate` output → the revision of each line, in line order.
    pub parse_annotate: fn(&[u8]) -> Vec<String>,
}

/// Status rows: `(marker, path relative to the project root)`, markers `M A D R U`.
pub type Rows = Vec<(char, String)>;

/// One revision of a file's history.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LogRow {
    /// The revision as the VCS names it (a hash, a number, a changelist).
    pub rev: String,
    /// Who made it.
    pub author: String,
    /// When, in seconds since 1970 (UTC).
    pub time: i64,
    /// The first line of its message.
    pub summary: String,
}

/// `template` with `{rel}`, `{repo}` (`prefix` + `rel`) and `{rev}` filled in, one argument
/// per word as written (no word splitting of the values, so any path is safe).
///
/// # Panics
///
/// If `template` is empty (every row's argv names its program).
#[must_use]
pub fn fill(template: &[&str], rel: &str, prefix: &str, rev: &str) -> Vec<String> {
    assert!(!template.is_empty(), "an argv names its program");
    let repo = format!("{prefix}{rel}");
    let argv: Vec<String> = (template.iter())
        .map(|a| {
            a.replace("{rel}", rel)
                .replace("{repo}", &repo)
                .replace("{rev}", rev)
        })
        .collect();
    assert_eq!(argv.len(), template.len(), "one argument per template word");
    argv
}

/// Splits `out` into lines, `\r` stripped, empty lines kept out.
pub(crate) fn lines(out: &[u8]) -> impl Iterator<Item = String> + '_ {
    out.split(|&b| b == b'\n')
        .map(|l| String::from_utf8_lossy(l.strip_suffix(b"\r").unwrap_or(l)).into_owned())
        .filter(|l| !l.is_empty())
}

/// `text` as one tab-separated field: tabs and line breaks become spaces.
pub(crate) fn field(text: &str) -> String {
    text.replace(['\t', '\n', '\r'], " ")
}

#[cfg(test)]
mod tests {
    #[test]
    fn fill_keeps_one_argument_per_word() {
        let argv = super::fill(&["git", "show", "{rev}:{repo}"], "a b.lua", "p/", "HEAD");
        assert_eq!(argv, ["git", "show", "HEAD:p/a b.lua"]);
        assert_eq!(super::field("a\tb\nc"), "a b c");
    }
}
