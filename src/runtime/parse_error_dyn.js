// One of std's parse errors, `name`, as a `dyn Error` (ADR 0141): its value
// is its message (ADR 0063), shown by `{:?}` as Rust shows it,
// `ParseIntError { kind: InvalidDigit }`, and by `{}` as its text, given the
// placeholder's options as std's `fmt` of it applies them.
function $parseErrorDyn(name) {
  return {
    Debug: () => ({ fmt: (value) => $debugParseError(value, name) }),
    Display: () => ({ fmt: (value, options) => $formatted(value, options) }),
    source: () => undefined,
  };
}
