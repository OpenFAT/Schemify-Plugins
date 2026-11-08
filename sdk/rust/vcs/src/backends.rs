//! The rows: one [`Backend`] per version-control system, each with its CLI's output parsers
//! in a file of its own. The argv and the output formats follow each tool's public reference
//! (cited per file); the tests feed output written from those references.
//!
//! | Row | Detect | Status | Base / show | Diff | Log | Annotate |
//! |---|---|---|---|---|---|---|
//! | [`GIT`] | `git rev-parse` | `status --porcelain=v1 -z` | `show HEAD:` / `show <rev>:` | `diff HEAD` | `log --format` | `blame --porcelain` |
//! | [`HG`] | `hg root` | `status` | `cat -r .` / `cat -r` | `diff` | `log -T` | `annotate -T` |
//! | [`JJ`] | `jj root` | `diff --summary` | `file show -r @-` / `-r` | `diff --git` | `log -T` | `file annotate -T` |
//! | [`SVN`] | `svn info` | `status` | `cat -r BASE` / `-r` | `diff` | `log` | `blame` |
//! | [`P4`] | `p4 -ztag info` | `fstat -Ro` | `print #have` / `@<change>` | `diff` | `changes -l` | `annotate -c -q` |
//!
//! Contents: `pub const GIT`, `HG`, `JJ`, `SVN`, `P4`, `ALL`

mod git;
mod hg;
mod jj;
mod p4;
mod svn;

pub use git::GIT;
pub use hg::HG;
pub use jj::JJ;
pub use p4::P4;
pub use svn::SVN;

use crate::Backend;

/// Every row, git first.
pub const ALL: [&Backend; 5] = [&GIT, &HG, &JJ, &SVN, &P4];

/// The rows' names are distinct words and every row names its program first in every argv.
#[cfg(test)]
mod tests {
    #[test]
    fn rows_are_well_formed() {
        for (i, b) in super::ALL.iter().enumerate() {
            assert!(
                super::ALL[..i].iter().all(|o| o.name != b.name),
                "{}",
                b.name
            );
            let annotate = Some(b.annotate).filter(|a| !a.is_empty());
            for argv in [b.root, b.status, b.base, b.diff, b.show, b.log]
                .into_iter()
                .chain(annotate)
            {
                assert_eq!(argv[0], b.program, "{}: {argv:?}", b.name);
            }
            assert!(b.show.iter().any(|a| a.contains("{rev}")), "{}", b.name);
        }
    }
}
