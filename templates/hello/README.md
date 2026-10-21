# hello: a starting point

A page with one component tree, one callback and a little CSS. Copy this folder, point the `moth` path in
`Cargo.toml` at wherever moth2 lives, and:

```
rustup target add wasm32-unknown-unknown
cargo install trunk --locked
trunk serve        # builds, serves on http://127.0.0.1:8080 and rebuilds when a file changes
```

`trunk build --release` makes an optimized `dist/` folder you can put on any static web server.

What to read next: [getting started](../../docs/getting-started.md), then the
[tutorial](../../docs/tutorial.md).
