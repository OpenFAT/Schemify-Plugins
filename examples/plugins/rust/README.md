# hello

A Schemify plugin in Rust: **Tools → Count nets** logs the nets that have a single
connection, with their names (`2 single-connection nets: c, d`).

## Build

```sh
rustup target add wasm32-unknown-unknown
cargo build --release
cp target/wasm32-unknown-unknown/release/hello.wasm .
schemify plugin check .
```

## Store listing

`listing`, `icon.png`, this README and `CHANGELOG.md` are what the store shows. Change
`publisher` in `listing` to your GitHub org, then follow "Your store listing" in
`docs/plugins/publishing.md`.
