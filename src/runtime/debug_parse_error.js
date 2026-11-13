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

// Its derived `Debug` into a writer's sink `f`, a piece at a time, as
// `$debugParseError` shows it (#1652): its name; a `()` field's, `()`; or
// its kind, one `write_str` of its variant's name, a field's or a tuple's.
function $writeDebugParseError(f, message, name, pretty) {
  if (name === "ParseBoolError") return f.write(name);
  if (name === "CharTryFromError" || name === "TryFromCharError") {
    return $writeDebugTuple(f, name, [[undefined, (g) => g.write("()")]], pretty);
  }
  const kind = [$parseErrorKind(message), (g, kind) => g.write(kind)];
  if (name === "TryFromIntError") return $writeDebugTuple(f, name, [kind], pretty);
  $writeDebugStruct(f, name, [["kind", ...kind]], pretty);
}

// Its derived `Debug`; with `{:#?}`, `options.alternate`, on lines of
// their own, indented.
function $debugParseError(message, name, options) {
  const kind = $parseErrorKind(message);
  if (name === "ParseBoolError") return name;
  // A `char`'s, which hold `()`.
  if (name === "CharTryFromError" || name === "TryFromCharError") return options?.alternate ? `${name}(\n    (),\n)` : `${name}(())`;
  if (name === "TryFromIntError") return options?.alternate ? `TryFromIntError(\n    ${kind},\n)` : `TryFromIntError(${kind})`;
  return options?.alternate ? `${name} {\n    kind: ${kind},\n}` : `${name} { kind: ${kind} }`;
}
