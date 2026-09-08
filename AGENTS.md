# Agent guidelines

See `crates/README.md` and `crates/*/README.md` for info on individual crates.

## Process

### Before making code changes

- Run `cargo +nightly fmt --check`. If it fails, notify the user and wait for them before continuing.

### After making code changes

- Run `cargo +nightly fmt` when you are done making code changes, or before committing.
- Only commit if explicitly requested in your instructions.

### Committing

- Permission to commit once does not constitute permission to commit multiple times.
  - "Commit this" -> permission to commit once
  - "Continue making commits" -> permission to commit multiple times
- If you do not have permission to commit and have reached a point where it makes sense to commit, ask the user whether you should commit. The user may commit themselves, or they may make edits and then prompt you to commit.
- When committing, always add the exact phrase `Co-authored-by: LLM <>` as a new paragraph at the end of the commit description.

## Code

- Prefer `TiVec` via aliases (`PerAxis`, `PerColor`, etc.) over `Vec` when applicable.
  - `TiVec<I, T>` is like `Vec<T>` except that it is indexed using `I` and indexing with `.get()` results a `Result`.
- Prefer the `Float` alias over `f64` when working with geometry.
- User-facing strings in the UI layer (`hyperspeedcube`) should use entries from `crate::L`, which is automatically generated from `crates/hyperspeedcube/locales/en.kdl`.
