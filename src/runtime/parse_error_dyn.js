// One of std's parse errors, `name`, as a `dyn Error` (ADR 0141): its value
// is its message (ADR 0063), shown by `{:?}` as Rust shows it,
// `ParseIntError { kind: InvalidDigit }`.
function $parseErrorDyn(name) {
  return {
    Debug: () => ({ fmt: (value) => $debugParseError(value, name) }),
    Display: () => ({ fmt: (value) => value }),
    source: () => undefined,
  };
}
