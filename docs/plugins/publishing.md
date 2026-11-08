# Publishing a plugin

## What users install

`schemify pack` installs a git repository by cloning it and loading the module at its root ([config.md](../spec/config.md) §5). There are no build hooks: the committed `.wasm` is what runs. So a release is a commit whose module is up to date, and CI's reproducibility check ([testing.md](testing.md)) guarantees that every commit on `main` is one.

## Releases

Tag a release with a version (`git tag v0.3.0 && git push --tags`). Users pin it with `version`, or follow a branch with `rev`:

```lua
-- init.lua
schemify.pack.add {
  { src = "https://github.com/Scouren/GmIdW", version = "v0.3.0",
    grant = { "read_project", "write_project", "spawn:ngspice" } },
}
```

`schemify pack sync` clones it and records the commit and the module's sha256 in `schemify-pack.lock`. `schemify pack update` moves to the newest commit of `rev`/`version`. The user grants capabilities; the plugin gets the intersection of those and what its `schemify.project` requests.

A project that needs the plugin (a docs page that calls its functions, a PDK flow) adds the same spec in its own Lua file, the one its `schemify.project` names with `init <file>` (config spec §2). Anyone who opens the project is asked to trust that file, and the plugin is then downloaded into the project and pinned in its `schemify-pack.lock`.

Users who cannot use git can install a single module with `url` and `sha256`. Attach the `.wasm` to a GitHub release for that.

## Moving to a newer Schemify

1. Pick the Schemify commit (usually `main` when the nightly is green).
2. Replace the `rev` of every Schemify dependency in `Cargo.toml` with its full sha, all to the same commit.
3. `cargo update -p schemify-abi` (and every other Schemify crate you use), then `./build.sh` and `cargo test`.
4. Commit `Cargo.toml`, `Cargo.lock` and the rebuilt `.wasm` together.

The module exports the ABI version it was built for (`schemify_abi`, [plugin-abi.md](../spec/plugin-abi.md) §2). An editor with another ABI version refuses to load it (`E_VERSION`) instead of running it with the wrong layout.

## Listing a plugin in Schemify

Schemify lists a few plugins in its [README.md](README.md); adding one is a pull request that adds its row. Schemify's CI does not build or test them; the plugin's own CI is what vouches for it.

## Your store listing

The Schemify store lists plugins and themes from one signed index. Each package is one file,
`packages/<name>`, in the public repository [Scouren/schemify-index](https://github.com/Scouren/schemify-index).
You change it by pull request; Scouren reviews it, signs the index with an offline key and
publishes it with a copy of every file it names, stored by its sha256, so nothing you link
can change after review. [examples/listing/](../../examples/listing) is a complete listing (a
theme, so it needs no build); the [Rust template](../../examples/plugins/rust) has one too.

### Four commands

```sh
schemify plugin new plugin netcount --lang rust --publisher acme   # or: new theme dusk
schemify plugin check netcount      # what the index will check, one line per problem
schemify plugin preview netcount    # your store page, as users will see it; installs nothing
schemify plugin publish netcount --index ~/src/schemify-index
```

- `new` copies the template for `--lang` (`rust c zig go odin as`) from this repository and adds
  everything below: the listing, a README with a screenshot slot, a CHANGELOG, a LICENSE,
  placeholder PNGs, `test.sh` (build, then `check`) and a CI workflow.
- `check` builds the entry from your folder and reads it with the store's own reader, then
  checks what a reader cannot: grant reasons, the README rules, image sizes and PNG, a licence
  file, and that a theme holds no code and removes no node the editor needs.
- `preview` opens the editor's store on your listing from a local, unsigned index. Install is
  refused there: a preview only shows.
- `publish` writes the entry for the version at the top of your CHANGELOG, every file at your
  GitHub release `v<version>` (`--base <url>` for elsewhere) with its sha256, and prints the
  files to attach to that release. With `--index <checkout>` it writes `packages/<name>` there,
  keeps your earlier releases below the new one, commits on a branch and opens the pull
  request with `gh` (or prints the steps without it).

### The folder

```
netcount/
  schemify.project   # the plugin record and the grants it requests (or the theme record)
  netcount.wasm      # the built module (a theme: its .records file)
  listing            # the store fields below
  README.md          # the Details section
  CHANGELOG.md       # the Changelog section; its first `## x.y.z` is the version
  LICENSE            # the licence `license` names
  icon.png  banner.png  screenshot.png
```

### The `listing` file

```
schemify-listing 1
publisher acme
license MIT
categories docs
summary Logs the nets that have a single connection.
icon icon.png
banner banner.png
screenshot screenshot.png Count nets on an OTA
image screenshot.png
grant read_project Reads the open design to count its nets
tool netcount.count Count the nets with a single connection
```

The first line is `schemify-listing 1`; the others may come in any order. Files are paths
inside the folder. Every line is plain ASCII.

| Line | Value |
|---|---|
| `publisher` | Your GitHub org or user, lowercase (`a-z 0-9 -`, at most 39). The same for every release. |
| `license` | An SPDX id (`MIT`, `Apache-2.0`, …). A `LICENSE`, `LICENSE.md`, `LICENSE.txt`, `COPYING` or `LICENSES/` must be in the folder. |
| `categories` | 1 to 4 of: `sizing simulation layout interop verification docs agent pdk` (plugins), `dark light high-contrast vscode` (themes). |
| `summary` | One line, at most 512 bytes: the tile's text. |
| `icon` | A 256 × 256 PNG, at most 64 KiB. Without one the tile shows coloured initials. |
| `banner` | A PNG, at most 512 KiB, over the Details section. |
| `screenshot` | `<file> <caption>`: up to 6 PNGs, at most 1 MiB each, shown in a strip with their one-line captions. |
| `image` | A README image that may show (below): up to 16 PNGs, at most 1 MiB each. |
| `grant` | `<capability> <reason>`: one for each capability `schemify.project` requests, no more and no fewer. |
| `tool` | `<command> <description>`: an agent tool the release exposes (up to 32), listed under Agent tools. |
| `paid` | `<cents> <sku> <seller>`: a paid listing (after seller onboarding with Scouren). Leave it out for a free one. |
| `includes`, `variants`, `token` | Themes: what it restyles (`canvas chrome waves docs`), its variants (`dark light high-contrast`), and up to 16 `token <name> <rrggbb>` colours (`canvas.bg`, `canvas.wire`, `canvas.symbol`, `canvas.pin`, `canvas.select`, `canvas.grid`) the store's preview paints the user's own design with. |

The name is the one in `schemify.project` (a plugin's name, a theme's id). `verified` and
`installs` are Scouren's: the store counts installs and shows a bucket (`↓ 4.1k`), never
ratings.

### Images: PNG only

The store shows PNG only; a JPEG is refused by name. Each image is pinned by its sha256 in the
index and kept by that hash, so it is shown only if its bytes are exactly the ones reviewed.
Sizes: icon 256 × 256 and 64 KiB, banner 512 KiB, screenshots and README images 1 MiB.

### README rules

The Details section renders your README as markdown with the docs page's widget:

- No raw HTML. Links are shown, never followed inside the app.
- An image shows only when it stands **alone on its line** and its path (beside the README,
  `![The sweep](screenshot.png)`) is an `image` line of the listing. Any other image, and any
  image from another site, shows as *Image not shown in the store*. A `plot` fence shows as
  *Plot not shown in the store*.
- At most 64 KiB. The changelog has the same limit; its images never show.

### Changelog

`CHANGELOG.md` is the Changelog section. Its first `## ` heading names the version you publish
(`## 1.2.0`, `## [1.2.0] - 2026-11-07` or `## v1.2.0`); `check` and `publish` need it.

### Agent tools and settings

`tool` lines name the commands the AI assistant may call; they show under **Agent tools** and
in the side column. Settings come from your plugin, not the listing: the options it contributes
(`contribute options`, [plugin ABI](../spec/plugin-abi.md) §7) show under **Settings** once the
plugin is installed.

### Grants and reasons

Users see every grant with your reason in the permission dialog before anything installs. Say
what the plugin does with it ("Reads the cells it sizes"), not what the capability is; `check`
refuses reasons under 12 bytes. An update that requests a new grant installs only after each
user allows it again; until then the old version stays.

### Review

The index's pull-request checks read the format exactly as the store does, fetch every file
and compare its sha256, enforce the image rules and compare grants with your previous release.
A reviewer then reads it, and Scouren checks it again before publishing. These always get a manual review:
a new publisher, a new grant and any risky grant (`spawn:*`, `network`, `write_project`,
`read:*`, `commands`), a paid listing, and a removed listing. Releases already listed are never
edited. No NDA content, ever: nothing from a foundry kit (model cards, rules, decks) in the
package, its README, images or changelog. A listing that breaks a rule is removed and its builds
are revoked, which disables installed copies.

### When users see your update

Schemify never checks for updates at startup and never installs on its own. After Scouren
publishes the index with your release, users see **Update** on your package when they open the
store, or when they press *Check for updates* in the Packages pane.
