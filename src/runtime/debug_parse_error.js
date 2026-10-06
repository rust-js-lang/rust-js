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

function $debugParseError(message, name) {
  if (name === "TryFromIntError") return `TryFromIntError(${$parseErrorKind(message)})`;
  return name === "ParseBoolError" ? name : `${name} { kind: ${$parseErrorKind(message)} }`;
}
