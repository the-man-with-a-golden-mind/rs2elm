### Formatting
The project uses unstable `rustfmt` settings, so the formatting has to be done with nightly.
```console
rustup run nightly rustfmt --edition 2024 --config-path rustfmt.toml
```

### Conventions
Edition 2024, `rust-version` 1.95. The workspace directories are `elm_rs` and `elm_rs_derive`. The published packages are `rs2elm` and `rs2elm_derive`. Both forbid `unsafe_code` and deny `missing_docs` plus the rest of the lint set in those files. Every public item has a doc comment. There are no binaries.

Non-trivial runtime and derive entry points carry `#[instrument]`. Log with the `tracing` macros re-exported from the library crate. The attribute expands to `::tracing::`.

Release builds use `lto`, `codegen-units = 1`, and `panic = "abort"`. `cargo test` uses the test profile, which still unwinds.

This workspace does not use Rayon or SSH. When a validate or deploy path opens parallel SSH connections, set `RAYON_NUM_THREADS` to the physical core count. A higher value, including hyperthreads, makes those connections fail.

Check dependencies with `cargo audit`.

### Testing
The tests use optional dependencies, so `--all-features` is needed. The tests use `elm repl` so Elm has to be installed.

```console
cargo test --all-features
```
