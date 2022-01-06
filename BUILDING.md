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
2. Run `rustup.exe toolchain install stable-x86_64-pc-windows-msvc` to install the MSVC toolchain.
3. Run `rustup.exe default stable-msvc` to select that toolchain as the default.
4. Download this project and extract it somewhere.
5. Open a terminal in the folder where you extracted Hyperspeedcube (it should have `Cargo.toml` in it) and build it using `cargo build --release` or run it using `cargo run --release`.

The first build may take ~10 minutes or more. Remove `--release` to disable optimizations, which makes building faster but Hyperspeedcube may run slower.
