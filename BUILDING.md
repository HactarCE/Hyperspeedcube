# Building Hyperspeedcube

**Hyperspeedcube requires the latest version of the Rust compiler.**

## Building on Linux or macOS

1. Download/install Cargo.
2. Clone this project and build/run:

```sh
git clone https://github.com/HactarCE/Hyperspeedcube
cd Hyperspeedcube
cargo run --release
```

The first build may take ~10 minutes or more. Remove `--release` to disable optimizations, which makes building faster but Hyperspeedcube may run slower.

## Building on Windows

1. Download/install [Rustup](https://www.rust-lang.org/tools/install).
2. Download this project and extract it somewhere.
3. Open a terminal in the folder where you extracted Hyperspeedcube (it should have `Cargo.toml` in it) and build it using `cargo build --release` or run it using `cargo run --release`.

The first build may take ~10 minutes or more. Remove `--release` to disable optimizations, which makes building faster but Hyperspeedcube may run slower.

## Building for web

1. Follow instructions above to run the native version first.
2. Install wasm32 target with `rustup target add wasm32-unknown-unknown`
3. Install Trunk with `cargo install --locked trunk`
4. Run `trunk serve` to build and serve on <http://127.0.0.1:8080>. Trunk will rebuild automatically if you edit the project. Open <http://127.0.0.1:8080/index.html#dev> in a browser.

Note that `assets/sw.js` script will try to cache the app, and loads the cached version when it cannot connect to server allowing the app to work offline (like PWA). appending `#dev` to `index.html` will skip this caching, allowing to load the latest builds during development.

Due to [cargo#8662](https://github.com/rust-lang/cargo/issues/8662) / [cargo#8716](https://github.com/rust-lang/cargo/issues/8716), switching between WASM and native may cause a rebuild of the full program. To work around this, set the `CARGO_TARGET_DIR` environment variable to point to a different directory when running `trunk serve`. `serve-web.ps1` accomplishes this on Windows.
