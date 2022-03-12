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
