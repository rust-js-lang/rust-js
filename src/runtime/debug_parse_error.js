// A parse error, which is its message (ADR 0063): its kind, which
// `ParseIntError::kind()` gives and its `Debug` shows.
function $parseErrorKind(message) {
  const kinds = {
    "cannot parse integer from empty string": "Empty",
    "invalid digit found in string": "InvalidDigit",
    "number too large to fit in target type": "PosOverflow",
    "number too small to fit in target type": "NegOverflow",
    "number would be zero for non-zero type": "Zero",
    "cannot parse float from empty string": "Empty",
    "invalid float literal": "Invalid",
    "cannot parse char from empty string": "EmptyString",
    "too many characters in string": "TooManyChars",
  };
  return kinds[message];
}

// Its derived `Debug`; with `{:#?}`, `options.alternate`, on lines of
// their own, indented.
function $debugParseError(message, name, options) {
  const kind = $parseErrorKind(message);
  if (name === "ParseBoolError") return name;
  if (name === "TryFromIntError") return options?.alternate ? `TryFromIntError(\n    ${kind},\n)` : `TryFromIntError(${kind})`;
  return options?.alternate ? `${name} {\n    kind: ${kind},\n}` : `${name} { kind: ${kind} }`;
}
