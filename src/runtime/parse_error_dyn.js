// One of std's parse errors, of `TypeId` `type`, as a `dyn Error` (ADR
// 0141): its value is its message (ADR 0063), shown by `{:?}` as Rust shows
// it, `ParseIntError { kind: InvalidDigit }`, its type's name, and by `{}`
// as its text, given the placeholder's options as std's `fmt` of it applies
// them, `pad`, whose writes it writes into a writer (#1652); its `type_id`
// `type`, which a downcast compares (ADR 0331).
function $parseErrorDyn(type) {
  const name = type.slice(type.lastIndexOf("::") + 2);
  return {
    Debug: () => ({ fmt: (value) => $debugParseError(value, name) }),
    Display: () => ({ fmt: (value, options) => $formatted(value, options), fmtInto: $writePadded }),
    source: () => undefined,
    typeId: () => type,
  };
}
