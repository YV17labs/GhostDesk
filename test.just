# Test recipes, exposed as `nestrs run test <kind>` (see `mod test` in the
# Justfile). Bare `nestrs run test` lists these — pick a kind to run.

# Bare `nestrs run test` lists the kinds instead of running one.
_default:
    @just --list test

# Unit + integration + doctests, no desktop.
unit:
    cargo nextest run --workspace -E 'not binary(e2e)'
    cargo test --workspace --doc          # nextest skips doctests; run them too

# e2e tests — the `tests/e2e/main.rs` suites, which for GhostDesk means a live
# Wayland desktop (the supervisord stack, `just stack`). No suite exists yet,
# so `--no-tests=pass` keeps this green until you write the first.
e2e:
    cargo nextest run --workspace -E 'binary(e2e)' --no-tests=pass

# Coverage over the full suite, e2e included. The only recipe with a prerequisite
# the CLI does not bootstrap: `cargo install cargo-llvm-cov` plus the LLVM tools
# it shells out to — `rustup component add llvm-tools-preview`. On a toolchain
# without rustup (Homebrew, distro packages) set LLVM_COV / LLVM_PROFDATA instead.
cov:
    cargo llvm-cov nextest --workspace

# Doctests only — the code examples inside `///` doc comments.
doc:
    cargo test --workspace --doc
