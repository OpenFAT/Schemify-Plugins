# Version control plugins

Schemify has no version control of its own. git and every other system (Jujutsu, Mercurial, Subversion, Perforce, a team's in-house or commercial data manager) is a plugin. Each one uses the same generic hooks as any other plugin ([plugin ABI](../spec/plugin-abi.md) §5, §7).

The first-party providers live in [Scouren/Schemify-Git](https://github.com/Scouren/Schemify-Git), one plugin crate each:

| Plugin | Built-in | VCS |
|---|---|---|
| `git` | on by default | git |
| `hg` | off until enabled | Mercurial |
| `jj` | off until enabled | Jujutsu |
| `svn` | off until enabled | Subversion |
| `p4` | off until enabled | Perforce Helix Core |

They ship inside the app as pinned built-ins, the same way as gmid and interop. The store's Plugins tab lists them under **Version control**. A project uses one version-control plugin at a time.

## The hooks a VCS plugin uses

| What the person sees | The hook | The git plugin's use |
|---|---|---|
| The Source control pane and its icon | A `ui` contribution: `pane scm "Source control" dock left size 240`, the content node `scm.pane`, and `node activity.scm button` with `parent activity` and `order 30` (the icon bar leaves that slot free). | Rows `scm.row.<k>` that run `git.diff.<k>`. |
| A letter after a changed file in the Explorer | `contribute annotations explorer` with `mark <project path> <severity> <text>`. | `mark ota.shcomp info M` |
| A diff, a log or a file at a revision | The `text <title>` record (`Out::text_tab`): a read-only tab that this plugin owns. | `ota.shcomp (diff)` |
| Running the VCS program | The `spawn:<program>` grant and `Host::spawn`. The result arrives as `Event::IoDone`. | `git status --porcelain=v1 -z`, `git show`, `git blame --porcelain` |
| A design file's changes, object by object | `schemify_format::diff_text(old, new)`, linked into the guest. | The `HEAD` text compared with the working file. |
| An action the person starts | A `commands` contribution. Running `<name>.<word>` passes `<word>` as the argument. | `git.refresh`, `git.diff`, `git.setup` |

Only one plugin should declare the pane `scm` and the icon `activity.scm`. If a second one declares them, it overrides the first (D8: the plugin loaded last wins).

## The `vcs.*` convention

Plugins cannot call each other. A plugin that wants version-control data (the blame plugin) asks through two commands that every VCS plugin declares. The last VCS plugin loaded owns these names.

| Command | Arguments | What the VCS plugin does |
|---|---|---|
| `vcs.annotate` | a project path | Writes the annotate file of that path (below). |
| `vcs.show` | a revision, a project path | Opens the file at that revision in a text tab. |

### The annotate file

`vcs.annotate <path>` writes `.schemify/vcs/annotate/<path>.tsv`. Projects keep `.schemify/` out of version control. The file is written whole: `fs_write` writes a temporary file and renames it, so a reader never sees half of one.

- **The first line is a header:** `schemify-annotate 1 <hash> <vcs>`.
  - `1` is the format version.
  - `<hash>` is `schemify_format::fnv1a64(FNV_OFFSET, bytes)` of the working file when the plugin ran, as 16 lowercase hex digits.
  - `<vcs>` is the plugin's name (`git`).
- **Then there is one line per line of the file as the VCS annotates it:** `<rev>\t<author>\t<unix time>\t<summary>`.
  - The fields hold no tabs or line breaks.
  - A line the log does not hold (not committed yet) is `\t\t0\tnot committed`.
- **git annotates the working file.** The others annotate the last committed revision.

A reader hashes the working file the same way and ignores a file whose hash differs: it is stale. Nothing signals that the file has been written, so the reader polls it with a timer, backs off, and gives up after a timeout. `schemify_vcs::annotate::parse` reads the file and checks the header.

## Rules

- Read the repository only for the person, and never block the editor. Run every program through `spawn`, so its result lands on a later frame.
- Commits, checkouts and anything else that changes the repository are the person's own, in a terminal or in the VCS's tools. A plugin never starts them on its own. The editor already refuses consent commands from plugins (`Cmd::needs_user`).
- Read the status only when it can have changed: on `Load`, on `Saved`, and when the person asks (the pane's Refresh button). Opening a file only reads it, so it should run nothing.
- Keep the pane's list bounded. The shared pane shows at most 1024 rows.

## Writing a provider for another VCS

A team with an in-house or commercial version-control system can write its own provider. The Rust helper crate [`schemify-vcs`](https://github.com/Scouren/Schemify-Plugins/tree/main/sdk/rust/vcs) does most of the work: one `Backend` row per VCS, and a driver that turns the row into the pane, the marks, the diffs and the `vcs.*` convention.

1. **Write the row.** Fill these fields:
   - the program;
   - the argv of each operation: detect, status, base, diff, show, log and annotate (`{rel}`, `{repo}` and `{rev}` are filled in for you);
   - a parser for each output that is not plain text: status rows, log rows, and one revision per annotated line.
2. **Test the parsers natively.** Feed them output copied from the tool's documented format. The rows in `schemify_vcs::backends` show the shape: each has tests written from its tool's reference.
3. **Make the plugin crate.** It is about ten lines: `export!` an `on_event` that calls `schemify_vcs::driver::on_event(&ROW, "", ev, host, out)`. Request `spawn:<program>`, `read_project`, `write_project` and `commands` in its `schemify.project`.
4. **Copy the build.** Copy a crate of [Scouren/Schemify-Git](https://github.com/Scouren/Schemify-Git) for its layout, build script and CI.

A VCS whose command line documents no per-line annotate output can still have the pane, the marks and the diffs: leave `annotate` empty (`&[]`), and the driver answers `vcs.annotate` with a note instead of a file.

In another language, use the same hooks directly: the table above lists every record the pane needs.
