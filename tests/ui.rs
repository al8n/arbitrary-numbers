//! Compile-failure diagnostics, compared against the `.stderr` snapshot next to each case.
//!
//! The cases in `tests/ui` produce the same output on stable and nightly compilers. The cases in
//! `tests/ui/nightly` write out a type whose default storage cannot be computed: the first error is
//! this crate's diagnostic, but the follow-up errors differ between compiler releases, so those
//! snapshots are compared only on nightly compilers.
//!
//! Regenerate the snapshots with `TRYBUILD=overwrite cargo test --test ui` after reviewing a change
//! in the compiler output.

use std::process::Command;

#[test]
#[cfg_attr(miri, ignore)]
fn ui() {
  let cases = trybuild::TestCases::new();
  cases.compile_fail("tests/ui/*.rs");
  if nightly_compiler() {
    cases.compile_fail("tests/ui/nightly/*.rs");
  }
}

/// Returns `true` if `rustc --version` reports a nightly or development build.
fn nightly_compiler() -> bool {
  let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
  match Command::new(rustc).arg("--version").output() {
    Ok(output) => {
      let version = String::from_utf8_lossy(&output.stdout);
      version.contains("-nightly") || version.contains("-dev")
    }
    Err(_) => false,
  }
}
