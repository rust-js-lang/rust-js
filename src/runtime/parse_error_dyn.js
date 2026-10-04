// One of std's parse errors, `name`, as a `dyn Error` (ADR 0141): its value
// is its message (ADR 0063), or a `TryFromIntError`'s its kind, shown by
// `{:?}` as Rust shows it, `ParseIntError { kind: InvalidDigit }`.
function $parseErrorDyn(name) {
  return {
    Debug: () => ({ fmt: (value) => $debugParseError(value, name) }),
    Display: () => ({
      fmt: (value) => (name === "TryFromIntError" ? "out of range integral type conversion attempted" : value),
    }),
    source: () => undefined,
  };
}
