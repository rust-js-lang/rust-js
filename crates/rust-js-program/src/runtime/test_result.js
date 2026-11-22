
// A `#[test]` returning a `Result` (ADR 0026): an `Err` fails it, as
// libtest's does, shown as `Error: ..`, by `$debug`, which knows no types:
// a string's or a number's as Rust shows it, a struct's without its name.
function $testResult(result) {
  if (result.TAG === "Err") {
    throw new Error("Error: " + $debug(result._0));
  }
}
